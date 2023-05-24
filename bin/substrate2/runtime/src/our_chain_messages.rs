// Copyright 2019-2021 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

//! Everything required to serve Peer <-> Substrate messages.

use crate::Runtime;

use bp_messages::{
	source_chain::{SenderOrigin, TargetHeaderChain},
	target_chain::{ProvedMessages, SourceHeaderChain},
	InboundLaneData, LaneId, Message, MessageNonce, Parameter as MessagesParameter,
};
use bp_runtime::{Chain, ChainId, PEER_CHAIN_ID, SUBSTRATE_CHAIN_ID};
use bridge_runtime_common::messages::{self, MessageBridge, MessageTransaction};
use codec::{Decode, Encode};
use frame_support::{
	parameter_types,
	weights::{DispatchClass, Weight},
	RuntimeDebug,
};
use scale_info::TypeInfo;
use sp_runtime::{traits::Saturating, FixedPointNumber, FixedU128};
use sp_std::{convert::TryFrom, ops::RangeInclusive};

/// Initial value of `PeerToSubstrateConversionRate` parameter.
pub const INITIAL_PEER_TO_SUBSTRATE_CONVERSION_RATE: FixedU128 =
	FixedU128::from_inner(FixedU128::DIV);
/// Initial value of `PeerFeeMultiplier` parameter.
pub const INITIAL_PEER_FEE_MULTIPLIER: FixedU128 = FixedU128::from_inner(FixedU128::DIV);
pub const BASE_XCM_WEIGHT_TWICE: Weight = crate::xcm_config::BaseXcmWeight::get().saturating_mul(2);

parameter_types! {
	/// Peer to Substrate conversion rate. Initially we treat both tokens as equal.
	pub storage PeerToSubstrateConversionRate: FixedU128 = INITIAL_PEER_TO_SUBSTRATE_CONVERSION_RATE;
	/// Fee multiplier value at Peer chain.
	pub storage PeerFeeMultiplier: FixedU128 = INITIAL_PEER_FEE_MULTIPLIER;
	pub const WeightCredit: Weight = BASE_XCM_WEIGHT_TWICE;
}

/// Message payload for Substrate -> Peer messages.
pub type ToPeerMessagePayload =
	messages::source::FromThisChainMessagePayload;

	pub type ToPeerMaximalOutboundPayloadSize =
	messages::source::FromThisChainMaximalOutboundPayloadSize<WithPeerMessageBridge>;

/// Message verifier for Substrate -> Peer messages.
pub type ToPeerMessageVerifier =
	messages::source::FromThisChainMessageVerifier<WithPeerMessageBridge>;

/// Message payload for Peer -> Substrate messages.
pub type FromPeerMessagePayload =
	messages::target::FromBridgedChainMessagePayload<WithPeerMessageBridge>;

/// Encoded Substrate Call as it comes from Peer.
pub type FromPeerEncodedCall = messages::target::FromBridgedChainEncodedMessageCall<crate::RuntimeCall>;

/// Call-dispatch based message dispatch for Peer -> Substrate messages.
pub type FromPeerMessageDispatch = messages::target::FromBridgedChainMessageDispatch<
	WithPeerMessageBridge,
	crate::Runtime,
	pallet_balances::Pallet<Runtime>,
	(),
	xcm_executor::XcmExecutor<crate::xcm_config::XcmConfig>,
	crate::xcm_config::XcmWeigher,
	WeightCredit,
	
>;

/// Messages proof for Peer -> Substrate messages.
pub type FromPeerMessagesProof = messages::target::FromBridgedChainMessagesProof<peer::Hash>;

/// Messages delivery proof for Substrate -> Peer messages.
pub type ToPeerMessagesDeliveryProof =
	messages::source::FromBridgedChainMessagesDeliveryProof<peer::Hash>;

/// Peer <-> Substrate message bridge.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct WithPeerMessageBridge;

impl MessageBridge for WithPeerMessageBridge {
	const RELAYER_FEE_PERCENT: u32 = 10;
	const THIS_CHAIN_ID: ChainId = SUBSTRATE_CHAIN_ID;
	const BRIDGED_CHAIN_ID: ChainId = PEER_CHAIN_ID;
	const BRIDGED_MESSAGES_PALLET_NAME: &'static str = substrate::WITH_SUBSTRATE_MESSAGES_PALLET_NAME;

	type ThisChain = Substrate;
	type BridgedChain = Peer;

	fn bridged_balance_to_this_balance(
		bridged_balance: substrate::Balance,
		bridged_to_this_conversion_rate_override: Option<FixedU128>,
	) -> substrate::Balance {
		let conversion_rate = bridged_to_this_conversion_rate_override
			.unwrap_or_else(|| PeerToSubstrateConversionRate::get());
		substrate::Balance::try_from(conversion_rate.saturating_mul_int(bridged_balance))
			.unwrap_or(substrate::Balance::MAX)
	}
}

/// Substrate chain from message lane point of view.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct Substrate;

impl messages::ChainWithMessages for Substrate {
	type Hash = substrate::Hash;
	type AccountId = substrate::AccountId;
	type Signer = substrate::AccountSigner;
	type Signature = substrate::Signature;
	type Weight = Weight;
	type Balance = substrate::Balance;
}

impl messages::ThisChainWithMessages for Substrate {
	type RuntimeOrigin = crate::RuntimeOrigin;
	type Call = crate::RuntimeCall;

	fn is_message_accepted(send_origin: &Self::RuntimeOrigin, lane: &LaneId) -> bool {
		send_origin.linked_account().is_some() && (*lane == [0, 0, 0, 0] || *lane == [0, 0, 0, 1])
	}

	fn maximal_pending_messages_at_outbound_lane() -> MessageNonce {
		MessageNonce::MAX
	}

	fn estimate_delivery_confirmation_transaction() -> MessageTransaction<Weight> {
		let inbound_data_size = InboundLaneData::<substrate::AccountId>::encoded_size_hint(
			substrate::MAXIMAL_ENCODED_ACCOUNT_ID_SIZE,
			1,
			1,
		)
		.unwrap_or(u32::MAX);

		MessageTransaction {
			dispatch_weight: substrate::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
			size: inbound_data_size
				.saturating_add(peer::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(substrate::TX_EXTRA_BYTES),
		}
	}

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> substrate::Balance {
		// `transaction` may represent transaction from the future, when multiplier value will
		// be larger, so let's use slightly increased value
		let multiplier = FixedU128::saturating_from_rational(110, 100)
			.saturating_mul(pallet_transaction_payment::Pallet::<Runtime>::next_fee_multiplier());
		// in our testnets, both per-byte fee and weight-to-fee are 1:1
		messages::transaction_payment(
			substrate::BlockWeights::get().get(DispatchClass::Normal).base_extrinsic,
			1,
			multiplier,
			|weight| weight.ref_time() as _,
			transaction,
		)
	}
}

/// Peer chain from message lane point of view.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct Peer;

impl messages::ChainWithMessages for Peer {
	type Hash = peer::Hash;
	type AccountId = peer::AccountId;
	type Signer = peer::AccountSigner;
	type Signature = peer::Signature;
	type Weight = Weight;
	type Balance = peer::Balance;
}

impl messages::BridgedChainWithMessages for Peer {
	fn maximal_extrinsic_size() -> u32 {
		peer::Peer::max_extrinsic_size()
	}

	fn message_weight_limits(_message_payload: &[u8]) -> RangeInclusive<Weight> {
		// we don't want to relay too large messages + keep reserve for future upgrades
		let upper_limit = messages::target::maximal_incoming_message_dispatch_weight(
			peer::Peer::max_extrinsic_weight(),
		);

		// we're charging for payload bytes in `WithPeerMessageBridge::transaction_payment`
		// function
		//
		// this bridge may be used to deliver all kind of messages, so we're not making any
		// assumptions about minimal dispatch weight here

		Weight::zero()..=upper_limit
	}


	fn verify_dispatch_weight(_message_payload: &[u8]) -> bool {
		true
	}

	fn estimate_delivery_transaction(
		message_payload: &[u8],
		include_pay_dispatch_fee_cost: bool,
		message_dispatch_weight: Weight,
	) -> MessageTransaction<Weight> {
		let message_payload_len = u32::try_from(message_payload.len()).unwrap_or(u32::MAX);
		let extra_bytes_in_payload = message_payload_len
			.saturating_sub(pallet_bridge_messages::EXPECTED_DEFAULT_MESSAGE_LENGTH);

		MessageTransaction {
			dispatch_weight: peer::ADDITIONAL_MESSAGE_BYTE_DELIVERY_WEIGHT
				.saturating_mul(extra_bytes_in_payload as u64)
				.saturating_add(peer::DEFAULT_MESSAGE_DELIVERY_TX_WEIGHT)
				.saturating_sub(if include_pay_dispatch_fee_cost {
					Weight::from_ref_time(0)
				} else {
					peer::PAY_INBOUND_DISPATCH_FEE_WEIGHT
				})
				.saturating_add(message_dispatch_weight),
			size: message_payload_len
				.saturating_add(substrate::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(peer::TX_EXTRA_BYTES),
		}
	}

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> peer::Balance {
		// we don't have a direct access to the value of multiplier at Peer chain
		// => it is a messages module parameter
		let multiplier = PeerFeeMultiplier::get();
		// in our testnets, both per-byte fee and weight-to-fee are 1:1
		messages::transaction_payment(
			peer::BlockWeights::get().get(DispatchClass::Normal).base_extrinsic,
			1,
			multiplier,
			|weight| weight.ref_time() as _,
			transaction,
		)
	}
}

impl TargetHeaderChain<ToPeerMessagePayload, peer::AccountId> for Peer {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof of one or several keys;
	// - id of the lane we prove state of.
	type MessagesDeliveryProof = ToPeerMessagesDeliveryProof;

	fn verify_message(payload: &ToPeerMessagePayload) -> Result<(), Self::Error> {
		messages::source::verify_chain_message::<WithPeerMessageBridge>(payload)
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<(LaneId, InboundLaneData<substrate::AccountId>), Self::Error> {
		messages::source::verify_messages_delivery_proof::<
			WithPeerMessageBridge,
			Runtime,
			crate::PeerGrandpaInstance,
		>(proof)
	}
}

impl SourceHeaderChain<peer::Balance> for Peer {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof of one or several keys;
	// - id of the lane we prove messages for;
	// - inclusive range of messages nonces that are proved.
	type MessagesProof = FromPeerMessagesProof;

	fn verify_messages_proof(
		proof: Self::MessagesProof,
		messages_count: u32,
	) -> Result<ProvedMessages<Message<peer::Balance>>, Self::Error> {
		messages::target::verify_messages_proof::<
			WithPeerMessageBridge,
			Runtime,
			crate::PeerGrandpaInstance,
		>(proof, messages_count)
	}
}

impl SenderOrigin<crate::AccountId> for crate::RuntimeOrigin {
	fn linked_account(&self) -> Option<crate::AccountId> {
		match self.caller {
			crate::OriginCaller::system(frame_system::RawOrigin::Signed(ref submitter)) =>
				Some(submitter.clone()),
			crate::OriginCaller::system(frame_system::RawOrigin::Root) |
			crate::OriginCaller::system(frame_system::RawOrigin::None) =>
				crate::RootAccountForPayments::get(),
			_ => None,
		}
	}
}

/// Substrate -> Peer message lane pallet parameters.
#[derive(RuntimeDebug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub enum SubstrateToPeerMessagesParameter {
	/// The conversion formula we use is: `SubstrateTokens = PeerTokens * conversion_rate`.
	PeerToSubstrateConversionRate(FixedU128),
}

impl MessagesParameter for SubstrateToPeerMessagesParameter {
	fn save(&self) {
		match *self {
			SubstrateToPeerMessagesParameter::PeerToSubstrateConversionRate(ref conversion_rate) =>
				PeerToSubstrateConversionRate::set(conversion_rate),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		AccountId, Call, DbWeight, ExistentialDeposit, PeerGrandpaInstance, Runtime, SystemCall,
		SystemConfig, WithPeerMessagesInstance, VERSION,
	};
	use bp_message_dispatch::CallOrigin;
	use bp_messages::{
		target_chain::{DispatchMessage, DispatchMessageData, MessageDispatch},
		MessageKey,
	};
	use bp_runtime::{derive_account_id, messages::DispatchFeePayment, Chain, SourceAccount};
	use bridge_runtime_common::{
		assert_complete_bridge_types,
		integrity::{
			assert_complete_bridge_constants, AssertBridgeMessagesPalletConstants,
			AssertBridgePalletNames, AssertChainConstants, AssertCompleteBridgeConstants,
		},
		messages::target::{FromBridgedChainEncodedMessageCall, FromBridgedChainMessagePayload},
	};
	use frame_support::{
		traits::Currency,
		weights::{GetDispatchInfo, WeightToFeePolynomial},
	};
	use sp_runtime::traits::Convert;

	#[test]
	fn transfer_happens_when_dispatch_fee_is_paid_at_target_chain() {
		// this test actually belongs to the `bridge-runtime-common` crate, but there we have no
		// mock runtime. Making another one there just for this test, given that both crates
		// live n single repo is an overkill
		let mut ext: sp_io::TestExternalities =
			SystemConfig::default().build_storage::<Runtime>().unwrap().into();
		ext.execute_with(|| {
			let bridge = PEER_CHAIN_ID;
			let call: Call = SystemCall::set_heap_pages { pages: 64 }.into();
			let dispatch_weight = call.get_dispatch_info().weight;
			let dispatch_fee = <Runtime as pallet_transaction_payment::Config>::WeightToFee::calc(
				&dispatch_weight,
			);
			assert!(dispatch_fee > 0);

			// create relayer account with minimal balance
			let relayer_account: AccountId = [1u8; 32].into();
			let initial_amount = ExistentialDeposit::get();
			let _ = <pallet_balances::Pallet<Runtime> as Currency<AccountId>>::deposit_creating(
				&relayer_account,
				initial_amount,
			);

			// create dispatch account with minimal balance + dispatch fee
			let dispatch_account = derive_account_id::<
				<Runtime as pallet_bridge_dispatch::Config>::SourceChainAccountId,
			>(bridge, SourceAccount::Root);
			let dispatch_account =
				<Runtime as pallet_bridge_dispatch::Config>::AccountIdConverter::convert(
					dispatch_account,
				);
			let _ = <pallet_balances::Pallet<Runtime> as Currency<AccountId>>::deposit_creating(
				&dispatch_account,
				initial_amount + dispatch_fee,
			);

			// dispatch message with intention to pay dispatch fee at the target chain
			FromPeerMessageDispatch::dispatch(
				&relayer_account,
				DispatchMessage {
					key: MessageKey { lane_id: Default::default(), nonce: 0 },
					data: DispatchMessageData {
						payload: Ok(FromBridgedChainMessagePayload::<WithPeerMessageBridge> {
							spec_version: VERSION.spec_version,
							weight: dispatch_weight,
							origin: CallOrigin::SourceRoot,
							dispatch_fee_payment: DispatchFeePayment::AtTargetChain,
							call: FromBridgedChainEncodedMessageCall::new(call.encode()),
						}),
						fee: 1,
					},
				},
			);

			// ensure that fee has been transferred from dispatch to relayer account
			assert_eq!(
				<pallet_balances::Pallet<Runtime> as Currency<AccountId>>::free_balance(
					&relayer_account
				),
				initial_amount + dispatch_fee,
			);
			assert_eq!(
				<pallet_balances::Pallet<Runtime> as Currency<AccountId>>::free_balance(
					&dispatch_account
				),
				initial_amount,
			);
		});
	}

	#[test]
	fn ensure_Substrate_message_lane_weights_are_correct() {
		type Weights = pallet_bridge_messages::weights::PeerWeight<Runtime>;

		pallet_bridge_messages::ensure_weights_are_correct::<Weights>(
			substrate::DEFAULT_MESSAGE_DELIVERY_TX_WEIGHT,
			substrate::ADDITIONAL_MESSAGE_BYTE_DELIVERY_WEIGHT,
			substrate::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
			substrate::PAY_INBOUND_DISPATCH_FEE_WEIGHT,
			DbWeight::get(),
		);

		let max_incoming_message_proof_size = peer::EXTRA_STORAGE_PROOF_SIZE.saturating_add(
			messages::target::maximal_incoming_message_size(substrate::Substrate::max_extrinsic_size()),
		);
		pallet_bridge_messages::ensure_able_to_receive_message::<Weights>(
			substrate::Substrate::max_extrinsic_size(),
			substrate::Substrate::max_extrinsic_weight(),
			max_incoming_message_proof_size,
			messages::target::maximal_incoming_message_dispatch_weight(
				substrate::Substrate::max_extrinsic_weight(),
			),
		);

		let max_incoming_inbound_lane_data_proof_size =
			bp_messages::InboundLaneData::<()>::encoded_size_hint(
				substrate::MAXIMAL_ENCODED_ACCOUNT_ID_SIZE,
				substrate::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX as _,
				substrate::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX as _,
			)
			.unwrap_or(u32::MAX);
		pallet_bridge_messages::ensure_able_to_receive_confirmation::<Weights>(
			substrate::Substrate::max_extrinsic_size(),
			substrate::Substrate::max_extrinsic_weight(),
			max_incoming_inbound_lane_data_proof_size,
			substrate::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
			substrate::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
			DbWeight::get(),
		);
	}

	#[test]
	fn ensure_bridge_integrity() {
		assert_complete_bridge_types!(
			runtime: Runtime,
			with_bridged_chain_grandpa_instance: PeerGrandpaInstance,
			with_bridged_chain_messages_instance: WithPeerMessagesInstance,
			bridge: WithPeerMessageBridge,
			this_chain: substrate::Substrate,
			bridged_chain: peer::Peer,
			this_chain_account_id_converter: substrate::AccountIdConverter
		);

		assert_complete_bridge_constants::<
			Runtime,
			PeerGrandpaInstance,
			WithPeerMessagesInstance,
			WithPeerMessageBridge,
			substrate::Substrate,
		>(AssertCompleteBridgeConstants {
			this_chain_constants: AssertChainConstants {
				block_length: substrate::BlockLength::get(),
				block_weights: substrate::BlockWeights::get(),
			},
			messages_pallet_constants: AssertBridgeMessagesPalletConstants {
				max_unrewarded_relayers_in_bridged_confirmation_tx:
					peer::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
				max_unconfirmed_messages_in_bridged_confirmation_tx:
					peer::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
				bridged_chain_id: bp_runtime::PEER_CHAIN_ID,
			},
			pallet_names: AssertBridgePalletNames {
				with_this_chain_messages_pallet_name: substrate::WITH_Substrate_MESSAGES_PALLET_NAME,
				with_bridged_chain_grandpa_pallet_name: peer::WITH_PEER_GRANDPA_PALLET_NAME,
				with_bridged_chain_messages_pallet_name:
					peer::WITH_PEER_MESSAGES_PALLET_NAME,
			},
		});

		assert_eq!(
			PeerToSubstrateConversionRate::key().to_vec(),
			bp_runtime::storage_parameter_key(
				substrate::PEER_TO_Substrate_CONVERSION_RATE_PARAMETER_NAME
			)
			.0,
		);
	}

	#[test]
	#[ignore]
	fn no_stack_overflow_when_decoding_nested_call_during_dispatch() {
		// this test is normally ignored, because it only makes sense to run it in release mode

		let mut ext: sp_io::TestExternalities =
			SystemConfig::default().build_storage::<Runtime>().unwrap().into();
		ext.execute_with(|| {
			let bridge = PEER_CHAIN_ID;

			let mut call: Call = SystemCall::set_heap_pages { pages: 64 }.into();

			for _i in 0..3000 {
				call = Call::Sudo(pallet_sudo::Call::sudo { call: Box::new(call) });
			}

			let dispatch_weight = 500;
			let dispatch_fee = <Runtime as pallet_transaction_payment::Config>::WeightToFee::calc(
				&dispatch_weight,
			);
			assert!(dispatch_fee > 0);

			// create relayer account with minimal balance
			let relayer_account: AccountId = [1u8; 32].into();
			let initial_amount = ExistentialDeposit::get();
			let _ = <pallet_balances::Pallet<Runtime> as Currency<AccountId>>::deposit_creating(
				&relayer_account,
				initial_amount,
			);

			// create dispatch account with minimal balance + dispatch fee
			let dispatch_account = derive_account_id::<
				<Runtime as pallet_bridge_dispatch::Config>::SourceChainAccountId,
			>(bridge, SourceAccount::Root);
			let dispatch_account =
				<Runtime as pallet_bridge_dispatch::Config>::AccountIdConverter::convert(
					dispatch_account,
				);
			let _ = <pallet_balances::Pallet<Runtime> as Currency<AccountId>>::deposit_creating(
				&dispatch_account,
				initial_amount + dispatch_fee,
			);

			// dispatch message with intention to pay dispatch fee at the target chain
			//
			// this is where the stack overflow has happened before the fix has been applied
			FromPeerMessageDispatch::dispatch(
				&relayer_account,
				DispatchMessage {
					key: MessageKey { lane_id: Default::default(), nonce: 0 },
					data: DispatchMessageData {
						payload: Ok(FromBridgedChainMessagePayload::<WithPeerMessageBridge> {
							spec_version: VERSION.spec_version,
							weight: dispatch_weight,
							origin: CallOrigin::SourceRoot,
							dispatch_fee_payment: DispatchFeePayment::AtTargetChain,
							call: FromBridgedChainEncodedMessageCall::new(call.encode()),
						}),
						fee: 1,
					},
				},
			);
		});
	}
}

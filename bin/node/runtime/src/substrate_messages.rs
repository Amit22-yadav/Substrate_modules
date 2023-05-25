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

//! Everything required to serve peer <-> Substrate messages.

use crate::Runtime;
use crate::OriginCaller::XcmPallet;
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

pub const XCM_LANE: LaneId = [0, 0, 0, 0];

/// Initial value of `RialtoToMillauConversionRate` parameter.
pub const INITIAL_SUBSTRATE_TO_PEER_CONVERSION_RATE: FixedU128 =
	FixedU128::from_inner(FixedU128::DIV);
/// Initial value of `RialtoFeeMultiplier` parameter.
pub const INITIAL_SUBSTRATE_FEE_MULTIPLIER: FixedU128 = FixedU128::from_inner(FixedU128::DIV);
pub const BASE_XCM_WEIGHT_TWICE: Weight = crate::xcm_config::BaseXcmWeight::get().saturating_mul(2);


parameter_types! {
	/// Substrate to Peer conversion rate. Initially we treat both tokens as equal.
	pub storage SubstrateToPeerConversionRate: FixedU128 = INITIAL_SUBSTRATE_TO_PEER_CONVERSION_RATE;
	/// Fee multiplier value at Substrate chain.
	pub storage SubstrateFeeMultiplier: FixedU128 = INITIAL_SUBSTRATE_FEE_MULTIPLIER;

	pub const WeightCredit: Weight = BASE_XCM_WEIGHT_TWICE;
}

/// Message payload for Millau -> Substrate messages.
pub type ToSubstrateMessagePayload =
	messages::source::FromThisChainMessagePayload;

	pub type ToSubstrateMaximalOutboundPayloadSize =
	messages::source::FromThisChainMaximalOutboundPayloadSize<WithSubstrateMessageBridge>;

/// Message verifier for Peer -> Substrate messages.
pub type ToSubstrateMessageVerifier =
	messages::source::FromThisChainMessageVerifier<WithSubstrateMessageBridge>;

/// Message payload for Substrate -> Peer messages.
pub type FromSubstrateMessagePayload =
	messages::target::FromBridgedChainMessagePayload<crate::RuntimeCall>;

/// Encoded Millau Call as it comes from Substrate.
pub type FromSubstrateEncodedCall = messages::target::FromBridgedChainEncodedMessageCall<crate::RuntimeCall>;

/// Messages proof for Substrate -> Peer messages.
pub type FromSubstrateMessagesProof = messages::target::FromBridgedChainMessagesProof<substrate::Hash>;

/// Messages delivery proof for Peer -> Substrate messages.
pub type ToSubstrateMessagesDeliveryProof =
	messages::source::FromBridgedChainMessagesDeliveryProof<substrate::Hash>;

/// Call-dispatch based message dispatch for Substrate -> Peer messages.
pub type FromSubstrateMessageDispatch = messages::target::FromBridgedChainMessageDispatch<
	WithSubstrateMessageBridge,
	xcm_executor::XcmExecutor<crate::xcm_config::XcmConfig>,
	crate::xcm_config::XcmWeigher,
	WeightCredit,
	crate::Runtime,
	pallet_balances::Pallet<Runtime>,
	//  (),

>;



/// Peer <-> Substrate message bridge.
#[derive(RuntimeDebug, Clone,Decode, Copy)]
pub struct WithSubstrateMessageBridge;

impl MessageBridge for WithSubstrateMessageBridge {
	const RELAYER_FEE_PERCENT: u32 = 10;
	const THIS_CHAIN_ID: ChainId = PEER_CHAIN_ID;
	const BRIDGED_CHAIN_ID: ChainId = SUBSTRATE_CHAIN_ID;
	const BRIDGED_MESSAGES_PALLET_NAME: &'static str = peer::WITH_PEER_MESSAGES_PALLET_NAME;

	type ThisChain = Peer;
	type BridgedChain = Substrate;

	fn bridged_balance_to_this_balance(
		bridged_balance: substrate::Balance,
		bridged_to_this_conversion_rate_override: Option<FixedU128>,
	) -> peer::Balance {
		let conversion_rate = bridged_to_this_conversion_rate_override
			.unwrap_or_else(|| SubstrateToPeerConversionRate::get());
		peer::Balance::try_from(conversion_rate.saturating_mul_int(bridged_balance))
			.unwrap_or(peer::Balance::MAX)
	}
}

/// Millau chain from message lane point of view.
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

impl messages::ThisChainWithMessages for Peer {
	type RuntimeOrigin = crate::RuntimeOrigin;
	type Call = crate::RuntimeCall;

	fn is_message_accepted(send_origin: &Self::RuntimeOrigin, lane: &LaneId) -> bool {
		let here_location =
			xcm::v3::MultiLocation::from(crate::xcm_config::UniversalLocation::get());
		match send_origin.caller {
			XcmPallet(pallet_xcm::Origin::Xcm(ref location))
				if *location == here_location =>
			{
				log::trace!(target: "runtime::bridge", "Verifying message sent using XCM pallet to Rialto");
			},
			_ => {
				// keep in mind that in this case all messages are free (in term of fees)
				// => it's just to keep testing bridge on our test deployments until we'll have a
				// better option
				log::trace!(target: "runtime::bridge", "Verifying message sent using messages pallet to Rialto");
			},
		}

		*lane == XCM_LANE || *lane == [0, 0, 0, 1]
	}
	fn maximal_pending_messages_at_outbound_lane() -> MessageNonce {
		MessageNonce::MAX
	}

	fn estimate_delivery_confirmation_transaction() -> MessageTransaction<Weight> {
		let inbound_data_size = InboundLaneData::<peer::AccountId>::encoded_size_hint(
			peer::MAXIMAL_ENCODED_ACCOUNT_ID_SIZE,
			1,
			1,
		)
		.unwrap_or(u32::MAX);

		MessageTransaction {
			dispatch_weight: peer::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
			size: inbound_data_size
				.saturating_add(substrate::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(peer::TX_EXTRA_BYTES),
		}
	}

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> peer::Balance {
		// `transaction` may represent transaction from the future, when multiplier value will
		// be larger, so let's use slightly increased value
		let multiplier = FixedU128::saturating_from_rational(110, 100)
			.saturating_mul(pallet_transaction_payment::Pallet::<Runtime>::next_fee_multiplier());
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

impl messages::BridgedChainWithMessages for Substrate {
	fn maximal_extrinsic_size() -> u32 {
		substrate::Substrate::max_extrinsic_size()
	}

	fn verify_dispatch_weight(_message_payload: &[u8]) -> bool {
		true
	}

	fn message_weight_limits(_message_payload: &[u8]) -> RangeInclusive<Weight> {
		// we don't want to relay too large messages + keep reserve for future upgrades
		let upper_limit = messages::target::maximal_incoming_message_dispatch_weight(
			substrate::Substrate::max_extrinsic_weight(),
		);

		// we're charging for payload bytes in `WithSubstrateMessageBridge::transaction_payment`
		// function
		//
		// this bridge may be used to deliver all kind of messages, so we're not making any
		// assumptions about minimal dispatch weight here

		Weight::zero()..=upper_limit
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
			dispatch_weight: substrate::ADDITIONAL_MESSAGE_BYTE_DELIVERY_WEIGHT
			.saturating_mul(extra_bytes_in_payload as u64)
				.saturating_add(substrate::DEFAULT_MESSAGE_DELIVERY_TX_WEIGHT)
				.saturating_sub(if include_pay_dispatch_fee_cost {
					Weight::from_ref_time(0)
				} else {
					substrate::PAY_INBOUND_DISPATCH_FEE_WEIGHT
				})
				.saturating_add(message_dispatch_weight),
			size: message_payload_len
				.saturating_add(peer::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(substrate::TX_EXTRA_BYTES),
		}
	}

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> substrate::Balance {
		// we don't have a direct access to the value of multiplier at Substrate chain
		// => it is a messages module parameter
		let multiplier = SubstrateFeeMultiplier::get();
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

impl TargetHeaderChain<ToSubstrateMessagePayload, substrate::AccountId> for Substrate {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof or one or several keys;
	// - id of the lane we prove state of.
	type MessagesDeliveryProof = ToSubstrateMessagesDeliveryProof;

	fn verify_message(payload: &ToSubstrateMessagePayload) -> Result<(), Self::Error> {
		messages::source::verify_chain_message::<WithSubstrateMessageBridge>(payload)
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<(LaneId, InboundLaneData<peer::AccountId>), Self::Error> {
		messages::source::verify_messages_delivery_proof::<
			WithSubstrateMessageBridge,
			Runtime,
			crate::SubstrateGrandpaInstance,
		>(proof)
	}
}

impl SourceHeaderChain<substrate::Balance> for Substrate {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof or one or several keys;
	// - id of the lane we prove messages for;
	// - inclusive range of messages nonces that are proved.
	type MessagesProof = FromSubstrateMessagesProof;

	fn verify_messages_proof(
		proof: Self::MessagesProof,
		messages_count: u32,
	) -> Result<ProvedMessages<Message<substrate::Balance>>, Self::Error> {
		messages::target::verify_messages_proof::<
			WithSubstrateMessageBridge,
			Runtime,
			crate::SubstrateGrandpaInstance,
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
			// crate::OriginCaller::BridgeSubstrateTokenSwap(
			// 	pallet_bridge_token_swap::RawOrigin::TokenSwap {
			// 		ref swap_account_at_this_chain,
			// 		..
			// 	},) 
			// => Some(swap_account_at_this_chain.clone()),
			_ => None,
		}
	}
}

/// Peer -> Substrate message lane pallet parameters.
#[derive(RuntimeDebug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]

pub enum PeerToSubstrateMessagesParameter {
	/// The conversion formula we use is: `MillauTokens = RialtoTokens * conversion_rate`.
	SubstrateToPeerConversionRate(FixedU128),
}

impl MessagesParameter for PeerToSubstrateMessagesParameter {
	fn save(&self) {
		match *self {
			PeerToSubstrateMessagesParameter::SubstrateToPeerConversionRate(ref conversion_rate) =>
				SubstrateToPeerConversionRate::set(conversion_rate),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{DbWeight, RialtoGrandpaInstance, Runtime, WithRialtoMessagesInstance};

	use bp_runtime::Chain;
	use bridge_runtime_common::{
		assert_complete_bridge_types,
		integrity::{
			assert_complete_bridge_constants, AssertBridgeMessagesPalletConstants,
			AssertBridgePalletNames, AssertChainConstants, AssertCompleteBridgeConstants,
		},
		messages,
	};

	#[test]
	fn ensure_millau_message_lane_weights_are_correct() {
		type Weights = pallet_bridge_messages::weights::MillauWeight<Runtime>;

		pallet_bridge_messages::ensure_weights_are_correct::<Weights>(
			peer::DEFAULT_MESSAGE_DELIVERY_TX_WEIGHT,
			peer::ADDITIONAL_MESSAGE_BYTE_DELIVERY_WEIGHT,
			peer::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
			peer::PAY_INBOUND_DISPATCH_FEE_WEIGHT,
			DbWeight::get(),
		);

		let max_incoming_message_proof_size = substrate::EXTRA_STORAGE_PROOF_SIZE.saturating_add(
			messages::target::maximal_incoming_message_size(peer::Millau::max_extrinsic_size()),
		);
		pallet_bridge_messages::ensure_able_to_receive_message::<Weights>(
			peer::Millau::max_extrinsic_size(),
			peer::Millau::max_extrinsic_weight(),
			max_incoming_message_proof_size,
			messages::target::maximal_incoming_message_dispatch_weight(
				peer::Millau::max_extrinsic_weight(),
			),
		);

		let max_incoming_inbound_lane_data_proof_size =
			bp_messages::InboundLaneData::<()>::encoded_size_hint(
				peer::MAXIMAL_ENCODED_ACCOUNT_ID_SIZE,
				peer::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX as _,
				peer::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX as _,
			)
			.unwrap_or(u32::MAX);
		pallet_bridge_messages::ensure_able_to_receive_confirmation::<Weights>(
			peer::Millau::max_extrinsic_size(),
			peer::Millau::max_extrinsic_weight(),
			max_incoming_inbound_lane_data_proof_size,
			peer::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
			peer::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
			DbWeight::get(),
		);
	}

	#[test]
	fn ensure_bridge_integrity() {
		assert_complete_bridge_types!(
			runtime: Runtime,
			with_bridged_chain_grandpa_instance: RialtoGrandpaInstance,
			with_bridged_chain_messages_instance: WithRialtoMessagesInstance,
			bridge: WithRialtoMessageBridge,
			this_chain: peer::Peer,
			bridged_chain: substrate::Substrate,
			this_chain_account_id_converter: peer::AccountIdConverter
		);

		assert_complete_bridge_constants::<
			Runtime,
			RialtoGrandpaInstance,
			WithRialtoMessagesInstance,
			WithRialtoMessageBridge,
			peer::Millau,
		>(AssertCompleteBridgeConstants {
			this_chain_constants: AssertChainConstants {
				block_length: peer::BlockLength::get(),
				block_weights: peer::BlockWeights::get(),
			},
			messages_pallet_constants: AssertBridgeMessagesPalletConstants {
				max_unrewarded_relayers_in_bridged_confirmation_tx:
					substrate::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
				max_unconfirmed_messages_in_bridged_confirmation_tx:
					substrate::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
				bridged_chain_id: bp_runtime::RIALTO_CHAIN_ID,
			},
			pallet_names: AssertBridgePalletNames {
				with_this_chain_messages_pallet_name: peer::WITH_MILLAU_MESSAGES_PALLET_NAME,
				with_bridged_chain_grandpa_pallet_name: substrate::WITH_RIALTO_GRANDPA_PALLET_NAME,
				with_bridged_chain_messages_pallet_name:
					substrate::WITH_RIALTO_MESSAGES_PALLET_NAME,
			},
		});

		assert_eq!(
			RialtoToMillauConversionRate::key().to_vec(),
			bp_runtime::storage_parameter_key(
				peer::RIALTO_TO_MILLAU_CONVERSION_RATE_PARAMETER_NAME
			)
			.0,
		);
	}
}

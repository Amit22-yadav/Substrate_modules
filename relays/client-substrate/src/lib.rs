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

//! Types used to connect to the Rialto-Substrate chain.

use bp_messages::MessageNonce;
use codec::{Compact, Decode, Encode};
use frame_support::weights::Weight;
use relay_substrate_client::{
	BalanceOf, Chain, ChainBase, ChainWithBalances, ChainWithGrandpa, ChainWithMessages,
	Error as SubstrateError, IndexOf, SignParam, TransactionSignScheme, UnsignedTransaction,
};
use sp_core::{storage::StorageKey, Pair};
use sp_runtime::{generic::SignedPayload, traits::IdentifyAccount};
use std::time::Duration;

/// Rialto header id.
pub type HeaderId = relay_utils::HeaderId<runtime::Hash, runtime::BlockNumber>;

/// Rialto chain definition
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Substrate2;

impl ChainBase for Substrate2 {
	type BlockNumber = runtime::BlockNumber;
	type Hash = runtime::Hash;
	type Hasher = runtime::Hashing;
	type Header = runtime::Header;

	type AccountId = runtime::AccountId;
	type Balance = runtime::Balance;
	type Index = runtime::Index;
	type Signature = runtime::Signature;

	fn max_extrinsic_size() -> u32 {
		chain_substrate::Substrate2::max_extrinsic_size()
	}

	fn max_extrinsic_weight() -> Weight {
		chain_substrate::Substrate2::max_extrinsic_weight()
	}
}

impl Chain for Substrate2 {
	const NAME: &'static str = "Substrate2";
	// Rialto token has no value, but we associate it with DOT token
	const TOKEN_ID: Option<&'static str> = Some("polkadot");
	const BEST_FINALIZED_HEADER_ID_METHOD: &'static str =
		chain_substrate::BEST_FINALIZED_RIALTO_HEADER_METHOD;
	const AVERAGE_BLOCK_INTERVAL: Duration = Duration::from_secs(5);
	const STORAGE_PROOF_OVERHEAD: u32 = chain_substrate::EXTRA_STORAGE_PROOF_SIZE;
	const MAXIMAL_ENCODED_ACCOUNT_ID_SIZE: u32 = chain_substrate::MAXIMAL_ENCODED_ACCOUNT_ID_SIZE;

	type SignedBlock = runtime::SignedBlock;
	type Call = runtime::RuntimeCall;
	//type WeightToFee = chain_substrate::WeightToFee;
}

impl ChainWithGrandpa for Substrate2 {
	const WITH_CHAIN_GRANDPA_PALLET_NAME: &'static str = chain_substrate::WITH_SUBSTRATE2_GRANDPA_PALLET_NAME;
}

impl ChainWithMessages for Substrate2 {
	const WITH_CHAIN_MESSAGES_PALLET_NAME: &'static str =
		chain_substrate::WITH_RIALTO_MESSAGES_PALLET_NAME;
	const TO_CHAIN_MESSAGE_DETAILS_METHOD: &'static str =
		chain_substrate::TO_RIALTO_MESSAGE_DETAILS_METHOD;
	const PAY_INBOUND_DISPATCH_FEE_WEIGHT_AT_CHAIN: Weight =
		chain_substrate::PAY_INBOUND_DISPATCH_FEE_WEIGHT;
	const MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX: MessageNonce =
		chain_substrate::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	const MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX: MessageNonce =
		chain_substrate::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
	type WeightInfo = ();
}

impl ChainWithBalances for Substrate2 {
	fn account_info_storage_key(account_id: &Self::AccountId) -> StorageKey {
		use frame_support::storage::generator::StorageMap;
		StorageKey(frame_system::Account::<runtime::Runtime>::storage_map_final_key(
			account_id,
		))
	}
}

impl TransactionSignScheme for Substrate2 {
	type Chain = Substrate2;
	type AccountKeyPair = sp_core::sr25519::Pair;
	type SignedTransaction = runtime::UncheckedExtrinsic;

	fn sign_transaction(param: SignParam<Self>) -> Result<Self::SignedTransaction, SubstrateError> {
		let raw_payload = SignedPayload::from_raw(
			param.unsigned.call.clone(),
			(
				frame_system::CheckNonZeroSender::<runtime::Runtime>::new(),
				frame_system::CheckSpecVersion::<runtime::Runtime>::new(),
				frame_system::CheckTxVersion::<runtime::Runtime>::new(),
				frame_system::CheckGenesis::<runtime::Runtime>::new(),
				frame_system::CheckEra::<runtime::Runtime>::from(param.era.frame_era()),
				frame_system::CheckNonce::<runtime::Runtime>::from(param.unsigned.nonce),
				frame_system::CheckWeight::<runtime::Runtime>::new(),
				pallet_asset_tx_payment::ChargeAssetTxPayment::<runtime::Runtime>::from(param.unsigned.tip,None),
			),
			(
				(),
				param.spec_version,
				param.transaction_version,
				param.genesis_hash,
				param.era.signed_payload(param.genesis_hash),
				(),
				(),
				(),
				
			),
		);
		let signature = raw_payload.using_encoded(|payload| param.signer.sign(payload));
		let signer: sp_runtime::MultiSigner = param.signer.public().into();
		let (call, extra, _) = raw_payload.deconstruct();

		Ok(runtime::UncheckedExtrinsic::new_signed(
			call.into_decoded()?,
			signer.into_account().into(),
			signature.into(),
			extra,
		))
	}

	fn is_signed(tx: &Self::SignedTransaction) -> bool {
		tx.signature.is_some()
	}

	fn is_signed_by(signer: &Self::AccountKeyPair, tx: &Self::SignedTransaction) -> bool {
		tx.signature
			.as_ref()
			.map(|(address, _, _)| *address == runtime::Address::Id(signer.public().into()))
			.unwrap_or(false)
	}

	fn parse_transaction(tx: Self::SignedTransaction) -> Option<UnsignedTransaction<Self::Chain>> {
		let extra = &tx.signature.as_ref()?.2;
		Some(UnsignedTransaction {
			call: tx.function.into(),
			nonce: Compact::<IndexOf<Self::Chain>>::decode(&mut &extra.5.encode()[..]).ok()?.into(),
			tip: Compact::<BalanceOf<Self::Chain>>::decode(&mut &extra.7.encode()[..])
				.ok()?
				.into(),
		})
	}
}

/// Rialto signing params.
pub type SigningParams = sp_core::sr25519::Pair;

/// Rialto header type used in headers sync.
pub type SyncHeader = relay_substrate_client::SyncHeader<runtime::Header>;

#[cfg(test)]
mod tests {
	use super::*;
	use relay_substrate_client::TransactionEra;

	#[test]
	fn parse_transaction_works() {
		let unsigned = UnsignedTransaction {
			call: runtime::Call::System(runtime::SystemCall::remark {
				remark: b"Hello world!".to_vec(),
			})
			.into(),
			nonce: 777,
			tip: 888,
		};
		let signed_transaction = Rialto::sign_transaction(SignParam {
			spec_version: 42,
			transaction_version: 50000,
			genesis_hash: [42u8; 32].into(),
			signer: sp_core::sr25519::Pair::from_seed_slice(&[1u8; 32]).unwrap(),
			era: TransactionEra::immortal(),
			unsigned: unsigned.clone(),
		})
		.unwrap();
		let parsed_transaction = Rialto::parse_transaction(signed_transaction).unwrap();
		assert_eq!(parsed_transaction, unsigned);
	}
}

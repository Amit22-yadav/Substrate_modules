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

//! Rialto chain specification for CLI.

use crate::cli::{
	bridge,
	encode_call::{self, Call, CliEncodeCall},
	encode_message,
	send_message::{self, DispatchFeePayment},
	CliChain,
};
use anyhow::anyhow;
use bp_message_dispatch::{CallOrigin, MessagePayload};
use bp_runtime::EncodedOrDecodedCall;
use codec::Decode;
use frame_support::weights::{DispatchInfo, GetDispatchInfo};
use client_substrate::Substrate;
use sp_version::RuntimeVersion;

impl CliEncodeCall for Substrate {
	fn encode_call(call: &Call) -> anyhow::Result<EncodedOrDecodedCall<Self::Call>> {
		Ok(match call {
			Call::Raw { data } => Self::Call::decode(&mut &*data.0)?.into(),
			Call::Remark { remark_payload, .. } =>
				runtime::RuntimeCall::System(runtime::SystemCall::remark {
					remark: remark_payload.as_ref().map(|x| x.0.clone()).unwrap_or_default(),
				})
				.into(),
			Call::Transfer { recipient, amount } =>
				runtime::RuntimeCall::Balances(runtime::BalancesCall::transfer {
					dest: recipient.raw_id().into(),
					value: amount.0,
				})
				.into(),
			Call::BridgeSendMessage { lane, payload, fee, bridge_instance_index } =>
				match *bridge_instance_index {
					bridge::SUBSTRATE_TO_PEER_INDEX => {
						let payload = Decode::decode(&mut &*payload.0)?;
						runtime::RuntimeCall::BridgePeerMessages(
							runtime::MessagesCall::send_message {
								lane_id: lane.0,
								payload,
								delivery_and_dispatch_fee: fee.0,
							},
						)
						.into()
					},
					_ => anyhow::bail!(
						"Unsupported target bridge pallet with instance index: {}",
						bridge_instance_index
					),
				},
		})
	}

	fn get_dispatch_info(call: &EncodedOrDecodedCall<Self::Call>) -> anyhow::Result<DispatchInfo> {
		Ok(call.to_decoded()?.get_dispatch_info())
	}
}

impl CliChain for Substrate {
	const RUNTIME_VERSION: RuntimeVersion = runtime::VERSION;

	type KeyPair = sp_core::sr25519::Pair;
	type MessagePayload = MessagePayload<
		substrate::AccountId,
		peer::AccountSigner,
		peer::Signature,
		Vec<u8>,
	>;

	fn ss58_format() -> u16 {
		runtime::SS58Prefix::get() as u16
	}

	fn encode_message(
		message: encode_message::MessagePayload,
	) -> anyhow::Result<Self::MessagePayload> {
		match message {
			encode_message::MessagePayload::Raw { data } => MessagePayload::decode(&mut &*data.0)
				.map_err(|e| anyhow!("Failed to decode Substrate's MessagePayload: {:?}", e)),
			encode_message::MessagePayload::Call { mut call, mut sender, dispatch_weight } => {
				type Source = Substrate;
				type Target = client_peer::Peer;

				sender.enforce_chain::<Source>();
				let spec_version = Target::RUNTIME_VERSION.spec_version;
				let origin = CallOrigin::SourceAccount(sender.raw_id());
				encode_call::preprocess_call::<Source, Target>(
					&mut call,
					bridge::SUBSTRATE_TO_PEER_INDEX,
				);	
				let call = Target::encode_call(&call)?;
				let dispatch_weight = dispatch_weight.map(Ok).unwrap_or_else(|| {
					call.to_decoded().map(|call| call.get_dispatch_info().weight)
				})?;

				Ok(send_message::message_payload(
					spec_version,
					dispatch_weight,
					origin,
					&call,
					DispatchFeePayment::AtSourceChain,
				))
			},
		}
	}
}

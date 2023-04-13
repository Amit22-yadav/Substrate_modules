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

//! Rialto-to-Millau messages sync entrypoint.

use messages_relay::relay_strategy::MixStrategy;
use client_peer::Peer;
use client_substrate::Substrate;
use substrate_relay_helper::messages_lane::{
	DirectReceiveMessagesDeliveryProofCallBuilder, DirectReceiveMessagesProofCallBuilder,
	SubstrateMessageLane,
};

/// Description of Rialto -> Millau messages bridge.
#[derive(Clone, Debug)]
pub struct SubstrateMessagesToPeer;
substrate_relay_helper::generate_direct_update_conversion_rate_call_builder!(
	Substrate,
	SubstrateMessagesToPeerUpdateConversionRateCallBuilder,
	runtime::Runtime,
	runtime::WithPeerMessagesInstance,
	runtime::our_chain_messages::SubstrateToPeerMessagesParameter::PeerToSubstrateConversionRate
);

impl SubstrateMessageLane for SubstrateMessagesToPeer {
	const SOURCE_TO_TARGET_CONVERSION_RATE_PARAMETER_NAME: Option<&'static str> =
		Some(peer::SUBSTRATE_TO_PEER_CONVERSION_RATE_PARAMETER_NAME);
	const TARGET_TO_SOURCE_CONVERSION_RATE_PARAMETER_NAME: Option<&'static str> =
		Some(substrate::PEER_TO_SUBSTRATE_CONVERSION_RATE_PARAMETER_NAME);

	const SOURCE_FEE_MULTIPLIER_PARAMETER_NAME: Option<&'static str> = None;
	const TARGET_FEE_MULTIPLIER_PARAMETER_NAME: Option<&'static str> = None;
	const AT_SOURCE_TRANSACTION_PAYMENT_PALLET_NAME: Option<&'static str> = None;
	const AT_TARGET_TRANSACTION_PAYMENT_PALLET_NAME: Option<&'static str> = None;

	type SourceChain = Substrate;
	type TargetChain = Peer;

	type SourceTransactionSignScheme = Substrate;
	type TargetTransactionSignScheme = Peer;

	type ReceiveMessagesProofCallBuilder = DirectReceiveMessagesProofCallBuilder<
		Self,
		kitchensink_runtime::Runtime,
		kitchensink_runtime::WithSubstrateMessagesInstance,
	>;
	type ReceiveMessagesDeliveryProofCallBuilder = DirectReceiveMessagesDeliveryProofCallBuilder<
		Self,
		runtime::Runtime,
		runtime::WithPeerMessagesInstance,
	>;

	type TargetToSourceChainConversionRateUpdateBuilder =
		SubstrateMessagesToPeerUpdateConversionRateCallBuilder;

	type RelayStrategy = MixStrategy;
}

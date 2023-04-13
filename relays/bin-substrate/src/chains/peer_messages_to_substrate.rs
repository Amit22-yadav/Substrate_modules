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

//! Millau-to-Rialto messages sync entrypoint.

use messages_relay::relay_strategy::MixStrategy;
use client_peer::Peer;
use client_substrate::Substrate;
use substrate_relay_helper::messages_lane::{
	DirectReceiveMessagesDeliveryProofCallBuilder, DirectReceiveMessagesProofCallBuilder,
	SubstrateMessageLane,
};

/// Description of Millau -> Rialto messages bridge.
#[derive(Clone, Debug)]
pub struct PeerMessagesToSubstrate;
substrate_relay_helper::generate_direct_update_conversion_rate_call_builder!(
	Peer,
	MillauMessagesToRialtoUpdateConversionRateCallBuilder,
	kitchensink_runtime::Runtime,
	kitchensink_runtime::WithSubstrateMessagesInstance,
	kitchensink_runtime::substrate_messages::PeerToSubstrateMessagesParameter::SubstrateToPeerConversionRate
);

impl SubstrateMessageLane for PeerMessagesToSubstrate {
	const SOURCE_TO_TARGET_CONVERSION_RATE_PARAMETER_NAME: Option<&'static str> =
		Some(substrate::PEER_TO_SUBSTRATE_CONVERSION_RATE_PARAMETER_NAME);
	const TARGET_TO_SOURCE_CONVERSION_RATE_PARAMETER_NAME: Option<&'static str> =
		Some(peer::SUBSTRATE_TO_PEER_CONVERSION_RATE_PARAMETER_NAME);

	const SOURCE_FEE_MULTIPLIER_PARAMETER_NAME: Option<&'static str> = None;
	const TARGET_FEE_MULTIPLIER_PARAMETER_NAME: Option<&'static str> = None;
	const AT_SOURCE_TRANSACTION_PAYMENT_PALLET_NAME: Option<&'static str> = None;
	const AT_TARGET_TRANSACTION_PAYMENT_PALLET_NAME: Option<&'static str> = None;

	type SourceChain = Peer;
	type TargetChain = Substrate;

	type SourceTransactionSignScheme = Peer;
	type TargetTransactionSignScheme = Substrate;

	type ReceiveMessagesProofCallBuilder = DirectReceiveMessagesProofCallBuilder<
		Self,
		runtime::Runtime,
		runtime::WithPeerMessagesInstance,
	>;
	type ReceiveMessagesDeliveryProofCallBuilder = DirectReceiveMessagesDeliveryProofCallBuilder<
		Self,
		kitchensink_runtime::Runtime,
		kitchensink_runtime::WithSubstrateMessagesInstance,
	>;

	type TargetToSourceChainConversionRateUpdateBuilder =
		MillauMessagesToRialtoUpdateConversionRateCallBuilder;

	type RelayStrategy = MixStrategy;
}

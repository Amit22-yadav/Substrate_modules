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

use strum::{EnumString, EnumVariantNames};

#[derive(Debug, PartialEq, Eq, EnumString, EnumVariantNames)]
#[strum(serialize_all = "kebab_case")]
/// Supported full bridges (headers + messages).
pub enum FullBridge {
	PeerToSubstrate,
	SubstrateToPeer,
	// RococoToWococo,
	// WococoToRococo,
	// KusamaToPolkadot,
	// PolkadotToKusama,
}

impl FullBridge {
	/// Return instance index of the bridge pallet in source runtime.
	pub fn bridge_instance_index(&self) -> u8 {
		match self {
			Self::PeerToSubstrate => PEER_TO_SUBSTRATE_INDEX,
			Self::SubstrateToPeer => SUBSTRATE_TO_PEER_INDEX,
			// Self::RococoToWococo => ROCOCO_TO_WOCOCO_INDEX,
			// Self::WococoToRococo => WOCOCO_TO_ROCOCO_INDEX,
			// Self::KusamaToPolkadot => KUSAMA_TO_POLKADOT_INDEX,
			// Self::PolkadotToKusama => POLKADOT_TO_KUSAMA_INDEX,
		}
	}
}

pub const PEER_TO_SUBSTRATE_INDEX: u8 = 0;
pub const SUBSTRATE_TO_PEER_INDEX: u8 = 0;
// pub const ROCOCO_TO_WOCOCO_INDEX: u8 = 0;
// pub const WOCOCO_TO_ROCOCO_INDEX: u8 = 0;
// pub const KUSAMA_TO_POLKADOT_INDEX: u8 = 0;
// pub const POLKADOT_TO_KUSAMA_INDEX: u8 = 0;

/// The macro allows executing bridge-specific code without going fully generic.
///
/// It matches on the [`FullBridge`] enum, sets bridge-specific types or imports and injects
/// the `$generic` code at every variant.
#[macro_export]
macro_rules! select_full_bridge {
	($bridge: expr, $generic: tt) => {
		match $bridge {
			FullBridge::PeerToSubstrate => {
				type Source = client_peer::Peer;
				#[allow(dead_code)]
				type Target = client_substrate::Substrate;

				// Derive-account
				#[allow(unused_imports)]
				use substrate::derive_account_from_peer_id as derive_account;

				// Relay-messages
				#[allow(unused_imports)]
				use crate::chains::peer_messages_to_substrate::PeerMessagesToSubstrate as MessagesLane;

				// Send-message / Estimate-fee
				#[allow(unused_imports)]
				use substrate::TO_SUBSTRATE_ESTIMATE_MESSAGE_FEE_METHOD as ESTIMATE_MESSAGE_FEE_METHOD;
				// Send-message
				#[allow(unused_imports)]
				use kitchensink_runtime::substrate_to_substrate2_account_ownership_digest as account_ownership_digest;

				$generic
			}
			FullBridge::SubstrateToPeer => {
				type Source = client_substrate::Substrate;
				#[allow(dead_code)]
				type Target = client_peer::Peer;

				// Derive-account
				#[allow(unused_imports)]
				use peer::derive_account_from_substrate_id as derive_account;

				// Relay-messages
				#[allow(unused_imports)]
				use crate::chains::substrate_messages_to_peer::SubstrateMessagesToPeer as MessagesLane;

				// Send-message / Estimate-fee
				use peer::TO_PEER_ESTIMATE_MESSAGE_FEE_METHOD as ESTIMATE_MESSAGE_FEE_METHOD;

				// Send-message
				#[allow(unused_imports)]
				use runtime::substrate_to_peer_account_ownership_digest as account_ownership_digest;

				$generic
			}
			// FullBridge::RococoToWococo => {
			// 	type Source = relay_rococo_client::Rococo;
			// 	#[allow(dead_code)]
			// 	type Target = relay_wococo_client::Wococo;

			// 	// Derive-account
			// 	#[allow(unused_imports)]
			// 	use bp_wococo::derive_account_from_rococo_id as derive_account;

			// 	// Relay-messages
			// 	#[allow(unused_imports)]
			// 	use crate::chains::rococo_messages_to_wococo::RococoMessagesToWococo as MessagesLane;

			// 	// Send-message / Estimate-fee
			// 	#[allow(unused_imports)]
			// 	use bp_wococo::TO_WOCOCO_ESTIMATE_MESSAGE_FEE_METHOD as ESTIMATE_MESSAGE_FEE_METHOD;
			// 	// Send-message
			// 	#[allow(unused_imports)]
			// 	use relay_rococo_client::runtime::rococo_to_wococo_account_ownership_digest as account_ownership_digest;

			// 	$generic
			// }
			// FullBridge::WococoToRococo => {
			// 	type Source = relay_wococo_client::Wococo;
			// 	#[allow(dead_code)]
			// 	type Target = relay_rococo_client::Rococo;

			// 	// Derive-account
			// 	#[allow(unused_imports)]
			// 	use bp_rococo::derive_account_from_wococo_id as derive_account;

			// 	// Relay-messages
			// 	#[allow(unused_imports)]
			// 	use crate::chains::wococo_messages_to_rococo::WococoMessagesToRococo as MessagesLane;

			// 	// Send-message / Estimate-fee
			// 	#[allow(unused_imports)]
			// 	use bp_rococo::TO_ROCOCO_ESTIMATE_MESSAGE_FEE_METHOD as ESTIMATE_MESSAGE_FEE_METHOD;
			// 	// Send-message
			// 	#[allow(unused_imports)]
			// 	use relay_wococo_client::runtime::wococo_to_rococo_account_ownership_digest as account_ownership_digest;

			// 	$generic
			// }
			// FullBridg::KusamaToPolkadot => {
			// 	type Source = relay_kusama_client::Kusama;
			// 	#[allow(dead_code)]
			// 	type Target = relay_polkadot_client::Polkadot;

			// 	// Derive-account
			// 	#[allow(unused_imports)]
			// 	use bp_polkadot::derive_account_from_kusama_id as derive_account;

			// 	// Relay-messages
			// 	#[allow(unused_imports)]
			// 	use crate::chains::kusama_messages_to_polkadot::KusamaMessagesToPolkadot as MessagesLane;

			// 	// Send-message / Estimate-fee
			// 	#[allow(unused_imports)]
			// 	use bp_polkadot::TO_POLKADOT_ESTIMATE_MESSAGE_FEE_METHOD as ESTIMATE_MESSAGE_FEE_METHOD;
			// 	// Send-message
			// 	#[allow(unused_imports)]
			// 	use relay_kusama_client::runtime::kusama_to_polkadot_account_ownership_digest as account_ownership_digest;

			// 	$generic
			// }
			// FullBridge::PolkadotToKusama => {
			// 	type Source = relay_polkadot_client::Polkadot;
			// 	#[allow(dead_code)]
			// 	type Target = relay_kusama_client::Kusama;

			// 	// Derive-account
			// 	#[allow(unused_imports)]
			// 	use bp_kusama::derive_account_from_polkadot_id as derive_account;

			// 	// Relay-messages
			// 	#[allow(unused_imports)]
			// 	use crate::chains::polkadot_messages_to_kusama::PolkadotMessagesToKusama as MessagesLane;

			// 	// Send-message / Estimate-fee
			// 	#[allow(unused_imports)]
			// 	use bp_kusama::TO_KUSAMA_ESTIMATE_MESSAGE_FEE_METHOD as ESTIMATE_MESSAGE_FEE_METHOD;
			// 	// Send-message
			// 	#[allow(unused_imports)]
			// 	use relay_polkadot_client::runtime::polkadot_to_kusama_account_ownership_digest as account_ownership_digest;

			// 	$generic
			// }
		}
	};
}

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

use structopt::StructOpt;
use strum::{EnumString, EnumVariantNames, VariantNames};

use relay_utils::metrics::{GlobalMetrics, StandaloneMetric};
use substrate_relay_helper::finality::SubstrateFinalitySyncPipeline;

use crate::cli::{
	PrometheusParams, SourceConnectionParams, TargetConnectionParams, TargetSigningParams,
};

/// Start headers relayer process.
#[derive(StructOpt)]
pub struct RelayHeaders {
	/// A bridge instance to relay headers for.
	#[structopt(possible_values = RelayHeadersBridge::VARIANTS, case_insensitive = true)]
	bridge: RelayHeadersBridge,
	/// If passed, only mandatory headers (headers that are changing the GRANDPA authorities set)
	/// are relayed.
	#[structopt(long)]
	only_mandatory_headers: bool,
	#[structopt(flatten)]
	source: SourceConnectionParams,
	#[structopt(flatten)]
	target: TargetConnectionParams,
	#[structopt(flatten)]
	target_sign: TargetSigningParams,
	#[structopt(flatten)]
	prometheus_params: PrometheusParams,
}

#[derive(Debug, EnumString, EnumVariantNames)]
#[strum(serialize_all = "kebab_case")]
/// Headers relay bridge.
pub enum RelayHeadersBridge {
	PeerToSubstrate,
	SubstrateToPeer,
	// WestendToMillau,
	// RococoToWococo,
	// WococoToRococo,
	// KusamaToPolkadot,
	// PolkadotToKusama,
}

macro_rules! select_bridge {
	($bridge: expr, $generic: tt) => {
		match $bridge {
			RelayHeadersBridge::PeerToSubstrate => {
				type Source = client_peer::Peer;
				type Target = client_substrate::Substrate;
				type Finality = crate::chains::peer_headers_to_substrate::PeerFinalityToSubstrate;

				$generic
			},
			RelayHeadersBridge::SubstrateToPeer => {
				type Source = client_substrate::Substrate;
				type Target = client_peer::Peer;
				type Finality = crate::chains::substrate_headers_to_peer::SubstrateFinalityToPeer;

				$generic
			},
			// RelayHeadersBridge::WestendToMillau => {
			// 	type Source = relay_westend_client::Westend;
			// 	type Target = relay_millau_client::Millau;
			// 	type Finality = crate::chains::westend_headers_to_millau::WestendFinalityToMillau;

			// 	$generic
			// },
			// RelayHeadersBridge::RococoToWococo => {
			// 	type Source = relay_rococo_client::Rococo;
			// 	type Target = relay_wococo_client::Wococo;
			// 	type Finality = crate::chains::rococo_headers_to_wococo::RococoFinalityToWococo;

			// 	$generic
			// },
			// RelayHeadersBridge::WococoToRococo => {
			// 	type Source = relay_wococo_client::Wococo;
			// 	type Target = relay_rococo_client::Rococo;
			// 	type Finality = crate::chains::wococo_headers_to_rococo::WococoFinalityToRococo;

			// 	$generic
			// },
			// RelayHeadersBridge::KusamaToPolkadot => {
			// 	type Source = relay_kusama_client::Kusama;
			// 	type Target = relay_polkadot_client::Polkadot;
			// 	type Finality = crate::chains::kusama_headers_to_polkadot::KusamaFinalityToPolkadot;

			// 	$generic
			// },
			// RelayHeadersBridge::PolkadotToKusama => {
			// 	type Source = relay_polkadot_client::Polkadot;
			// 	type Target = relay_kusama_client::Kusama;
			// 	type Finality = crate::chains::polkadot_headers_to_kusama::PolkadotFinalityToKusama;

			// 	$generic
			// },
		}
	};
}

impl RelayHeaders {
	/// Run the command.
	pub async fn run(self) -> anyhow::Result<()> {
		select_bridge!(self.bridge, {
			let source_client = self.source.to_client::<Source>().await?;
			let target_client = self.target.to_client::<Target>().await?;
			let target_transactions_mortality = self.target_sign.target_transactions_mortality;
			let target_sign = self.target_sign.to_keypair::<Target>()?;

			let metrics_params: relay_utils::metrics::MetricsParams = self.prometheus_params.into();
			GlobalMetrics::new()?.register_and_spawn(&metrics_params.registry)?;

			let target_transactions_params = substrate_relay_helper::TransactionParams {
				signer: target_sign,
				mortality: target_transactions_mortality,
			};
			Finality::start_relay_guards(
				&target_client,
				&target_transactions_params,
				self.target.can_start_version_guard(),
			)
			.await?;

			substrate_relay_helper::finality::run::<Finality>(
				source_client,
				target_client,
				self.only_mandatory_headers,
				target_transactions_params,
				metrics_params,
			)
			.await
		})
	}
}

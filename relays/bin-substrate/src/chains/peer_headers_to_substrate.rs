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

//! Peer-to-Substrate headers sync entrypoint.

use substrate_relay_helper::finality::{
	engine::Grandpa as GrandpaFinalityEngine, DirectSubmitGrandpaFinalityProofCallBuilder,
	SubstrateFinalitySyncPipeline,
};

/// Description of Peer -> Rialto finalized headers bridge.
#[derive(Clone, Debug)]
pub struct PeerFinalityToSubstrate;

impl SubstrateFinalitySyncPipeline for PeerFinalityToSubstrate {
	type SourceChain = client_peer::Peer;
	type TargetChain = client_substrate::Substrate;

	type FinalityEngine = GrandpaFinalityEngine<Self::SourceChain>;
	type SubmitFinalityProofCallBuilder = DirectSubmitGrandpaFinalityProofCallBuilder<
		Self,
		runtime::Runtime,
		runtime::PeerGrandpaInstance,
	>;
	type TransactionSignScheme = client_substrate::Substrate;
}

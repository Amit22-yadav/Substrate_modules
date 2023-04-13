#!/bin/bash
# A script for relaying Rialto messages to the Millau chain.
#
# Will not work unless both the Rialto and Millau are running (see `run-rialto-node.sh`
# and `run-millau-node.sh).
set -xeu

PEER_PORT="${PEER_PORT:-9945}"
SUBSTRATE_PORT="${SUBSTRATE_PORT:-9944}"

RUST_LOG=bridge=debug \
./target/release/bin-substrate relay-messages substrate-to-peer \
	--relayer-mode=altruistic \
	--lane 00000000 \
	--source-host localhost \
	--source-port $SUBSTRATE_PORT \
	--source-signer //Bob \
	--target-host localhost \
	--target-port $PEER_PORT \
	--target-signer //Bob \
	--prometheus-host=0.0.0.0

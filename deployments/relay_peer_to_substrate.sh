#!/bin/bash

# A script for relaying Millau headers to the Rialto chain.
#
# Will not work unless both the Rialto and Millau are running (see `run-rialto-node.sh`
# and `run-millau-node.sh).

PEER_PORT="${PEER_PORT:-9945}"
SUBSTRATE_PORT="${SUBSTRATE_PORT:-9944}"

RUST_LOG=bridge=debug \
./target/release/bin-substrate init-bridge peer-to-substrate \
	--source-host localhost \
	--source-port $PEER_PORT \
	--target-host localhost \
	--target-port $SUBSTRATE_PORT \
	--target-signer //Alice \
	--source-version-mode Bundle \
	--target-version-mode Bundle

sleep 5
RUST_LOG=bridge=debug \
./target/release/bin-substrate relay-headers peer-to-substrate \
	--source-host localhost \
	--source-port $PEER_PORT \
	--target-host localhost \
	--target-port $SUBSTRATE_PORT \
	--target-signer //Alice \
	--prometheus-host=0.0.0.0

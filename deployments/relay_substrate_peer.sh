#!/bin/bash

# A script for relaying Rialto headers to the Millau chain.
#
# Will not work unless both the Rialto and Millau are running (see `run-rialto-node.sh`
# and `run-millau-node.sh).

PEER_PORT="${PEER_PORT:-9945}"
SUBSTRATE_PORT="${SUBSTRATE_PORT:-9944}"

RUST_LOG=bridge=debug \
./target/release/bin-substrate init-bridge substrate-to-peer \
	--target-host localhost \
	--target-port $PEER_PORT \
	--source-host localhost \
	--source-port $SUBSTRATE_PORT \
	--target-signer //Alice \

sleep 5
RUST_LOG=bridge=debug \
./target/release/bin-substrate relay-headers substrate-to-peer \
	--target-host localhost \
	--target-port $PEER_PORT \
	--source-host localhost \
	--source-port $SUBSTRATE_PORT \
	--target-signer //Alice \
	--prometheus-host=0.0.0.0

#!/bin/bash

# Run a development instance of the Millau Substrate bridge node.
# To override the default port just export MILLAU_PORT=9945

PEER_PORT="${PEER_PORT:-9945}"

RUST_LOG=runtime=trace \
./target/release/peer --dev --tmp \
    --rpc-cors=all --unsafe-rpc-external --unsafe-ws-external \
    --port 33044 --rpc-port 9934 --ws-port $PEER_PORT \

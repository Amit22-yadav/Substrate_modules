#!/bin/bash

# Run a development instance of the Rialto Substrate bridge node.
# To override the default port just export RIALTO_PORT=9944

SUBSTRATE_PORT="${SUBSTRATE_PORT:-9944}"

RUST_LOG=runtime=trace \
    ./target/release/substrate --dev --tmp \
    --rpc-cors=all --unsafe-rpc-external --unsafe-ws-external \
    --port 33033 --rpc-port 9933 --ws-port $SUBSTRATE_PORT \

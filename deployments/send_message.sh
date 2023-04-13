#!/bin/bash

# Used for manually sending a message to a running network.
#
# You could for example spin up a full network using the Docker Compose files
# we have (to make sure the message relays are running), but remove the message
# generator service. From there you may submit messages manually using this script.

PEER_PORT="${SUBSTRATE_PORT:-9945}"

case "$1" in
	remark)
		RUST_LOG=runtime=trace,substrate-relay=trace,bridge=trace \
		./target/release/bin-substrate send-message rialto-to-millau \
			--source-host localhost \
			--source-port $MILLAU_PORT \
			--source-signer //Alice \
			--target-signer //Bob \
			--lane 00000000 \
			--origin Target \
			remark \
		;;
	transfer)
		RUST_LOG=runtime=trace,substrate-relay=trace,bridge=trace \
		./target/release/bin-substrate send-message peer-to-substrate \
			--source-host localhost \
			--source-port $PEER_PORT \
			--source-signer //Alice \
			--target-signer //Bob \
			--lane 00000000 \
			--origin Target \
			transfer \
			--amount 100000000000000 \
			--recipient 5EjrnbTfqEv8hpM6TXQfUhH2S6ruBuX3dLqgL2GA8TKkSMdU \
		;;
	*) echo "A message type is require. Supported messages: remark, transfer."; exit 1;;
esac

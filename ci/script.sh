#!/usr/bin/env bash
# This script takes care of testing your crate
set -eux -o pipefail


# TODO: wait for https://github.com/casey/just/pull/465 and a new release, then use 
# the following lines instead
# curl -LSfs https://japaric.github.io/trust/install.sh | \
#   sh -s -- --git casey/just --force
# just tests
cargo build
cargo test --tests --examples
tests/mcp/journey-tests.sh target/debug/mcp

[ -d generated ] || git clone --depth=1 https://github.com/google-apis-rs/generated

cd generated
export MCP=$PWD/../target/debug/mcp
./ci/script.sh

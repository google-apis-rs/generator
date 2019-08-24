#!/usr/bin/env bash
# This script takes care of testing your crate
set -eux -o pipefail


curl -LSfs https://japaric.github.io/trust/install.sh | \
  sh -s -- --git casey/just --target x86_64-unknown-linux-musl --force

just tests

[ -d generated ] || git clone --depth=1 https://github.com/google-apis-rs/generated

cd generated
export MCP=$PWD/../target/debug/mcp
./ci/script.sh

#!/usr/bin/env bash
# This script takes care of testing your crate
set -eux -o pipefail

make tests

[ -d generated ] || git clone --depth=1 https://github.com/google-apis-rs/generated

cd generated
export MCP=$PWD/../target/debug/mcp
./ci/script.sh

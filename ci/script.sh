#!/usr/bin/env bash
# This script takes care of testing your crate
set -eux -o pipefail

make tests

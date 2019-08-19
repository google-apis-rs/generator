#!/bin/bash

set -eu -o pipefail
exe=${1:?First argument is the executable under test}
exe="$(cd "${exe%/*}" && pwd)/${exe##*/}"

rela_root="${0%/*}"
root="$(cd "${rela_root}" && pwd)"
# shellcheck source=./tests/utilities.sh
source "$root/utilities.sh"

WITH_FAILURE=1
SUCCESSFULLY=0

fixture="$root/fixtures"
snapshot="$root/shared.snapshots"

# shellcheck source=./tests/included-stateless-substitute.sh
source "$root/included-stateless-substitute.sh"


#!/usr/bin/env bash
set -euo pipefail

# Independent remote-Nix gate for the Member.Effort Codex argv contract.
# The assertions live in flow-nexus-core because codex_arguments is private;
# this script deliberately owns only the test invocation path.
repo=${1:-$(git rev-parse --show-toplevel)}
cd "$repo"
exec nix develop --command cargo test -p flow-nexus-core codex_arguments

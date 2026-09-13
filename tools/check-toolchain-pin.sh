#!/usr/bin/env bash
# Fail if rust-toolchain.toml has drifted from the rustc the flake pins.
#
# The compiler version lives in two places and nothing links them: flake.lock
# pins nixpkgs (which decides the devshell's rustc) and rust-toolchain.toml
# pins the version rustup resolves on the cargo-remote build host. They have to
# agree, or remote builds silently use a different compiler than local ones --
# the exact failure rust-toolchain.toml was added to prevent (see #57).
#
# Drift can only be introduced by editing one of those two files, so that is
# when CI runs this.
#
# Reads the version out of the locked nixpkgs by evaluation only -- no rustc is
# built or downloaded, which keeps this to a few seconds.
#
# Run: tools/check-toolchain-pin.sh

set -euo pipefail

cd "$(dirname "$0")/.."

for cmd in nix jq; do
    command -v "$cmd" >/dev/null || { echo "error: $cmd not found on PATH" >&2; exit 2; }
done

pinned=$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml)
if [[ -z $pinned ]]; then
    echo "error: no channel found in rust-toolchain.toml" >&2
    exit 2
fi

# Resolve through root.inputs rather than assuming the node is named "nixpkgs".
rev=$(jq -er '.nodes[.nodes.root.inputs.nixpkgs].locked.rev' flake.lock)
flake_rustc=$(nix eval --raw "github:NixOS/nixpkgs/$rev#rustc.version")

if [[ $pinned != "$flake_rustc" ]]; then
    cat >&2 <<EOF
error: toolchain pin drift

  rust-toolchain.toml channel : $pinned
  flake.lock nixpkgs rustc    : $flake_rustc

These must match. If the flake was updated deliberately, set channel to
$flake_rustc in rust-toolchain.toml. If not, revert the flake.lock change.
EOF
    exit 1
fi

echo "toolchain pin ok: $pinned"

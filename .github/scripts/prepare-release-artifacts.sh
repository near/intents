#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 <component> <build-directory> <release-directory>" >&2
  exit 2
fi

component=$1
build_dir=$2
release_dir=$3

case "$component" in
  defuse) artifacts=(defuse.wasm defuse.far.wasm) ;;
  global-deployer) artifacts=(defuse-global-deployer.wasm) ;;
  poa-factory) artifacts=(defuse-poa-factory.wasm) ;;
  poa-token) artifacts=(defuse-poa-token.wasm defuse-poa-token.no_registration.wasm) ;;
  wallet)
    artifacts=(
      defuse-wallet-ed25519.wasm
      defuse-wallet-no-sign.wasm
      defuse-wallet-webauthn-ed25519.wasm
      defuse-wallet-webauthn-p256.wasm
    )
    ;;
  *)
    echo "unsupported release component: $component" >&2
    exit 2
    ;;
esac

mkdir -p "$release_dir"
for artifact in "${artifacts[@]}"; do
  if [[ ! -f "$build_dir/$artifact" ]]; then
    echo "missing release artifact: $build_dir/$artifact" >&2
    exit 1
  fi
  cp "$build_dir/$artifact" "$release_dir/"

  abi=${artifact%.wasm}.abi.json
  if [[ -f "$build_dir/$abi" ]]; then
    cp "$build_dir/$abi" "$release_dir/"
  fi
done

(
  cd "$release_dir"
  find . -maxdepth 1 -type f ! -name SHA256SUMS -printf '%f\n' \
    | LC_ALL=C sort \
    | xargs -r sha256sum > SHA256SUMS
)

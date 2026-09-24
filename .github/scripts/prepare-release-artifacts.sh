#!/usr/bin/env bash

#
# Used by CI release flow: define for each component
# the list of WASMs for each component to push to the
# relative release
#
set -euo pipefail

validate_args() {
  if [[ $# -ne 3 ]]; then
    echo "usage: $0 <component> <build-directory> <release-directory>" >&2
    exit 2
  fi
}

select_artifacts() {
  local component=$1
  local -n selected_artifacts=$2

  case "$component" in
    defuse) selected_artifacts=(defuse.wasm defuse.far.wasm) ;;
    global-deployer) selected_artifacts=(defuse-global-deployer.wasm) ;;
    poa-factory) selected_artifacts=(defuse-poa-factory.wasm) ;;
    poa-token) selected_artifacts=(defuse-poa-token.wasm defuse-poa-token.no_registration.wasm) ;;
    wallet)
      selected_artifacts=(
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
}

copy_artifacts() {
  local build_dir=$1
  local release_dir=$2
  local artifact abi
  shift 2

  mkdir -p "$release_dir"
  for artifact in "$@"; do
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
}

generate_checksums() (
  cd "$1"
  find . -maxdepth 1 -type f ! -name SHA256SUMS -printf '%f\n' \
    | LC_ALL=C sort \
    | xargs -r sha256sum > SHA256SUMS
)

main() {
  validate_args "$@"

  local component=$1
  local build_dir=$2
  local release_dir=$3
  local -a out_artifacts_array

  select_artifacts "$component" out_artifacts_array
  copy_artifacts "$build_dir" "$release_dir" "${out_artifacts_array[@]}"
  generate_checksums "$release_dir"
}

main "$@"

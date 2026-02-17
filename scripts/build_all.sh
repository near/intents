#!/usr/bin/env bash
set -e

is_truthy() {
    case "${1}" in
        1|true) echo 1 ;;
        *) echo 0 ;;
    esac
}

build_contract() {
    local contract_name="$1"
    local skip_build="$2"

    if [ "$skip_build" -eq 1 ]; then
        echo "Skipping build of ${contract_name}"
    else
        echo "Building ${contract_name}"
        cargo build-$contract_name$BUILD_REPRODUCIBLE_FLAG
    fi
}

compute_sha256() {
     DEFUSE_OUT_DIR="${DEFUSE_OUT_DIR:-res}"

    for wasm in "${DEFUSE_OUT_DIR}"/*.wasm; do
        [ -e "$wasm" ] || continue
        sha_file="${wasm%.wasm}.sha256"
        echo "Generating SHA256 for $wasm -> $sha_file"
        sha256sum "$wasm" | awk '{print $1}' > "$sha_file"
    done
}

BUILD_REPRODUCIBLE=$(is_truthy "${DEFUSE_BUILD_REPRODUCIBLE:-0}")

# SKIP_DEFUSE_BUILD=$(is_truthy "${SKIP_DEFUSE_BUILD:-0}")
# SKIP_POA_BUILD=$(is_truthy "${SKIP_POA_BUILD:-0}")
# SKIP_ESCROW_BUILD=$(is_truthy "${SKIP_ESCROW_BUILD:-0}")
# SKIP_GLOBAL_DEPLOYER_BUILD=$(is_truthy "${SKIP_GLOBAL_DEPLOYER_BUILD:-0}")
# SKIP_MULTI_TOKEN_RECEIVER_STUB_BUILD=$(is_truthy "${SKIP_MULTI_TOKEN_RECEIVER_STUB_BUILD:-0}")

# BUILD_REPRODUCIBLE_FLAG=""
# if [ "${BUILD_REPRODUCIBLE}" -eq 1 ]; then
#     echo "Building in reproducible mode"
#     BUILD_REPRODUCIBLE_FLAG="-reproducible"
# fi

# build_contract "defuse" "${SKIP_DEFUSE_BUILD}"
# build_contract "poa-factory" "${SKIP_POA_BUILD}"
# build_contract "escrow-swap" "${SKIP_ESCROW_BUILD}"
# build_contract "global-deployer" "${SKIP_GLOBAL_DEPLOYER_BUILD}"
# build_contract "multi-token-receiver-stub" "${SKIP_MULTI_TOKEN_RECEIVER_STUB_BUILD}"

if [ "$BUILD_REPRODUCIBLE" -eq 1 ]; then
    compute_sha256
fi

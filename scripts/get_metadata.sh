#!/bin/sh
# Usage:
#   ./get_metadata.sh                  # for all token_ids in VERIFIER_CONTRACT
#   ./get_metadata.sh TOKEN_ID ...     # for given TOKEN_ID(s)
#   ./get_metadata.sh < asset_ids.txt  # read token_ids from file
# Environment variables:
#   VERIFIER_CONTRACT (default: intents.near)
set -e

VERIFIER_CONTRACT=${VERIFIER_CONTRACT:-intents.near}

component() {
  # Usage: component ASSET_ID INDEX
  ASSET_ID="$1"
  COMPONENT="$2"
  printf '%s' "${ASSET_ID}" | cut -d':' -f"${COMPONENT}"
}

token_metadata() {
  # Usage: token_metadata ASSET_ID
  ASSET_ID="$1"
  ASSET_STANDARD="$(component "${ASSET_ID}" 1)"

  if [ "${ASSET_STANDARD}" = 'nep141' ]; then
    CONTRACT_ID="$(component "${ASSET_ID}" 2)"
    near --quiet contract call-function as-read-only "${CONTRACT_ID}" \
      ft_metadata json-args '{}' \
      network-config mainnet now 2>/dev/null \
      | jq "{ asset_id: \"${ASSET_ID}\", standard: \"${ASSET_STANDARD}\", contract_id: \"${CONTRACT_ID}\", metadata: . }"
  elif [ "${ASSET_STANDARD}" = 'nep171' ]; then
    CONTRACT_ID="$(component "${ASSET_ID}" 2)"
    TOKEN_ID="$(component "${ASSET_ID}" 3)"
    near --quiet contract call-function as-read-only "${CONTRACT_ID}" \
      nft_token json-args "{\"token_id\": \"${TOKEN_ID}\"}" \
      network-config mainnet now 2>/dev/null \
      | jq "{ asset_id: \"${ASSET_ID}\", standard: \"${ASSET_STANDARD}\", contract_id: \"${CONTRACT_ID}\", token_id: \"${TOKEN_ID}\", metadata: . }"
  elif [ "${ASSET_STANDARD}" = 'nep245' ]; then
    CONTRACT_ID="$(component "${ASSET_ID}" 2)"
    TOKEN_ID="$(component "${ASSET_ID}" 3)"
    near --quiet contract call-function as-read-only "${CONTRACT_ID}" \
      mt_metadata_base_by_token_id json-args "{\"token_ids\": [\"${TOKEN_ID}\"]}" \
      network-config mainnet now 2>/dev/null \
      | jq "{ asset_id: \"${ASSET_ID}\", standard: \"${ASSET_STANDARD}\", contract_id: \"${CONTRACT_ID}\", token_id: \"${TOKEN_ID}\", metadata: .[0] }"
  else
    echo "Unknown token standard: '${ASSET_STANDARD}'" >&2 && exit 1
  fi
}

token_metadata_all() {
  while read -r ASSET_ID; do
    token_metadata "${ASSET_ID}"
  done
}

mt_token_ids() {
  near --quiet contract call-function as-read-only "${VERIFIER_CONTRACT}" \
    mt_tokens json-args '{}' \
    network-config mainnet now \
    | jq -r '.[].token_id'
}

if [ $# -ne 0 ]; then
  printf '%s\n' $@
elif test -t 0; then
  mt_token_ids
else
  cat
fi | token_metadata_all


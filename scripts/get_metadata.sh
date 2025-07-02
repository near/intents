#!/bin/sh
set -e

VERIFIER_CONTRACT=${VERIFIER_CONTRACT:-intents.near}

if test -t 0; then
  near --quiet contract call-function as-read-only "${VERIFIER_CONTRACT}" \
    mt_tokens json-args '{}' \
    network-config mainnet now \
    | jq -r '.[].token_id' \
    | exec "$0"
fi

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

while read -r ASSET_ID; do
  token_metadata "${ASSET_ID}"
done

#!/bin/sh -e
VERIFIER_CONTRACT=${VERIFIER_CONTRACT:-intents.near}

if test -t 0; then
  near --quiet contract call-function as-read-only "${VERIFIER_CONTRACT}" \
    mt_tokens json-args '{}' \
    network-config mainnet now \
    | jq -r '.[].token_id' \
    | exec "$0"
fi

component() {
  TOKEN_ID="$1"
  COMPONENT="$2"
  printf "${TOKEN_ID}" | cut -d':' -f"${COMPONENT}"
}

token_metadata() {
  ASSET_ID="$1"
  ASSET_STANDARD="$(component "${ASSET_ID}" 1)"
  JQ_ARGS='.'

  if [ "${ASSET_STANDARD}" = 'nep141' ]; then
    CONTRACT_ID="$(component "${ASSET_ID}" 2)"
    METHOD_NAME='ft_metadata'
    JSON_ARGS='{}'
  elif [ "${ASSET_STANDARD}" = 'nep171' ]; then
    CONTRACT_ID="$(component "${ASSET_ID}" 2)"
    TOKEN_ID="$(component "${ASSET_ID}" 3)"
    METHOD_NAME='nft_token'
    JSON_ARGS="{\"token_id\": \"${TOKEN_ID}\"}"
  elif [ "${ASSET_STANDARD}" = 'nep245' ]; then
    CONTRACT_ID="$(component "${ASSET_ID}" 2)"
    TOKEN_ID="$(component "${ASSET_ID}" 3)"
    METHOD_NAME='mt_metadata_base_by_token_id'
    JSON_ARGS="{\"token_ids\": [\"${TOKEN_ID}\"]}"
    JQ_ARGS='.[0]'
  else
    echo "Unknown token standard: '${ASSET_STANDARD}'" >&2 && exit 1
  fi

  near --quiet contract call-function as-read-only "${CONTRACT_ID}" \
    "${METHOD_NAME}" json-args "${JSON_ARGS}" \
    network-config mainnet now 2>/dev/null \
    | jq "${JQ_ARGS} | { asset_id: \"${ASSET_ID}\" } + ."
}

while read -r ASSET_ID; do
  token_metadata "${ASSET_ID}"
done

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
  TOKEN_ID="$1"
  TOKEN_STANDARD="$(component "${TOKEN_ID}" 1)"
  JQ_ARGS='.'

  if [ "${TOKEN_STANDARD}" = 'nep141' ]; then
    CONTRACT_ID="$(component "${TOKEN_ID}" 2)"
    METHOD_NAME='ft_metadata'
    JSON_ARGS='{}'
  elif [ "${TOKEN_STANDARD}" = 'nep171' ]; then
    CONTRACT_ID="$(component "${TOKEN_ID}" 2)"
    METHOD_NAME='nft_metadata'
    JSON_ARGS='{}'
  elif [ "${TOKEN_STANDARD}" = 'nep245' ]; then
    CONTRACT_ID="$(component "${TOKEN_ID}" 2)"
    METHOD_NAME='mt_metadata_base_by_token_id'
    JSON_ARGS="{\"token_ids\": [\"$(component "${TOKEN_ID}" 3)\"]}"
    JQ_ARGS='.[0]'
  else
    echo "Unknown token standard: '${TOKEN_STANDARD}'" >&2 && exit 1
  fi

  near --quiet contract call-function as-read-only "${CONTRACT_ID}" \
    "${METHOD_NAME}" json-args "${JSON_ARGS}" \
    network-config mainnet now 2>/dev/null \
    | jq "${JQ_ARGS} | { asset_id: \"${TOKEN_ID}\" } + ."
}

while read -r TOKEN_ID; do
  token_metadata "${TOKEN_ID}"
done


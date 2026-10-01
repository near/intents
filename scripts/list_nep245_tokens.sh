#!/usr/bin/env bash
# Dump all token ids registered in intents.near into tokens.txt.
# Then compare mt_supply against intents.near's balance on each token contract:
# mismatches go to mismatch.txt, failed queries to skipped.txt.
# Env: NETWORK near network config name (default: mainnet)
#      BLOCK   block height
#      LIMIT   stop after this many tokens (default: no limit)
set -euo pipefail

OUT=tokens.txt
PAGE=5000
FROM=0

: >"$OUT"
: >mismatch.txt
: >skipped.txt
while :; do
  N=$PAGE
  if [ -n "${LIMIT:-}" ]; then
    [ "$FROM" -ge "$LIMIT" ] && break
    N=$(( (LIMIT - FROM) < PAGE ? (LIMIT - FROM) : PAGE ))
  fi
  RESULT=$(near --quiet contract call-function as-read-only intents.near mt_tokens json-args "{\"from_index\":\"$FROM\",\"limit\":$N}" network-config "${NETWORK:-mainnet}" at-block-height "${BLOCK:?BLOCK env var required}")
  COUNT=$(printf '%s' "$RESULT" | jq 'length')
  printf '%s' "$RESULT" | jq -r '.[].token_id' >>"$OUT"
  echo "fetched tokens $FROM .. $((FROM + COUNT))"
  [ "$COUNT" -lt "$N" ] && break
  FROM=$((FROM + COUNT))
  sleep 0.1
done
TOTAL=$(wc -l <"$OUT")
grep '^nep245' "$OUT" >"$OUT.tmp" || true
mv "$OUT.tmp" "$OUT"
echo "before filter: $TOTAL tokens"
echo "after filter:  $(wc -l <"$OUT") nep245 tokens in $OUT"

while read -r ASSET_ID; do
  CONTRACT="${ASSET_ID#nep245:}"; CONTRACT="${CONTRACT%%:*}"
  TOKEN="${ASSET_ID#"nep245:$CONTRACT:"}"
  SUPPLY=$(near --quiet contract call-function as-read-only intents.near mt_supply json-args "{\"token_id\":\"$ASSET_ID\"}" network-config "${NETWORK:-mainnet}" at-block-height "$BLOCK" 2>/dev/null | jq -r '.') || { echo "skip $ASSET_ID: supply query failed" >&2; echo "$ASSET_ID" >>skipped.txt; continue; }
  BALANCE=$(near --quiet contract call-function as-read-only "$CONTRACT" mt_balance_of json-args "{\"account_id\":\"intents.near\",\"token_id\":\"$TOKEN\"}" network-config "${NETWORK:-mainnet}" at-block-height "$BLOCK" 2>/dev/null | jq -r '.') || { echo "skip $ASSET_ID: balance query failed" >&2; echo "$ASSET_ID" >>skipped.txt; continue; }
  if [ "$(echo "$SUPPLY > $BALANCE" | bc 2>/dev/null)" = 1 ]; then
    echo "MISMATCH $ASSET_ID mt_supply=$SUPPLY intents.near balance=$BALANCE"
    echo "$ASSET_ID" >>mismatch.txt
  fi
done <"$OUT"
echo "mismatches: $(wc -l <mismatch.txt) in mismatch.txt, skipped: $(wc -l <skipped.txt) in skipped.txt"

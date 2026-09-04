#!/usr/bin/env bash
set -euo pipefail

: "${GH_TOKEN:?GH_TOKEN must be set}"
: "${TAG:?TAG must be set}"
: "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY must be set}"

issuer=https://token.actions.githubusercontent.com
identity_regexp="^https://github.com/${GITHUB_REPOSITORY}/.github/workflows/"
marker='<!-- sigstore-verification -->'
notes=$(gh release view "$TAG" --json body -q .body)
notes=${notes%%$marker*}

{
  printf '%s\n\n' "$notes"
  echo "$marker"
  echo '### Verify release artifacts'
  echo
  echo 'Download all assets from this release, then run:'
  echo
  echo '```sh'
  echo 'cosign verify-blob \'
  echo '  --bundle SHA256SUMS.sigstore.json \'
  echo "  --certificate-oidc-issuer=${issuer} \\"
  echo "  --certificate-identity-regexp='${identity_regexp}' \\"
  echo '  SHA256SUMS'
  echo 'sha256sum --check SHA256SUMS'
  echo '```'
} > /tmp/intents-release-notes.md

gh release edit "$TAG" --notes-file /tmp/intents-release-notes.md

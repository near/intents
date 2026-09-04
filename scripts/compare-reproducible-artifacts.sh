#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <baseline-directory> <candidate-directory>" >&2
  exit 2
fi

checksums() {
  local directory=$1
  (
    cd "$directory"
    find . -type f -printf '%P\n' \
      | LC_ALL=C sort \
      | while IFS= read -r artifact; do sha256sum "$artifact"; done
  )
}

baseline_file=$(mktemp)
candidate_file=$(mktemp)
trap 'rm -f "$baseline_file" "$candidate_file"' EXIT
checksums "$1" > "$baseline_file"
checksums "$2" > "$candidate_file"

if ! diff -u "$baseline_file" "$candidate_file"; then
  echo "reproducible build artifacts changed" >&2
  exit 1
fi

echo "reproducible build artifacts are byte-for-byte identical"

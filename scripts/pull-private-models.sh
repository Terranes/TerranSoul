#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODELS_DIR="${ROOT_DIR}/public/models/default"
ARCHIVE_PATH="$(mktemp)"

cleanup() {
  rm -f "${ARCHIVE_PATH}"
}
trap cleanup EXIT

if [[ -z "${TERRANSOUL_PRIVATE_MODELS_URL:-}" ]]; then
  echo "TERRANSOUL_PRIVATE_MODELS_URL is required." >&2
  exit 1
fi

CURL_ARGS=(-fsSL)
if [[ -n "${TERRANSOUL_PRIVATE_MODELS_TOKEN:-}" ]]; then
  CURL_ARGS+=(-H "Authorization: Bearer ${TERRANSOUL_PRIVATE_MODELS_TOKEN}")
fi
CURL_ARGS+=("${TERRANSOUL_PRIVATE_MODELS_URL}" -o "${ARCHIVE_PATH}")

curl "${CURL_ARGS[@]}"

if [[ -n "${TERRANSOUL_PRIVATE_MODELS_SHA256:-}" ]]; then
  echo "${TERRANSOUL_PRIVATE_MODELS_SHA256}  ${ARCHIVE_PATH}" | sha256sum --check --status
fi

mkdir -p "${MODELS_DIR}"
find "${MODELS_DIR}" -maxdepth 1 -type f -name '*.vrm' -delete
tar -xzf "${ARCHIVE_PATH}" -C "${MODELS_DIR}" --no-same-owner

required_models=(
  "Annabelle the Sorcerer.vrm"
  "M58.vrm"
  "2250278607152806301.vrm"
)

for model_file in "${required_models[@]}"; do
  if [[ ! -f "${MODELS_DIR}/${model_file}" ]]; then
    echo "Missing expected model file: ${model_file}" >&2
    exit 1
  fi
done

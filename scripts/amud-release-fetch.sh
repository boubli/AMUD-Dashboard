#!/usr/bin/env bash
# Shared secure curl + GitHub release asset helpers for AMUD installers.
# shellcheck disable=SC2034

CURL_SECURE=(--proto '=https' --tlsv1.2)

amud_curl() {
  curl "${CURL_SECURE[@]}" "$@"
  return $?
}

amud_fetch_latest_tag() {
  local repo="$1"
  local tag
  tag=$(amud_curl -sS "https://api.github.com/repos/${repo}/releases/latest" \
    | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || true)
  if [[ -z "$tag" ]]; then
    tag="v1.0.0"
  fi
  printf '%s' "$tag"
  return 0
}

amud_download_release_asset() {
  local repo="$1"
  local tag="$2"
  local name="$3"
  local dest="$4"
  amud_curl -L -sS -f -o "$dest" \
    "https://github.com/${repo}/releases/download/${tag}/${name}"
  return $?
}

amud_verify_release_asset() {
  local sums_file="$1"
  local file="$2"
  local name="$3"
  local expected actual
  expected=$(grep -E "[[:space:]/]${name}$" "$sums_file" | awk '{print $1}' || true)
  if [[ -z "$expected" ]]; then
    echo "Checksum for ${name} not found in SHA256SUMS" >&2
    return 1
  fi
  actual=$(sha256sum "$file" | awk '{print $1}' || true)
  if [[ "$actual" != "$expected" ]]; then
    echo "Checksum verification failed for ${name}" >&2
    return 1
  fi
}

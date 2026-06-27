#!/usr/bin/env bash
set -euo pipefail

base_url="${WEBDAV_URL:-http://127.0.0.1:3000/dav}"
base_url="${base_url%/}"
destination_url="${WEBDAV_DESTINATION_URL:-https://proxy.example.test/reverse/prefix/dav}"
destination_url="${destination_url%/}"
username="${WEBDAV_USERNAME:-webdav}"
token="${WEBDAV_TOKEN:?Set WEBDAV_TOKEN to an API token with WebDAV write access}"
root="codex-smoke-$(date +%s)"
evidence_dir="${WEBDAV_EVIDENCE_DIR:-docs/evidence/webdav-smoke-$(date +%Y%m%d-%H%M%S)}"
lock_token=""

mkdir -p "$evidence_dir"
exec > >(tee "$evidence_dir/webdav-smoke.log") 2>&1

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'missing required command: %s\n' "$1" >&2
    exit 127
  fi
}

optional_command() {
  if command -v "$1" >/dev/null 2>&1; then
    printf 'optional client available: %s\n' "$1"
  else
    printf 'optional client missing: %s\n' "$1"
  fi
}

curl_dav() {
  curl --fail-with-body --silent --show-error --user "$username:$token" "$@"
}

curl_status() {
  local body_file="$1"
  shift
  curl --silent --show-error --user "$username:$token" --output "$body_file" --write-out '%{http_code}' "$@"
}

curl_cleanup() {
  curl --silent --show-error --user "$username:$token" --output /dev/null "$@" >/dev/null 2>&1 || true
}

expect_status() {
  local expected="$1"
  local body_file="$2"
  shift 2
  local status
  status="$(curl_status "$body_file" "$@")"
  if [[ "$status" != "$expected" ]]; then
    printf 'expected HTTP %s, got %s\n' "$expected" "$status" >&2
    cat "$body_file" >&2 || true
    exit 1
  fi
}

cleanup() {
  if [[ -n "${lock_token:-}" ]]; then
    curl_cleanup -X UNLOCK -H "Lock-Token: <$lock_token>" "$base_url/$root/renamed.txt"
  fi
  curl_cleanup -X DELETE "$base_url/$root-copy"
  curl_cleanup -X DELETE "$base_url/$root"
}
trap cleanup EXIT

require_command curl
optional_command rclone
optional_command cadaver

printf 'WebDAV smoke start: %s\n' "$(date -Iseconds)"
printf 'base_url=%s\n' "$base_url"
printf 'destination_url=%s\n' "$destination_url"
printf 'evidence_dir=%s\n' "$evidence_dir"

curl_dav -X OPTIONS -i "$base_url/" | tee "$evidence_dir/options.txt" | grep -qi "DAV: 1, 2"
curl_dav -X MKCOL "$base_url/$root" >/dev/null
printf 'hello webdav smoke\n' | curl_dav -X PUT --data-binary @- "$base_url/$root/file.txt" >/dev/null
curl_dav -X PROPFIND -H "Depth: 1" "$base_url/$root" | tee "$evidence_dir/propfind-depth-1.xml" | grep -q "file.txt"
curl_dav -X COPY -H "Destination: $base_url/$root-copy" "$base_url/$root" >/dev/null
curl_dav -X MOVE -H "Destination: $destination_url/$root/renamed%20via%20proxy.txt" "$base_url/$root/file.txt" >/dev/null
curl_dav -X MOVE -H "Destination: renamed.txt" "$base_url/$root/renamed%20via%20proxy.txt" >/dev/null
curl_dav -i -H "Range: bytes=-6" "$base_url/$root/renamed.txt" | tee "$evidence_dir/range.txt" | grep -q "206 Partial Content"
grep -q "smoke" "$evidence_dir/range.txt"

lock_response="$evidence_dir/lock-response.txt"
curl_dav -X LOCK -i "$base_url/$root/renamed.txt" | tee "$lock_response" >/dev/null
lock_token="$(awk -F'[<>]' 'tolower($0) ~ /^lock-token:/ { print $2 }' "$lock_response")"
test -n "$lock_token"

locked_body="$evidence_dir/locked-write-body.txt"
printf 'blocked write\n' | expect_status 423 "$locked_body" -X PUT --data-binary @- "$base_url/$root/renamed.txt"
printf 'locked-write-status=423\n' | tee "$evidence_dir/locked-write-status.txt" >/dev/null
printf 'locked write\n' | curl_dav -X PUT -H "If: (<$lock_token>)" --data-binary @- "$base_url/$root/renamed.txt" >/dev/null
curl_dav -X UNLOCK -H "Lock-Token: <$lock_token>" "$base_url/$root/renamed.txt" >/dev/null
lock_token=""

curl_dav -X DELETE "$base_url/$root-copy" >/dev/null
curl_dav -X DELETE "$base_url/$root" >/dev/null

printf 'WebDAV smoke passed for %s\n' "$base_url"

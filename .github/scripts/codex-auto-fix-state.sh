#!/usr/bin/env bash
set -euo pipefail

mode="${1:-apply}"

current="${CURRENT_ROUND:-${CURRENT_ROUND_LABEL:-gemini-review-round-1}}"
fixed="${FIXED:-false}"
push_blocked="${PUSH_BLOCKED:-false}"
pending_count="${PENDING_COUNT:-0}"
max_rounds="${MAX_ROUNDS:-2}"
strict="${CODEX_AUTO_FIX_STRICT:-false}"

round_label() {
  printf 'gemini-review-round-%s' "$1"
}

round_number() {
  case "$1" in
    gemini-review-round-max) printf '%s' "$max_rounds" ;;
    gemini-review-round-*) printf '%s' "${1#gemini-review-round-}" ;;
    *) printf '1' ;;
  esac
}

bool() {
  local value
  value="$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')"
  case "$value" in
    true|1|yes) printf 'true' ;;
    *) printf 'false' ;;
  esac
}

fixed="$(bool "$fixed")"
push_blocked="$(bool "$push_blocked")"
strict="$(bool "$strict")"
current_number="$(round_number "$current")"
next_number=$((current_number + 1))
next_round="gemini-review-round-max"
if (( next_number <= max_rounds )); then
  next_round="$(round_label "$next_number")"
fi

action="advance"
request_review="true"
ready_to_merge="false"
human_block="false"
state_label=""

if [[ "$strict" != "true" ]]; then
  action="relaxed_clear"
  next_round="gemini-review-round-max"
  request_review="false"
  ready_to_merge="true"
  human_block="false"
  state_label="gemini-review-clean"
elif [[ "$current" == "gemini-review-round-max" ]]; then
  action="max_stop"
  request_review="false"
  ready_to_merge="false"
elif [[ "$push_blocked" == "true" ]]; then
  action="push_blocked"
  next_round="$current"
  request_review="false"
  human_block="true"
  state_label="gemini-review-needs-human"
elif (( pending_count > 0 )) && [[ "$fixed" == "false" ]]; then
  if [[ "$strict" == "true" ]]; then
    action="needs_human"
    next_round="$current"
    request_review="false"
    human_block="true"
    state_label="gemini-review-needs-human"
  elif (( current_number >= max_rounds )); then
    action="complete_with_pending"
    next_round="gemini-review-round-max"
    request_review="false"
    human_block="true"
    state_label="gemini-review-needs-human"
  else
    action="advance_with_pending"
    request_review="true"
    state_label="gemini-review-pending"
  fi
elif (( pending_count > 0 )) && [[ "$fixed" == "true" ]]; then
  if (( current_number >= max_rounds )); then
    action="complete_with_pending"
    next_round="gemini-review-round-max"
    request_review="false"
    human_block="true"
    state_label="gemini-review-needs-human"
  else
    action="advance_with_pending"
    request_review="true"
    state_label="gemini-review-pending"
  fi
elif (( current_number >= max_rounds )); then
  action="complete"
  next_round="gemini-review-round-max"
  request_review="false"
  ready_to_merge="true"
  state_label="gemini-review-clean"
fi

print_plan() {
  printf 'action=%s\n' "$action"
  printf 'current_round=%s\n' "$current"
  printf 'next_round=%s\n' "$next_round"
  printf 'request_review=%s\n' "$request_review"
  printf 'ready_to_merge=%s\n' "$ready_to_merge"
  printf 'human_block=%s\n' "$human_block"
  printf 'state_label=%s\n' "$state_label"
}

ensure_label() {
  local name="$1"
  local color="$2"
  local description="$3"
  gh label create "$name" --color "$color" --description "$description" >/dev/null 2>&1 || true
}

gh_retry() {
  local attempt
  for attempt in 1 2 3; do
    if "$@"; then
      return 0
    fi
    sleep $((attempt * 2))
  done
  echo "::warning::GitHub operation failed after retries: $*" >&2
  return 1
}

remove_label() {
  gh_retry gh api \
    --method DELETE \
    "repos/${GH_REPO:?}/issues/${PR_NUMBER:?}/labels/$1" \
    >/dev/null 2>&1 || true
}

add_label() {
  gh_retry gh api \
    --method POST \
    "repos/${GH_REPO:?}/issues/${PR_NUMBER:?}/labels" \
    -f "labels[]=$1" \
    >/dev/null || true
}

post_comment() {
  gh_retry gh api \
    --method POST \
    "repos/${GH_REPO:?}/issues/${PR_NUMBER:?}/comments" \
    -f "body=$1" \
    >/dev/null || true
}

markdown_table_cell() {
  local value="$1"
  value="${value//$'\r'/ }"
  value="${value//$'\n'/ }"
  value="${value//$'\t'/ }"
  value="${value//|/\\|}"
  value="$(printf '%s' "$value" | sed -E 's/[[:space:]]+/ /g; s/^ //; s/ $//')"
  if (( ${#value} > 240 )); then
    value="${value:0:237}..."
  fi
  printf '%s' "$value"
}

review_status_label() {
  case "$1" in
    resolved) printf '✅ 已解决' ;;
    blocked) printf '⚠️ 推送阻塞' ;;
    tracked) printf '📘 已记录' ;;
    *) printf '🧭 未解决' ;;
  esac
}

review_solution_for_status() {
  local status="$1"
  local suggestion="$2"
  local explanation="$3"
  local failure_reason="$4"

  case "$status" in
    resolved)
      printf 'Codex 已自动处理：%s' "$explanation"
      ;;
    blocked)
      printf '自动推送/安全检查阻塞：%s' "${failure_reason:-$explanation}"
      ;;
    tracked)
      printf '已记录为追踪项：%s' "$explanation"
      ;;
    *)
      if [[ -n "$suggestion" ]]; then
        printf '建议修复：%s' "$suggestion"
      elif [[ -n "$failure_reason" ]]; then
        printf '未自动修复原因：%s' "$failure_reason"
      else
        printf '建议按 Gemini 反馈人工修复；自动化说明：%s' "$explanation"
      fi
      ;;
  esac
}

relaxed_review_status_comment() {
  local result_path="${CODEX_RESULT_PATH:-/tmp/codex-result.json}"
  local rows_file="${RUNNER_TEMP:-/tmp}/codex-review-status-${PR_NUMBER:-pr}.tsv"
  local rows=""
  local issue_count=0

  if [[ ! -f "$result_path" ]] || ! command -v jq >/dev/null 2>&1; then
    printf '🤖 **Codex 已按宽松模式清理本轮 Gemini Review。**\n\n未找到可用于生成表格的自动修复结果 JSON，因此本轮只清理状态；请查看上方 Codex 自动修复评论或 workflow 日志。'
    return 0
  fi

  if ! jq -r '
    (.issue_statuses // [])
    | to_entries[]
    | [
        (.key + 1),
        (.value.severity // ""),
        (.value.file // ""),
        ((.value.line // 0) | tostring),
        (.value.description // "" | gsub("[\r\n\t]"; " ")),
        (.value.status // "pending"),
        (.value.explanation // "" | gsub("[\r\n\t]"; " ")),
        (.value.suggestion // "" | gsub("[\r\n\t]"; " ")),
        (.value.failure_reason // "" | gsub("[\r\n\t]"; " "))
      ]
    | @tsv
  ' "$result_path" > "$rows_file"; then
    printf '🤖 **Codex 已按宽松模式清理本轮 Gemini Review。**\n\n自动修复结果 JSON 解析失败，因此本轮只清理状态；请查看上方 Codex 自动修复评论或 workflow 日志。'
    return 0
  fi

  while IFS=$'\t' read -r index severity file line description status explanation suggestion failure_reason; do
    [[ -n "${index:-}" ]] || continue
    issue_count=$((issue_count + 1))
    local location="$file"
    if [[ "${line:-0}" != "0" ]]; then
      location="${location}:${line}"
    fi
    local solution
    solution="$(review_solution_for_status "$status" "$suggestion" "$explanation" "$failure_reason")"
    rows+=$(printf '| %s | %s | `%s` | %s | %s | %s |\n' \
      "$index" \
      "$(markdown_table_cell "$severity")" \
      "$(markdown_table_cell "$location")" \
      "$(markdown_table_cell "$description")" \
      "$(markdown_table_cell "$(review_status_label "$status")")" \
      "$(markdown_table_cell "$solution")")
  done < "$rows_file"

  if (( issue_count == 0 )); then
    printf '🤖 **Codex 已按宽松模式清理本轮 Gemini Review。**\n\n本轮自动修复结果没有结构化 Gemini 问题表；请查看上方 Codex 自动修复评论或 workflow 日志。'
    return 0
  fi

  cat <<EOF
🤖 **Codex 已按宽松模式清理本轮 Gemini Review。**

宽松模式只表示这些问题不再阻塞自动闭环，不代表每一项都已代码修复。下面是本轮 Gemini 问题与 Codex 处理状态：

| # | 严重级别 | 位置 | Gemini 问题 | Codex 状态 | 解决方案/说明 |
|---|---|---|---|---|---|
${rows}
如需更严格审核，请临时设置 \`CODEX_AUTO_FIX_STRICT=true\` 后重跑。
EOF
}

issue_status_note="问题清单见上方 Codex 分析评论中的 \`Medium/Medium+/High/Critical 对应状态\` 表；每个 Gemini 问题都会标记已解决、未解决或推送阻塞。"

apply_plan() {
  ensure_label "gemini-review-round-1" "6f42c1" "Gemini/Codex review loop round 1"
  ensure_label "gemini-review-round-2" "6f42c1" "Gemini/Codex review loop round 2"
  ensure_label "gemini-review-round-max" "5319e7" "Gemini/Codex automated review loop completed"
  ensure_label "gemini-review-pending" "d29922" "Codex has pending Medium/Medium+/High/Critical review items while another round is queued"
  ensure_label "gemini-review-needs-human" "b60205" "Automated review loop requires human decision"
  ensure_label "gemini-review-clean" "0e8a16" "Automated review loop has no pending Medium/Medium+/High/Critical findings"

  labels_to_clear=(
    "gemini-review-round-1"
    "gemini-review-round-2"
    "gemini-review-round-max"
    "gemini-review-pending"
    "gemini-review-needs-human"
    "gemini-review-clean"
  )
  for label in "${labels_to_clear[@]}"; do
    remove_label "$label"
  done

  case "$action" in
    relaxed_clear)
      add_label "gemini-review-round-max"
      add_label "gemini-review-clean"
      post_comment "$(relaxed_review_status_comment)"
      ;;
    max_stop)
      add_label "gemini-review-round-max"
      post_comment "🤖 **Codex 自动修复已达到 ${max_rounds} 轮上限。** ${issue_status_note} 请人工 Review 后决定合并或重跑。"
      ;;
    push_blocked)
      add_label "$current"
      add_label "gemini-review-needs-human"
      post_comment "🤖 **Codex 自动修复已阻塞。** 安全审计 fail-closed，未推送自动修复。${issue_status_note} 请人工处理后决定是否重跑 Gemini Review。"
      ;;
    needs_human)
      add_label "$current"
      add_label "gemini-review-needs-human"
      post_comment "🤖 **Codex 未能清理当前 Gemini Review 的 Medium/Medium+/High/Critical 问题。** ${issue_status_note} 本轮不会误判为可合并，请人工处理或重跑。"
      ;;
    advance|advance_with_pending)
      add_label "$next_round"
      if [[ -n "$state_label" ]]; then
        add_label "$state_label"
      fi
      review_requested_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
      post_comment "/gemini review"
      if [[ "${WAIT_FOR_GEMINI_REVIEW:-false}" == "true" ]]; then
        REVIEW_REQUESTED_AT="$review_requested_at" bash .github/scripts/gemini-review-watchdog.sh watch
      fi
      if [[ "$action" == "advance_with_pending" ]]; then
        post_comment "🤖 **Codex 已推送部分修复，并请求下一轮 Gemini Review。** ${issue_status_note} 当前仍有 Medium/Medium+/High/Critical 未自动修复说明，下一轮后仍存在则需要人工决策。"
      else
        post_comment "🤖 **Codex 本轮已完成，已请求下一轮 Gemini Review。** ${issue_status_note}"
      fi
      ;;
    complete)
      add_label "gemini-review-round-max"
      add_label "gemini-review-clean"
      post_comment "🤖 **Codex/Gemini 自动 Review 闭环已完成 ${max_rounds} 轮，当前没有 Medium/Medium+/High/Critical 未处理项。** ${issue_status_note} 请人工做最终 diff Review 后决定是否合并。"
      ;;
    complete_with_pending)
      add_label "gemini-review-round-max"
      add_label "gemini-review-needs-human"
      post_comment "🤖 **Codex/Gemini 自动 Review 已达到 ${max_rounds} 轮，但仍有 Medium/Medium+/High/Critical 未自动修复说明。** ${issue_status_note} 请人工处理或明确接受这些 pending 后再合并。"
      ;;
  esac
}

case "$mode" in
  plan) print_plan ;;
  apply) apply_plan ;;
  *) echo "usage: $0 [plan|apply]" >&2; exit 2 ;;
esac

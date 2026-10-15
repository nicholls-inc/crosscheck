#!/bin/sh
# Usage: pr-gate.sh PR [OWNER/REPO]
# Prints one JSON object: is PR's HEAD approved by pr-swarm, and is CI green on it.
# Waits up to GATE_WAIT_S seconds (default 900) for pending checks to finish.
# Read-only. Exit 0 whenever it could read the PR, whatever the verdict.
set -eu
PR="$1"
REPO="${2:-$(gh repo view --json nameWithOwner -q .nameWithOwner)}"
OWNER="${REPO%/*}"
NAME="${REPO#*/}"
WAIT="${GATE_WAIT_S:-900}"
deadline=$(( $(date +%s) + WAIT ))

while :; do
  view=$(gh pr view "$PR" --repo "$REPO" --json headRefOid,state,isDraft,statusCheckRollup)
  pending=$(printf '%s' "$view" | jq '[.statusCheckRollup[] | select((.status // (if (.state == "PENDING" or .state == "EXPECTED") then "PENDING" else "COMPLETED" end)) != "COMPLETED")] | length')
  if [ "$pending" = 0 ] || [ "$(date +%s)" -ge "$deadline" ]; then break; fi
  sleep 30
done

summary=$(gh api "repos/$REPO/issues/$PR/comments" --paginate \
  --jq '[.[] | select(.body | contains("<!-- pr-swarm-summary -->")) | .body][0] // ""' | jq -Rs .)

threads=$(gh api graphql --paginate -F owner="$OWNER" -F repo="$NAME" -F num="$PR" -f query='
  query($owner:String!, $repo:String!, $num:Int!, $endCursor:String) {
    repository(owner:$owner, name:$repo) { pullRequest(number:$num) {
      reviewThreads(first:100, after:$endCursor) {
        pageInfo { hasNextPage endCursor }
        nodes { isResolved isOutdated } } } } }' \
  | jq -s '[.[].data.repository.pullRequest.reviewThreads.nodes[] | select(.isResolved == false and .isOutdated == false)] | length')

printf '%s' "$view" | jq --argjson summary "$summary" --argjson threads "$threads" --arg pr "$PR" --arg repo "$REPO" '
  # A rerun leaves the superseded run in the rollup; only the newest run of each check counts.
  ([.statusCheckRollup
    | group_by([.workflowName // "", .name // .context])[]
    | max_by(.startedAt // .completedAt // "")
    | {
      name: (.name // .context),
      done: ((.status // (if (.state == "PENDING" or .state == "EXPECTED") then "PENDING" else "COMPLETED" end)) == "COMPLETED"),
      result: (.conclusion // .state)
    }]) as $checks
  | ([$summary | capture("## Verdict: [^A-Z]*(?<verdict>[A-Z][A-Z _]*[A-Z])\\** <sub>\\(round (?<round>[0-9]+) @ (?<sha>[0-9a-f]+)\\)</sub>")][0] // null) as $v
  | ($v != null and (.headRefOid | startswith($v.sha))) as $at_head
  | ([$checks[] | select(.done and (.result | IN("SUCCESS", "SKIPPED", "NEUTRAL") | not)) | .name]) as $failed
  | ([$checks[] | select(.done | not) | .name]) as $pending
  | {
      pr: ($pr | tonumber), repo: $repo, head_sha: .headRefOid, state, draft: .isDraft,
      verdict: (if $v then ($v.verdict | gsub(" "; "_")) else null end),
      verdict_round: (if $v then ($v.round | tonumber) else null end),
      verdict_sha: (if $v then $v.sha else null end),
      verdict_at_head: $at_head,
      checks_total: ($checks | length), checks_pending: $pending, checks_failed: $failed,
      unresolved_threads: $threads
    }
  | .ai_approved = (.verdict_at_head and (.verdict | IN("APPROVE", "APPROVE_WITH_NITS")))
  | .ci_green = (.checks_total > 0 and (.checks_pending | length) == 0 and (.checks_failed | length) == 0)
  | .done = (.state == "OPEN" and .ai_approved and .ci_green)
'

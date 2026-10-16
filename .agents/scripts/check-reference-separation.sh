#!/usr/bin/env bash
# Usage: check-reference-separation.sh <revision range>     (e.g. origin/main..HEAD)
#
# The commit gate's reference-data rule, for every commit in a range (CI checks a push or a
# pull request): recorded evidence, snapshots and traces are never added or modified in the
# same commit as crates/ code, so an implementation can never be "verified" by reference data
# written alongside it. Renames (quarantine moves) are allowed. Merge commits are skipped.
set -u
range="$1"
failed=0
for commit in $(git rev-list --no-merges "$range"); do
    changes=$(git show --format= --name-status -M "$commit")
    reference=$(printf '%s\n' "$changes" | grep -E '^[AM][[:space:]]+corpus/(unverified/)?(evidence|snapshots|traces)/' || true)
    code=$(printf '%s\n' "$changes" | grep -E '^[AMD][[:space:]]+crates/' || true)
    if [ -n "$reference" ] && [ -n "$code" ]; then
        echo "::error::$(git log -1 --format='%h %s' "$commit"): reference data and crates/ code in one commit"
        printf '%s\n' "$reference" | head -5
        failed=1
    fi
done
if [ "$failed" -eq 0 ]; then
    echo "Reference data and engine code are in separate commits ($(git rev-list --no-merges --count "$range") commits checked)."
fi
exit "$failed"

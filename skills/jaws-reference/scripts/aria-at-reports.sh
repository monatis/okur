#!/usr/bin/env bash
# Usage: aria-at-reports.sh <output json>
#
# Saves what ARIA-AT's published reports (https://aria-at.w3.org/reports) recorded each screen
# reader say for each command of its candidate and recommended test plans: the response of its
# GraphQL API, every final report's test results with their AT version and output. JAWS's
# newer versions are there (2025.2508.120 when this was written, newer than old-pc's
# 2025.2505.43): `okur-lab aria-at-report --newer-jaws <output json>` lists the commands where
# the newest JAWS said otherwise than the recorded one.
#
# Research, never evidence: these outputs are typed or collected by ARIA-AT's testers on
# their machines, with their settings. They show what a newer JAWS says; what old-pc's JAWS
# says is still recorded there.
set -euo pipefail

[ $# -eq 1 ] || { echo "usage: aria-at-reports.sh <output json>" >&2; exit 2; }
OUT=$1

query='{ testPlanVersions(phases: [CANDIDATE, RECOMMENDED]) { id title phase gitSha testPlan { directory } testPlanReports(isFinal: true) { id at { key name } browser { name } finalizedTestResults { test { title rowNumber renderedUrl } atVersion { name } scenarioResults { output untestable scenario { commands { id text atOperatingMode } } assertionResults { passed assertion { phrase priority } } } } } } }'
body=$(mktemp)
trap 'rm -f "$body"' EXIT
printf '{"query":"%s"}' "$query" > "$body"
curl -sS --fail -m 300 -X POST -H 'content-type: application/json' --data @"$body" \
    https://aria-at.w3.org/api/graphql -o "$OUT"
if grep -q '"errors"' "$OUT"; then
    echo "aria-at-reports: the API answered with errors, see $OUT" >&2
    exit 1
fi
echo "aria-at-reports: saved $(wc -c < "$OUT") bytes to $OUT"

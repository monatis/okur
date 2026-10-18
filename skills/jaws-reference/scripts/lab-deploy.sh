#!/usr/bin/env bash
# Usage: lab-deploy.sh [--okur <exe>] [--recorder <x64 dll>] [--recorder32 <x86 dll>] [--peer <name>]
#
# Installs the harness on the reference machine straight into place with `opendesk push`, which
# verifies each file's SHA-256 and replaces the destination only once the whole file arrived:
#   bin/okur.exe            <- --okur (default target/agent/release/okur.exe), if built
#   bin/okur-ui.exe         <- okur-ui.exe beside it, if built (Okur's windows)
#   skill scripts           <- skills/jaws-reference/scripts/*.ps1
#   corpus fixtures         <- corpus/fixtures/*.html, *.ps1
#   corpus scenarios        <- corpus/scenarios/*.json
#   recorder DLLs           <- --recorder / --recorder32 (stop JAWS first if it uses the voice)
# Files whose installed copy already has the same hash are not sent. Run from the repository
# root. Replaces the Dropbox pair (lab-push.ps1 / lab-pull.ps1), which stays as a fallback.
set -euo pipefail

PEER=old-pc
OKUR=target/agent/release/okur.exe
RECORDER=
RECORDER32=
while [ $# -gt 0 ]; do
    case "$1" in
        --okur) OKUR=$2; shift 2 ;;
        --recorder) RECORDER=$2; shift 2 ;;
        --recorder32) RECORDER32=$2; shift 2 ;;
        --peer) PEER=$2; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done
LAB=C:/Users/ailabs

# One "local<TAB>remote" line per file to install.
files() {
    [ -f "$OKUR" ] && printf '%s\t%s\n' "$OKUR" "$LAB/sr-harness/bin/okur.exe"
    # Okur starts the program that draws its windows from its own folder.
    local ui
    ui="$(dirname "$OKUR")/okur-ui.exe"
    [ -f "$ui" ] && printf '%s\t%s\n' "$ui" "$LAB/sr-harness/bin/okur-ui.exe"
    for f in skills/jaws-reference/scripts/*.ps1; do
        printf '%s\t%s\n' "$f" "$LAB/skills/jaws-reference/scripts/$(basename "$f")"
    done
    for f in corpus/fixtures/*.html corpus/fixtures/*.ps1; do
        [ -f "$f" ] && printf '%s\t%s\n' "$f" "$LAB/corpus/fixtures/$(basename "$f")"
    done
    for f in corpus/scenarios/*.json; do
        printf '%s\t%s\n' "$f" "$LAB/corpus/scenarios/$(basename "$f")"
    done
    [ -n "$RECORDER" ] && printf '%s\t%s\n' "$RECORDER" "$LAB/sr-harness/recorder/sr_sapi_recorder64.dll"
    [ -n "$RECORDER32" ] && printf '%s\t%s\n' "$RECORDER32" "$LAB/sr-harness/recorder/sr_sapi_recorder32.dll"
    return 0
}

list=$(mktemp)
query=$(mktemp)
trap 'rm -f "$list" "$query"' EXIT
files > "$list"

# The installed copies' hashes, in one call: "<sha256> <remote path>", or "- <path>" if absent.
{
    echo '$paths = @('
    cut -f2 "$list" | sed "s/.*/'&',/"
    echo "'')"
    echo 'foreach ($p in $paths) { if (-not $p) { continue }; if (Test-Path -LiteralPath $p) { (Get-FileHash -Algorithm SHA256 -LiteralPath $p).Hash.ToLower() + " " + $p } else { "- " + $p } }'
} > "$query"
remote=$(opendesk exec --peer "$PEER" --timeout 120 --file "$query" | tr -d '\r' | grep -E '^([0-9a-f]{64}|-) ' || true)

sent=0
same=0
while IFS=$'\t' read -r local target; do
    want=$(sha256sum "$local" | cut -d' ' -f1)
    have=$(printf '%s\n' "$remote" | awk -v t="$target" '{ p = substr($0, index($0, " ") + 1); if (p == t) print $1 }')
    if [ "$want" = "$have" ]; then
        same=$((same + 1))
        continue
    fi
    opendesk push --peer "$PEER" "$local" "$target" > /dev/null
    echo "sent $local -> $target"
    sent=$((sent + 1))
done < "$list"
echo "lab-deploy: $sent file(s) sent, $same already installed (peer $PEER)"

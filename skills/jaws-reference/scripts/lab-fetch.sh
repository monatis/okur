#!/usr/bin/env bash
# Usage: lab-fetch.sh <remote dir> <local dir> [--peer <name>]
#
# Copies every file under <remote dir> on the reference machine into <local dir>, keeping the
# folder structure, with `opendesk pull` (SHA-256 verified). Files already present with the same
# hash are not fetched. For evidence, traces and snapshots, e.g.
#   lab-fetch.sh C:/Users/ailabs/sr-harness/evidence-out "$SCRATCH/evidence-out"
# Copy what you fetched into corpus/ unmodified.
set -euo pipefail

[ $# -ge 2 ] || { echo "usage: lab-fetch.sh <remote dir> <local dir> [--peer <name>]" >&2; exit 2; }
REMOTE=${1%/}
LOCAL=${2%/}
shift 2
PEER=old-pc
while [ $# -gt 0 ]; do
    case "$1" in
        --peer) PEER=$2; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

query=$(mktemp)
trap 'rm -f "$query"' EXIT
cat > "$query" <<EOF
\$root = (Resolve-Path -LiteralPath '$REMOTE').Path.TrimEnd('\\')
Get-ChildItem -LiteralPath \$root -Recurse -File | ForEach-Object {
    # A file still being written (a trace during a run) is locked: it comes on a later fetch.
    try { \$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath \$_.FullName -ErrorAction Stop).Hash.ToLower() } catch { return }
    \$hash + ' ' + \$_.FullName.Substring(\$root.Length + 1).Replace('\\', '/')
}
EOF
listing=$(timeout 360 opendesk exec --peer "$PEER" --timeout 300 --file "$query" | tr -d '\r' | grep -E '^[0-9a-f]{64} ' || true)

# One pull can hang on a connection that stopped answering (rounds 50, 54, 55): each gets a
# time limit and three tries, and a file that still fails is named and left for the next run.
PULL_TIMEOUT=${PULL_TIMEOUT:-180}
fetched=0
same=0
failed=()
while read -r hash rel; do
    [ -n "$hash" ] || continue
    dest="$LOCAL/$rel"
    if [ -f "$dest" ] && [ "$(sha256sum "$dest" | cut -d' ' -f1)" = "$hash" ]; then
        same=$((same + 1))
        continue
    fi
    mkdir -p "$(dirname "$dest")"
    got=no
    for _ in 1 2 3; do
        if timeout "$PULL_TIMEOUT" opendesk pull --peer "$PEER" "$REMOTE/$rel" "$dest" > /dev/null 2>&1 \
            && [ "$(sha256sum "$dest" | cut -d' ' -f1)" = "$hash" ]; then
            got=yes
            break
        fi
    done
    if [ "$got" = yes ]; then
        fetched=$((fetched + 1))
    else
        rm -f "$dest"
        failed+=("$rel")
    fi
done <<< "$listing"
echo "lab-fetch: $fetched file(s) fetched, $same already here, ${#failed[@]} failed, from $REMOTE (peer $PEER)"
for rel in "${failed[@]}"; do
    echo "lab-fetch: not fetched: $rel" >&2
done
[ ${#failed[@]} -eq 0 ]

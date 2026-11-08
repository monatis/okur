#!/usr/bin/env bash
# From the dev box: brings the Linux lab's AT-SPI snapshots
# (D:\okur-linux\snapshots\<stem>.chrome.atspi.json on old-pc, written by
# capture-atspi-snapshots.sh) into corpus/snapshots/, each as the capture wrote it. Run from the
# repository root:
#   bash skills/linux-lab/scripts/fetch-snapshots.sh [--peer old-pc] <fixture key>...
# Then `cargo run --bin okur-lab -- parity --api atspi`, and commit the snapshots with the
# manifest (corpus-manifest --write), apart from engine code. A snapshot is Chrome's page as
# Linux gives it, never JAWS evidence.
set -euo pipefail
PEER=old-pc
fixtures=()
while [ $# -gt 0 ]; do
    case "$1" in
        --peer) PEER=$2; shift 2 ;;
        *) fixtures+=("$1"); shift ;;
    esac
done
if [ ${#fixtures[@]} -eq 0 ]; then
    echo "usage: fetch-snapshots.sh [--peer old-pc] <fixture key>..." >&2
    exit 2
fi
failed=
for fixture in "${fixtures[@]}"; do
    stem="${fixture%.html}"
    name="${stem//\//.}.chrome.atspi.json"
    # A pull that fails (the peer briefly unreachable) is said, and the rest still fetched.
    if ! timeout 120 opendesk pull --peer "$PEER" "D:/okur-linux/snapshots/$name" "corpus/snapshots/$name" > /dev/null; then
        echo "$name: NOT fetched" >&2
        failed=1
        continue
    fi
    if ! grep -q '"tool_version": "okur-linux' "corpus/snapshots/$name"; then
        echo "$name: not an AT-SPI snapshot, removed" >&2
        rm -f "corpus/snapshots/$name"
        failed=1
        continue
    fi
    echo "fetched $name"
done
if [ -n "$failed" ]; then
    echo "fetch-snapshots: some snapshots were not fetched" >&2
    exit 1
fi

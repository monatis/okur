#!/usr/bin/env bash
# From the dev box: puts this repository's current branch into the Linux lab on old-pc (the WSL
# distribution "okur" on D:), as a git bundle, and checks it out at ~/okur there. Run from the
# repository root:
#   bash skills/linux-lab/scripts/sync.sh [--peer old-pc] [--distro okur]
# The lab's scripts (setup.sh, a11y-stack.sh, dns.sh) go along to D:\okur-linux. Uncommitted changes do
# not travel: commit first (the lab builds what the branch holds).
set -euo pipefail
PEER=old-pc
DISTRO=okur
while [ $# -gt 0 ]; do
    case "$1" in
        --peer) PEER=$2; shift 2 ;;
        --distro) DISTRO=$2; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done
branch=$(git rev-parse --abbrev-ref HEAD)
head=$(git rev-parse --short HEAD)
bundle=$(mktemp -u).bundle
git bundle create -q "$bundle" "$branch"
trap 'rm -f "$bundle"' EXIT
timeout 600 opendesk push --peer "$PEER" "$bundle" D:/okur-linux/okur.bundle > /dev/null
for script in setup.sh a11y-stack.sh dns.sh probe-chrome.sh; do
    timeout 120 opendesk push --peer "$PEER" "skills/linux-lab/scripts/$script" "D:/okur-linux/$script" > /dev/null
done
# Clone once, then fetch: the lab's tree lives in the distribution's own disk, where builds are fast.
inner="set -e; if [ ! -d ~/okur/.git ]; then git clone -q -b $branch /mnt/d/okur-linux/okur.bundle ~/okur; fi; cd ~/okur; git fetch -q /mnt/d/okur-linux/okur.bundle $branch:refs/remotes/bundle/$branch; git checkout -q -B $branch refs/remotes/bundle/$branch; git log --oneline -1"
timeout 300 opendesk exec --peer "$PEER" --timeout 280 "wsl.exe -d $DISTRO -u root -- bash -lc '$inner'" | tr -d '\000'
echo "synced $branch at $head to $PEER ($DISTRO)"

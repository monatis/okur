#!/usr/bin/env bash
# From the dev box: puts this repository's current branch into the Linux lab on old-pc (the WSL
# distribution "okur" on D:), as a git bundle, and checks it out at ~/okur there. Run from the
# repository root:
#   bash skills/linux-lab/scripts/sync.sh [--peer old-pc] [--distro okur]
# The lab's scripts (setup.sh, a11y-stack.sh, dns.sh, the probes) go along to D:\okur-linux. Uncommitted changes do
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
trap 'rm -f "$bundle"' EXIT
# Only the commits the lab lacks: the whole branch is minutes to send, a few commits seconds.
lab_head=$(timeout 120 opendesk exec --peer "$PEER" --timeout 100 \
    "wsl.exe -d $DISTRO -u root -- git -C /root/okur rev-parse HEAD" 2>/dev/null \
    | tr -d '\000\r' | grep -E '^[0-9a-f]{40}$' | tail -1 || true)
if [ -n "$lab_head" ] && [ "$lab_head" = "$(git rev-parse HEAD)" ]; then
    send=
elif [ -n "$lab_head" ] && git merge-base --is-ancestor "$lab_head" HEAD 2>/dev/null; then
    git bundle create -q "$bundle" "$lab_head..$branch"
    send=1
else
    git bundle create -q "$bundle" "$branch"
    send=1
fi
if [ -n "$send" ]; then
    timeout 600 opendesk push --peer "$PEER" "$bundle" D:/okur-linux/okur.bundle > /dev/null
fi
# Clone once, then fetch: the lab's tree lives in the distribution's own disk, where builds are
# fast. The lab's scripts (every one but the dev box's own, this one and job.sh) are copied to
# D:\okur-linux from the commit checked out, in the same call: pushed one by one, thirty of
# them outlasted a caller's timeout before the checkout came (2026-11-08).
checkout=
if [ -n "$send" ]; then
    checkout="if [ ! -d ~/okur/.git ]; then git clone -q -b $branch /mnt/d/okur-linux/okur.bundle ~/okur; fi; cd ~/okur; git fetch -q /mnt/d/okur-linux/okur.bundle $branch:refs/remotes/bundle/$branch; git checkout -q -B $branch refs/remotes/bundle/$branch;"
fi
# No shell variable in it: PowerShell, between opendesk and WSL, would expand it.
copy="cd ~/okur/skills/linux-lab/scripts; cp *.sh *.py /mnt/d/okur-linux/; rm -f /mnt/d/okur-linux/sync.sh /mnt/d/okur-linux/job.sh"
inner="set -e; $checkout $copy; cd ~/okur; git log --oneline -1"
lab=$(timeout 300 opendesk exec --peer "$PEER" --timeout 280 "wsl.exe -d $DISTRO -u root -- bash -lc '$inner'" | tr -d '\000\r')
echo "$lab"
# The lab's checkout must be this one: a sync that did not reach it is said, never assumed.
case "$lab" in
    *"$head"*) echo "synced $branch at $head to $PEER ($DISTRO)" ;;
    *) echo "sync FAILED: the lab is not at $head" >&2; exit 1 ;;
esac

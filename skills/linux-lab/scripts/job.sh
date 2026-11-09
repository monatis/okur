#!/usr/bin/env bash
# From the dev box: a long command in the Linux lab, detached, so it outlives `opendesk exec`'s
# timeout. The command runs as root in ~/okur inside the distribution; its output and exit code
# land in D:\okur-linux\jobs\<name>.log and <name>.done on old-pc. From the repository root:
#   bash skills/linux-lab/scripts/job.sh start probe1 'bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-chrome.sh'
#   bash skills/linux-lab/scripts/job.sh wait probe1 [lines]   # until it ends, then its exit and log
#   bash skills/linux-lab/scripts/job.sh log probe1 [lines]    # the log so far
#   bash skills/linux-lab/scripts/job.sh stop probe1            # the distribution stopped, job too
# The command is written into a bash file pushed to the lab, so no quoting passes through
# PowerShell. Never start a job while a JAWS batch runs (run-batch.ps1 shuts WSL down).
set -euo pipefail
PEER=${PEER:-old-pc}
DISTRO=${DISTRO:-okur}
JOBS=D:/okur-linux/jobs
action=${1:-}
name=${2:-}
case "$name" in
    ''|*[!A-Za-z0-9_-]*) echo "usage: job.sh start|wait|log|stop <name> ..." >&2; exit 2 ;;
esac
remote() { timeout 90 opendesk exec --peer "$PEER" --timeout 60 "$1" 2>&1 | tr -d '\000'; }
show_log() {
    remote "if (Test-Path $JOBS/$name.done) { 'exit: ' + (Get-Content $JOBS/$name.done) } else { 'running' }; if (Test-Path $JOBS/$name.log) { Get-Content $JOBS/$name.log -Tail $1 }"
}
case "$action" in
    start)
        command=${3:?the command to run}
        script=$(mktemp)
        trap 'rm -f "$script"' EXIT
        cat > "$script" <<EOF
#!/usr/bin/env bash
# Not a login shell: rustup's PATH comes from its env file.
if [ -f ~/.cargo/env ]; then . ~/.cargo/env; fi
cd ~/okur
{ $command
} > /mnt/d/okur-linux/jobs/$name.log 2>&1
echo \$? > /mnt/d/okur-linux/jobs/$name.done
EOF
        # Test-Path before Remove-Item: a missing file's error would fail `opendesk exec`.
        remote "New-Item -ItemType Directory -Force $JOBS | Out-Null; foreach (\$f in '$JOBS/$name.done', '$JOBS/$name.log') { if (Test-Path \$f) { Remove-Item \$f } }" > /dev/null
        timeout 120 opendesk push --peer "$PEER" "$script" "$JOBS/$name.job.sh" > /dev/null
        remote "Start-Process wsl.exe -WindowStyle Hidden -ArgumentList '-d','$DISTRO','-u','root','--','bash','/mnt/d/okur-linux/jobs/$name.job.sh'; 'started $name'"
        ;;
    wait)
        until remote "Test-Path $JOBS/$name.done" | grep -q True; do sleep 30; done
        show_log "${3:-60}"
        ;;
    log)
        show_log "${3:-60}"
        ;;
    stop)
        # Every process of the distribution goes with it (the job, Xvfb, Chrome, Okur), which
        # a11y-stack.sh's own ending would not see; the job is marked stopped.
        remote "wsl.exe --terminate $DISTRO | Out-Null; Set-Content -Path $JOBS/$name.done -Value stopped; 'stopped $name'"
        ;;
    *) echo "usage: job.sh start|wait|log|stop <name> ..." >&2; exit 2 ;;
esac

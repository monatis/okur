#!/usr/bin/env bash
# The lab's distribution gets public DNS servers. On old-pc, WSL's NAT networking does not start
# and WSL falls back to its user-mode networking ("Consomme"), where outbound connections work but
# the router's DNS (WSL's generated resolv.conf) does not answer (2026-11-05). As root:
#   wsl.exe -d okur -u root -- bash /mnt/d/okur-linux/dns.sh
set -euo pipefail
cat > /etc/wsl.conf <<'CONF'
[network]
generateResolvConf = false
CONF
rm -f /etc/resolv.conf
cat > /etc/resolv.conf <<'CONF'
nameserver 1.1.1.1
nameserver 8.8.8.8
CONF
for host in archive.ubuntu.com dl.google.com static.rust-lang.org; do
    if timeout 8 getent hosts "$host" > /dev/null; then echo "dns ok: $host"; else echo "dns FAILED: $host"; fi
done

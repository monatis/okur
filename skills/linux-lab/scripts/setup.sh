#!/usr/bin/env bash
# Sets up Okur's headless Linux lab inside a WSL distribution (Ubuntu 26.04), as root:
#   wsl.exe -d okur -u root -- bash /mnt/d/okur-linux/setup.sh
# The stack is the one Orca's and xa11y's own CI run (docs/references/linux-lab-proposal.md §4.2):
# a virtual X display (Xvfb), a private session bus, the AT-SPI bus and registry, a small window
# manager, GTK 4 and Chrome to read, speech-dispatcher with espeak-ng into a null sink, and Rust at
# the version rust-toolchain.toml pins. No desktop and no GUI on Windows' screen: this machine is
# the JAWS lab too. Safe to run again: it installs only what is missing.
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

apt-get update -q
apt-get install -y -q --no-install-recommends \
    build-essential pkg-config git curl ca-certificates wget gnupg \
    libdbus-1-dev libxcb1-dev libxkbcommon-dev libx11-dev libxtst-dev libxi-dev \
    xvfb x11-utils xauth dbus dbus-x11 \
    at-spi2-core libatk-adaptor libatk-bridge2.0-0 python3-gi gir1.2-atspi-2.0 \
    fluxbox gtk-4-examples gtk-3-examples \
    speech-dispatcher speech-dispatcher-espeak-ng espeak-ng pulseaudio-utils \
    fonts-dejavu-core

# Chrome from Google's own repository: Ubuntu's chromium is a snap, and snaps need systemd.
if ! command -v google-chrome >/dev/null 2>&1; then
    install -d -m 0755 /etc/apt/keyrings
    wget -q -O- https://dl.google.com/linux/linux_signing_key.pub \
        | gpg --dearmor -o /etc/apt/keyrings/google-chrome.gpg
    echo "deb [arch=amd64 signed-by=/etc/apt/keyrings/google-chrome.gpg] https://dl.google.com/linux/chrome/deb/ stable main" \
        > /etc/apt/sources.list.d/google-chrome.list
    apt-get update -q
    apt-get install -y -q google-chrome-stable
fi

# Rust for the lab's user (root here: the lab has no other), at the pinned version.
if ! command -v rustup >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/rustup" ]; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
fi
"$HOME/.cargo/bin/rustup" toolchain install 1.99.0 --profile minimal -c rustfmt -c clippy

echo "setup done: $(lsb_release -ds 2>/dev/null || cat /etc/os-release | head -1)"
google-chrome --version || true
"$HOME/.cargo/bin/rustc" +1.99.0 --version

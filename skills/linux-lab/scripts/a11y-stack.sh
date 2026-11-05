#!/usr/bin/env bash
# Runs a command inside a private, headless desktop for accessibility work: a virtual X display, a
# session bus of its own, the AT-SPI bus and registry with screen reading switched on, and a small
# window manager, as Orca's and xa11y's CI do. Everything stops when the command ends.
#   bash a11y-stack.sh <command> [args...]
# Inside the command: DISPLAY and DBUS_SESSION_BUS_ADDRESS are set; GTK and Chrome expose AT-SPI.
set -euo pipefail
if [ "${1:-}" = "--inner" ]; then
    shift
    # The daemons started here end with the command, not after it.
    trap 'kill $(jobs -p) 2>/dev/null' EXIT
    # The AT-SPI bus, and the switches toolkits read before they expose their trees.
    /usr/libexec/at-spi-bus-launcher --launch-immediately &
    sleep 0.5
    for switch in IsEnabled ScreenReaderEnabled; do
        busctl --user call org.a11y.Bus /org/a11y/bus org.freedesktop.DBus.Properties Set ssv \
            org.a11y.Status "$switch" b true 2>/dev/null || \
            dbus-send --session --dest=org.a11y.Bus /org/a11y/bus org.freedesktop.DBus.Properties.Set \
                string:org.a11y.Status "string:$switch" variant:boolean:true
    done
    /usr/libexec/at-spi2-registryd &
    fluxbox >/dev/null 2>&1 &
    sleep 1
    export ACCESSIBILITY_ENABLED=1 GTK_A11Y=atspi NO_AT_BRIDGE=0
    "$@"
    exit $?
fi
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/okur-runtime}"
mkdir -p "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
exec xvfb-run -a -s "-screen 0 1920x1080x24" dbus-run-session -- bash "$0" --inner "$@"

<#
.SYNOPSIS
    Windows Settings at System > Sound: output and input devices, the master volume, links
    (Windows 10).

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "Settings", "app_frame": true), as desktop-settings-home.ps1. Scenarios only move through
    the page and never change a setting: no arrow key on a combo box or the volume slider, no
    Space on a button. Do not change it without re-recording every scenario that uses it (the
    manifest pins its SHA-256).
#>
Start-Process 'ms-settings:sound'

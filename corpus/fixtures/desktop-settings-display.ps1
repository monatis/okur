<#
.SYNOPSIS
    Windows Settings at System > Display: a toggle, links and combo boxes (Windows 10).

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "Settings", "app_frame": true), as desktop-settings-home.ps1. Scenarios only move through
    the page and never change a setting: no arrow key on a combo box, no Space on the toggle.
    Do not change it without re-recording every scenario that uses it (the manifest pins its
    SHA-256).
#>
Start-Process 'ms-settings:display'

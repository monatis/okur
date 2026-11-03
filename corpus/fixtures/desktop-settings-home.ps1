<#
.SYNOPSIS
    Windows Settings at its home: the search box and the grid of its 13 categories (Windows 10).

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "Settings", "app_frame": true). Settings is a Store app: its window is a frame of
    ApplicationFrameHost's, closed by the runner between trials, so each trial opens it fresh.
    Do not change it without re-recording every scenario that uses it (the manifest pins its
    SHA-256).
#>
Start-Process 'ms-settings:'

<#
.SYNOPSIS
    Windows Settings at Apps & features (Windows 10; Windows 11's "Installed apps").

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "Settings", "app_frame": true). Settings is a Store app: its window is a frame of
    ApplicationFrameHost's, closed by the runner between trials, so each trial opens it fresh
    with the focus on its first control. The list is the reference machine's installed
    programs; scenarios read only its first items, Windows' own and 7-Zip, sorted by name.
    Do not change it without re-recording every scenario that uses it (the manifest pins its
    SHA-256).
#>
Start-Process 'ms-settings:appsfeatures'

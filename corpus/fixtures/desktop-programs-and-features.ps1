<#
.SYNOPSIS
    Control Panel's Programs and Features (appwiz.cpl), Windows' classic "Add or remove programs".

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "Control Panel\All Control Panel Items\Programs and Features", "shell_window": true,
    "focus_selected": false). The page is a File Explorer window, closed by the runner between
    trials, so each trial opens it fresh with the focus on its first control. The list is the
    reference machine's installed programs; scenarios read only its first items, sorted by
    name. Do not change it without re-recording every scenario that uses it (the manifest pins
    its SHA-256).
#>
Start-Process control.exe -ArgumentList 'appwiz.cpl'

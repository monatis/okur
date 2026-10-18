<#
.SYNOPSIS
    Deterministic console fixture for the Virtual Viewer in a console window.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). Runs in the
    console window PowerShell opens (the classic console on Windows 10), titles it, writes
    four lines, and waits at a prompt, so the cursor is on the last line. Do not change it
    without re-recording every scenario that uses it (the manifest pins its SHA-256).
#>
$Host.UI.RawUI.WindowTitle = 'SR Console Fixture'
Clear-Host
Write-Host 'First console line'
Write-Host 'Second console line, with C:\Windows\System32'
Write-Host ''
Write-Host 'Last line before the prompt'
$null = Read-Host 'Type here'

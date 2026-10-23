<#
.SYNOPSIS
    Deterministic console fixture for the text a console writes after a command (Screen Echo).

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). Runs in the
    console window PowerShell opens (the classic console on Windows 10), titles it, writes one
    line, and asks for a command; each answer is followed by two numbered lines and the
    question again, so every Enter writes new text below the cursor. Do not change it without
    re-recording every scenario that uses it (the manifest pins its SHA-256).
#>
$Host.UI.RawUI.WindowTitle = 'SR Console Echo Fixture'
Clear-Host
Write-Host 'Console echo fixture'
$n = 0
while ($true) {
    $answer = Read-Host 'Command'
    $n++
    Write-Host "Output line $n for $answer"
    Write-Host "Second output line $n"
}

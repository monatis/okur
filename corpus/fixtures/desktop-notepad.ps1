<#
.SYNOPSIS
    Deterministic Notepad fixture: a small code file opened in Windows Notepad.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "sr-notepad-fixture.txt - Notepad"). Writes the same text to %TEMP% on every run, then
    opens it in Notepad, so the application is the real Notepad the user works with. ASCII
    only. Do not change it without re-recording every scenario that uses it (the manifest
    pins its SHA-256).
#>
$path = Join-Path $env:TEMP 'sr-notepad-fixture.txt'
$lines = @(
    'fn main() {',
    '    let total = price * 1.5; // tax',
    '    println!("{total}");',
    '}'
)
[System.IO.File]::WriteAllText($path, ($lines -join "`r`n"), (New-Object System.Text.UTF8Encoding $false))
Start-Process notepad.exe -ArgumentList "`"$path`""

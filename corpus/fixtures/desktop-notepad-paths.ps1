<#
.SYNOPSIS
    Deterministic Notepad fixture: a path, a web address and an email address on one line.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "sr-notepad-paths.txt - Notepad"), for words read with punctuation in them (Ctrl+Right,
    Say Word) at each punctuation level. Writes the same text to %TEMP% on every run, then
    opens it in Notepad. ASCII only; the fixtures' "Ada" and a reserved domain only. Do not
    change it without re-recording every scenario that uses it (the manifest pins its
    SHA-256).
#>
$path = Join-Path $env:TEMP 'sr-notepad-paths.txt'
$lines = @(
    'C:\Users\Ada\Downloads or https://example.org/a-b or ada@example.org'
)
[System.IO.File]::WriteAllText($path, ($lines -join "`r`n"), (New-Object System.Text.UTF8Encoding $false))
Start-Process notepad.exe -ArgumentList "`"$path`""

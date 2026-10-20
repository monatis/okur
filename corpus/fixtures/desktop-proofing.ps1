<#
.SYNOPSIS
    Deterministic Notepad fixture: one text inconsistency per line, for JAWS's text analysis.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "sr-proofing-fixture.txt - Notepad"). Writes the same text to %TEMP% on every run, then
    opens it in Notepad. Each line holds one kind of inconsistency JAWS's Text Analyzer names
    (its help: mismatched symbols, stray or unspaced punctuation, extra spaces, missing
    capitals, inverted case), and the first line none. ASCII only. Do not change it without
    re-recording every scenario that uses it (the manifest pins its SHA-256).
#>
$path = Join-Path $env:TEMP 'sr-proofing-fixture.txt'
$lines = @(
    'This line is clean.',
    'Two  spaces here.',
    'no capital at the start.',
    'A stray , comma.',
    'An unspaced,comma.',
    'An open (paren.',
    'A close) paren.',
    'An open [bracket and a "quote.',
    'tHIS IS INVERTED.',
    'Ends with a space. '
)
[System.IO.File]::WriteAllText($path, ($lines -join "`r`n"), (New-Object System.Text.UTF8Encoding $false))
Start-Process notepad.exe -ArgumentList "`"$path`""

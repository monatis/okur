<#
.SYNOPSIS
    Deterministic Notepad fixture: more text for JAWS's text analysis, to find its rules.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "sr-proofing-fixture-2.txt - Notepad"). Writes the same text to %TEMP% on every run, then
    opens it in Notepad. Where desktop-proofing.ps1 showed one inconsistency per line, these
    lines ask how far JAWS pairs brackets (a parenthesis closed two lines later, one never
    closed, a bracket never opened) and which symbols, numbers, spaces and cases it names.
    ASCII only. Do not change it without re-recording every scenario that uses it.
#>
$path = Join-Path $env:TEMP 'sr-proofing-fixture-2.txt'
$lines = @(
    'One (open paren here.',
    'Nothing on this line.',
    'Here ) it closes.',
    'A lone (paren.',
    'A lone ] bracket.',
    'Math 2 * 3 = 6.',
    'Price 1.5 and 3,000 here.',
    'Twice  and   thrice.',
    'Quote "closed" here.',
    "It's fine.",
    'Wait... what?',
    'Hello!World',
    'For e.g. this.',
    'Mixed Case wORD.',
    '  Leading spaces.',
    'A {brace} and <angle> here.'
)
[System.IO.File]::WriteAllText($path, ($lines -join "`r`n"), (New-Object System.Text.UTF8Encoding $false))
Start-Process notepad.exe -ArgumentList "`"$path`""

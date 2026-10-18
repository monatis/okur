<#
.SYNOPSIS
    Deterministic property sheet fixture: a text file's Properties.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). Writes a text
    file with fixed content and dates to the user's temporary folder and opens its Properties,
    a Win32 property sheet (#32770) whose pages are dialog boxes inside it. The sheet runs in
    this process, which stays until ExitAfterSeconds; the window is titled
    "SR Property Fixture.txt Properties". Do not change it without re-recording every scenario
    that uses it (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

$folder = Join-Path $env:TEMP 'sr-property-fixture'
New-Item -ItemType Directory -Force -Path $folder | Out-Null
$file = Join-Path $folder 'SR Property Fixture.txt'
Set-Content -Path $file -Value 'A file whose Properties the fixture opens.' -Encoding ascii
$date = Get-Date -Year 2026 -Month 1 -Day 2 -Hour 3 -Minute 4 -Second 5 -Millisecond 0
$item = Get-Item $file
$item.CreationTime = $date
$item.LastWriteTime = $date
$item.LastAccessTime = $date

$shell = New-Object -ComObject Shell.Application
$shell.NameSpace($folder).ParseName('SR Property Fixture.txt').InvokeVerb('Properties')
Start-Sleep -Seconds $ExitAfterSeconds

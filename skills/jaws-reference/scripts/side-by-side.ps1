<#
.SYNOPSIS
    Research, never evidence: what JAWS and Okur each say, key by key, in an application on
    old-pc, side by side.

.DESCRIPTION
    JAWS keeps running; Okur runs beside it without speech and with ScrollLock as its key, so
    navigation keys reach both and each says its own (an Insert chord reaches JAWS alone). After
    each key: JAWS's speech history (okur-lab jaws-history), then Okur's utterances since the
    key (its speech log). Found Okur silent in all of Windows Settings in one run (2026-11-02).
    Output, under C:\Users\ailabs\sr-harness\explore\<Name>-<time>\: steps.jsonl (key, jaws,
    okur), Okur's speech.jsonl and debug.log, and with -Outline the window's UI Automation
    outline (uia-outline.ps1). The outline and the debug log can hold the lab machine's own
    data (its installed programs in Settings): keep them out of the repository.
      -Open     what to start first: a URI (ms-settings:appsfeatures) or a program, optional
      -Allow    processes allowed in front for the keys (comma-separated, without .exe)
      -Steps    keys separated by ';' ("Tab;Down;wait:500")
      -NoOkur   JAWS alone
      -Outline  write the UI Automation outline of the window in front at the end
      -Bin      where okur.exe and okur-lab.exe are (another build than the lab's)
.EXAMPLE
    side-by-side.ps1 -Name apps -Open ms-settings:appsfeatures -OpenWaitMs 8000 -Steps 'Tab;Tab;Down'
#>
param([string]$Open = '', [string]$Allow = 'ApplicationFrameHost,SystemSettings', [Parameter(Mandatory)][string]$Steps,
      [string]$Name = 'side-by-side', [switch]$NoOkur, [switch]$Outline, [int]$OpenWaitMs = 6000, [switch]$KeepOkur,
      [string]$Bin = 'C:\Users\ailabs\sr-harness\bin')
$ErrorActionPreference = 'Continue'
$lab = Join-Path $Bin 'okur-lab.exe'
$okur = Join-Path $Bin 'okur.exe'
$out = "C:\Users\ailabs\sr-harness\explore\$Name-$(Get-Date -Format 'yyyyMMdd-HHmmss')"
New-Item -ItemType Directory -Force $out | Out-Null

if ($Open) {
    Start-Process $Open
    Start-Sleep -Milliseconds $OpenWaitMs
}
if (-not $NoOkur) {
    & $okur quit 2>&1 | Out-Null
    Get-Process okur, okur-ui -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    $settings = Join-Path $out 'settings'
    New-Item -ItemType Directory -Force $settings | Out-Null
    Start-Process -FilePath $okur -WindowStyle Hidden -ArgumentList @('run', '--engine', 'no-speech', '--okur-key', 'scrolllock',
        '--settings', "`"$settings`"", '--speech-log', "`"$out\speech.jsonl`"", '--debug-log', "`"$out\debug.log`"")
    Start-Sleep -Seconds 4
}
& $lab wait-silence --onset-ms 300 --quiet-ms 500 --max-ms 8000 2>&1 | Out-Null
[void](& $lab jaws-history --clear 2>&1)
$read = 0
# Okur's utterances logged since the last call.
function OkurSince {
    $f = Join-Path $out 'speech.jsonl'
    if (-not (Test-Path $f)) { return @() }
    $all = @(Get-Content -Encoding UTF8 $f)
    $new = $all | Select-Object -Skip $script:read
    $script:read = $all.Count
    @($new | ForEach-Object { try { ($_ | ConvertFrom-Json).text } catch { $_ } })
}
[void](OkurSince)
foreach ($s in ($Steps -split ';')) {
    if (-not $s) { continue }
    $r = & $lab keys --allow $Allow $s 2>&1 | Out-String
    if ($s -notlike 'wait:*') { & $lab wait-silence --onset-ms 1200 --quiet-ms 600 --max-ms 10000 2>&1 | Out-Null }
    Start-Sleep -Milliseconds 300
    $jaws = (& $lab jaws-history --clear 2>&1 | Out-String).Trim() -replace "`r?`n", ' | '
    $ours = (OkurSince) -join ' | '
    $line = [ordered]@{ key = $s; jaws = $jaws; okur = $ours }
    if ($r -match 'refus|error') { $line.sent = $r.Trim() }
    Add-Content -Path (Join-Path $out 'steps.jsonl') -Value ($line | ConvertTo-Json -Compress) -Encoding utf8
    "{0,-14} JAWS: {1}" -f $s, $jaws
    if (-not $NoOkur) { "{0,-14} OKUR: {1}" -f '', $ours }
    if ($line.sent) { "   keys: $($line.sent)" }
}
if ($Outline) {
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'uia-outline.ps1') -Out (Join-Path $out 'outline.txt') -MaxNodes 3000
}
if (-not $NoOkur -and -not $KeepOkur) {
    & $okur quit 2>&1 | Out-Null
    Start-Sleep -Seconds 1
    Get-Process okur, okur-ui -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
}
"out: $out"

<#
.SYNOPSIS
    Runs many scenarios unattended on the reference machine, serially, with a progress log.

.DESCRIPTION
    For each scenario file (sorted) matching ScenarioGlob, one wildcard path or several separated
    by commas (a batch re-recording chosen scenarios), runs run-scenario.ps1 and writes
    <OutputDir>\<scenario-id>-evidence.json. Progress lines go to <OutputDir>\batch.log, and
    <OutputDir>\batch.done is written last, so a remote caller can launch this detached
    (Start-Process) and poll for completion instead of holding a long-lived connection.

.EXAMPLE
    Start-Process powershell -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass',
      '-File','C:\Users\ailabs\skills\jaws-reference\scripts\run-batch.ps1',
      '-ScenarioGlob','C:\Users\ailabs\corpus\scenarios\1*.json',
      '-OutputDir','C:\Users\ailabs\Dropbox\sr-harness\evidence-out'
#>
param(
    [Parameter(Mandatory = $true)][string]$ScenarioGlob,
    [Parameter(Mandatory = $true)][string]$OutputDir,
    [int]$Repetitions = 5,
    # Record an accessibility event trace per trial into OutputDir/traces.
    [switch]$Trace,
    # With Trace: the tracer's timings too (run-scenario.ps1 -TraceTimings).
    [switch]$TraceTimings,
    # Restart JAWS before the first scenario (jaws-process.ps1, elevated; the lab only): JAWS
    # keeps state between scenarios that keys left wrong can spoil. After a day of stuck keys
    # JAWS stopped toggling Caps Lock on CapsLock pressed twice until it was restarted (309,
    # old-pc, 2026-10-18).
    [switch]$FreshJaws
)
$ErrorActionPreference = 'Continue'
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$log = Join-Path $OutputDir 'batch.log'
$done = Join-Path $OutputDir 'batch.done'
Remove-Item $done -ErrorAction SilentlyContinue
$runner = Join-Path $PSScriptRoot 'run-scenario.ps1'

$helper = Join-Path $PSScriptRoot 'jaws-process.ps1'
$settingsTool = Join-Path $PSScriptRoot 'jaws-settings.ps1'

# Stops JAWS (killed: it writes no settings on the way out), runs $between (settings changes),
# and starts JAWS again, waiting until it has loaded its settings and spoken.
function Restart-Jaws([scriptblock]$between) {
    $p = Start-Process powershell -Verb RunAs -WindowStyle Hidden -PassThru -ArgumentList @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$helper`"", '-Stop')
    if ($p) { [void]$p.WaitForExit(60000) }
    Start-Sleep -Seconds 3
    if ($between) { & $between }
    $p = Start-Process powershell -Verb RunAs -WindowStyle Hidden -PassThru -ArgumentList @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$helper`"", '-Start')
    if ($p) { [void]$p.WaitForExit(60000) }
    Start-Sleep -Seconds 3
    # JAWS takes a while to load its settings and start speaking.
    for ($i = 0; $i -lt 30 -and -not (Get-Process jfw -ErrorAction SilentlyContinue); $i++) {
        Start-Sleep -Seconds 1
    }
    Start-Sleep -Seconds 15
}

# A scenario's "jaws_settings" as one comparable string ('' for none).
function Get-SettingsKey($file) {
    $scenario = Get-Content -LiteralPath $file -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($scenario.PSObject.Properties.Name -notcontains 'jaws_settings' -or $null -eq $scenario.jaws_settings) { return '' }
    return ($scenario.jaws_settings | ConvertTo-Json -Depth 6 -Compress)
}

# Settings a stopped batch left applied are put back first.
if (Test-Path (Join-Path $env:USERPROFILE 'sr-harness\settings-backup\manifest.json')) {
    Restart-Jaws { & powershell -NoProfile -ExecutionPolicy Bypass -File $settingsTool -Restore | Out-File $log -Append -Encoding utf8 }
} elseif ($FreshJaws) { Restart-Jaws $null }

$files = @(Get-ChildItem ($ScenarioGlob -split ',') | Sort-Object Name -Unique)
"=== batch started $(Get-Date -Format o): $($files.Count) scenario(s), $Repetitions trial(s) each" | Out-File $log -Encoding utf8
$i = 0
$applied = ''
foreach ($f in $files) {
    $i++
    $id = [System.IO.Path]::GetFileNameWithoutExtension($f.Name)
    $out = Join-Path $OutputDir "$id-evidence.json"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    # A scenario that names JAWS settings runs with them in JAWS's files; the next scenario
    # without them runs with the files put back. JAWS restarts only when they change.
    $wanted = Get-SettingsKey $f.FullName
    if ($wanted -ne $applied) {
        $scenarioFile = $f.FullName
        Restart-Jaws {
            if ($wanted) { & powershell -NoProfile -ExecutionPolicy Bypass -File $settingsTool -Apply $scenarioFile | Out-File $log -Append -Encoding utf8 }
            else { & powershell -NoProfile -ExecutionPolicy Bypass -File $settingsTool -Restore | Out-File $log -Append -Encoding utf8 }
        }
        $applied = $wanted
    }
    $runnerArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $runner, '-ScenarioFile', $f.FullName, '-Repetitions', $Repetitions, '-OutputPath', $out, '-Quiet')
    if ($Trace) { $runnerArgs += @('-TraceDir', (Join-Path $OutputDir 'traces')) }
    if ($Trace -and $TraceTimings) { $runnerArgs += '-TraceTimings' }
    # What the run said, kept when it failed: round 53's eight failures told nothing else.
    $said = & powershell @runnerArgs 2>&1
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        $said | Select-Object -Last 40 | Out-File (Join-Path $OutputDir "$id-run.log") -Encoding utf8
    }
    "[$i/$($files.Count)] $id exit=$code $([int]$sw.Elapsed.TotalSeconds)s" | Out-File $log -Append -Encoding utf8
}
# Old-pc's own settings back for whatever runs next.
if ($applied) {
    Restart-Jaws { & powershell -NoProfile -ExecutionPolicy Bypass -File $settingsTool -Restore | Out-File $log -Append -Encoding utf8 }
}
"=== batch finished $(Get-Date -Format o)" | Out-File $log -Append -Encoding utf8
Get-Date -Format o | Out-File $done -Encoding utf8

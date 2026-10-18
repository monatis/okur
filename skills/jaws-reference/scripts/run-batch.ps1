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

if ($FreshJaws) {
    $helper = Join-Path $PSScriptRoot 'jaws-process.ps1'
    foreach ($action in '-Stop', '-Start') {
        $p = Start-Process powershell -Verb RunAs -WindowStyle Hidden -PassThru -ArgumentList @(
            '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$helper`"", $action)
        if ($p) { [void]$p.WaitForExit(60000) }
        Start-Sleep -Seconds 3
    }
    # JAWS takes a while to load its settings and start speaking.
    for ($i = 0; $i -lt 30 -and -not (Get-Process jfw -ErrorAction SilentlyContinue); $i++) {
        Start-Sleep -Seconds 1
    }
    Start-Sleep -Seconds 15
}

$files = @(Get-ChildItem ($ScenarioGlob -split ',') | Sort-Object Name -Unique)
"=== batch started $(Get-Date -Format o): $($files.Count) scenario(s), $Repetitions trial(s) each" | Out-File $log -Encoding utf8
$i = 0
foreach ($f in $files) {
    $i++
    $id = [System.IO.Path]::GetFileNameWithoutExtension($f.Name)
    $out = Join-Path $OutputDir "$id-evidence.json"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $runnerArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $runner, '-ScenarioFile', $f.FullName, '-Repetitions', $Repetitions, '-OutputPath', $out, '-Quiet')
    if ($Trace) { $runnerArgs += @('-TraceDir', (Join-Path $OutputDir 'traces')) }
    & powershell @runnerArgs
    $code = $LASTEXITCODE
    "[$i/$($files.Count)] $id exit=$code $([int]$sw.Elapsed.TotalSeconds)s" | Out-File $log -Append -Encoding utf8
}
"=== batch finished $(Get-Date -Format o)" | Out-File $log -Append -Encoding utf8
Get-Date -Format o | Out-File $done -Encoding utf8

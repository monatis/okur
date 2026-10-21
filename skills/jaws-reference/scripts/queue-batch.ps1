<#
.SYNOPSIS
    Starts a batch once the batch before it has finished, so the reference machine records the
    next scenarios while the last ones are being implemented.

.DESCRIPTION
    Waits until <After>\batch.done exists, then runs run-batch.ps1 with the given scenarios and
    output directory. Batches must never run at the same time (JAWS, the focus and the
    clipboard are shared), so a queued batch only ever follows one. Writes <OutputDir>\queued.log
    while it waits; run-batch.ps1 writes batch.log and batch.done as usual.

.EXAMPLE
    Start-Process powershell -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass',
      '-File','C:\Users\ailabs\skills\jaws-reference\scripts\queue-batch.ps1',
      '-After','C:\Users\ailabs\sr-harness\batches\round3',
      '-ScenarioGlob','C:\Users\ailabs\corpus\scenarios\278-*.json',
      '-OutputDir','C:\Users\ailabs\sr-harness\batches\round4','-Repetitions','3','-Trace','-FreshJaws'
#>
param(
    [Parameter(Mandatory = $true)][string]$After,
    [Parameter(Mandatory = $true)][string]$ScenarioGlob,
    [Parameter(Mandatory = $true)][string]$OutputDir,
    [int]$Repetitions = 5,
    [switch]$Trace,
    [switch]$TraceTimings,
    [switch]$FreshJaws,
    # Fixtures (file names under corpus\fixtures) whose IA2 and UIA snapshots are captured into
    # <OutputDir>\snapshots before the batch: a capture must not run during another batch.
    # Several are named with commas ("a.html,b.html"): -File passes them as one string.
    [string[]]$Snapshots = @(),
    # Harness scripts (and okur.exe, okur-lab.exe) copied into place once the batch before has finished, so a
    # changed capture script reaches only the batches queued after the change (each batch's
    # evidence then names one capture script hash). Scenarios (*.json) in it go to
    # corpus\scenarios the same way, so a scenario the batch before also runs changes only
    # after it.
    [string]$StagedScripts,
    # A harness script run once the staged files are in place and before the batch, outside
    # any trial (a setup the scenarios need that is not a fixture of its own).
    [string]$Before,
    # Give up waiting after this long.
    [int]$MaxWaitMinutes = 360
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$log = Join-Path $OutputDir 'queued.log'
$previous = Join-Path $After 'batch.done'
"queued $(Get-Date -Format o): waiting for $previous" | Out-File $log -Encoding utf8
$deadline = (Get-Date).AddMinutes($MaxWaitMinutes)
while (-not (Test-Path $previous)) {
    if ((Get-Date) -gt $deadline) {
        "gave up $(Get-Date -Format o): $previous never appeared" | Out-File $log -Append -Encoding utf8
        exit 1
    }
    Start-Sleep -Seconds 10
}
"starting $(Get-Date -Format o)" | Out-File $log -Append -Encoding utf8
if ($StagedScripts) {
    foreach ($script in Get-ChildItem -LiteralPath $StagedScripts -Filter *.ps1) {
        Copy-Item -LiteralPath $script.FullName -Destination (Join-Path $PSScriptRoot $script.Name) -Force
        "installed $($script.Name) $((Get-FileHash -LiteralPath $script.FullName).Hash.ToLower())" | Out-File $log -Append -Encoding utf8
    }
    # Staged programs (the screen reader, okur.exe; the harness's tracer, okur-lab.exe) are
    # installed the same way: replacing one while a batch traces would fail, the file being in
    # use.
    foreach ($program in 'okur.exe', 'okur-lab.exe') {
        $binary = Join-Path $StagedScripts $program
        if (Test-Path -LiteralPath $binary) {
            Copy-Item -LiteralPath $binary -Destination (Join-Path 'C:\Users\ailabs\sr-harness\bin' $program) -Force
            "installed $program $((Get-FileHash -LiteralPath $binary).Hash.ToLower())" | Out-File $log -Append -Encoding utf8
        }
    }
    foreach ($scenario in Get-ChildItem -LiteralPath $StagedScripts -Filter *.json) {
        Copy-Item -LiteralPath $scenario.FullName -Destination (Join-Path 'C:\Users\ailabs\corpus\scenarios' $scenario.Name) -Force
        "installed $($scenario.Name) $((Get-FileHash -LiteralPath $scenario.FullName).Hash.ToLower())" | Out-File $log -Append -Encoding utf8
    }
}
if ($Before) {
    $said = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot $Before) 2>&1 |
        Select-Object -Last 1
    "before the batch, ${Before}: $said" | Out-File $log -Append -Encoding utf8
}
$Snapshots = @($Snapshots | ForEach-Object { $_ -split ',' } | Where-Object { $_ })
if ($Snapshots.Count -gt 0) {
    $capture = Join-Path $PSScriptRoot 'capture-snapshot.ps1'
    $okurLab = 'C:\Users\ailabs\sr-harness\bin\okur-lab.exe'
    $folder = Join-Path $OutputDir 'snapshots'
    New-Item -ItemType Directory -Force -Path $folder | Out-Null
    foreach ($fixture in $Snapshots) {
        $stem = [IO.Path]::GetFileNameWithoutExtension($fixture)
        foreach ($api in 'ia2', 'uia') {
            $out = Join-Path $folder "$stem.chrome.$api.json"
            # A failed capture is logged and the batch still runs: with 'Stop', the capture's
            # error output once ended this script before its batch, and every batch queued
            # after it waited for a batch.done that never came (round 83, 2026-10-20).
            try {
                $said = & powershell -NoProfile -ExecutionPolicy Bypass -File $capture `
                    -Fixture (Join-Path 'C:\Users\ailabs\corpus\fixtures' $fixture) `
                    -OutputPath $out -OkurLab $okurLab -Api $api 2>&1 | Select-Object -Last 1
            } catch {
                $said = "failed: $($_.Exception.Message)"
            }
            "snapshot ${stem} ${api}: ${said}" | Out-File $log -Append -Encoding utf8
        }
    }
}
$batch = @{
    ScenarioGlob = $ScenarioGlob
    OutputDir    = $OutputDir
    Repetitions  = $Repetitions
}
if ($Trace) { $batch.Trace = $true }
if ($TraceTimings) { $batch.TraceTimings = $true }
if ($FreshJaws) { $batch.FreshJaws = $true }
& (Join-Path $PSScriptRoot 'run-batch.ps1') @batch

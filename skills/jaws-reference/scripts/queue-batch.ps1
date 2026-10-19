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
    [switch]$FreshJaws,
    # Fixtures (file names under corpus\fixtures) whose IA2 and UIA snapshots are captured into
    # <OutputDir>\snapshots before the batch: a capture must not run during another batch.
    [string[]]$Snapshots = @(),
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
if ($Snapshots.Count -gt 0) {
    $capture = Join-Path $PSScriptRoot 'capture-snapshot.ps1'
    $okur = 'C:\Users\ailabs\sr-harness\bin\okur.exe'
    $folder = Join-Path $OutputDir 'snapshots'
    New-Item -ItemType Directory -Force -Path $folder | Out-Null
    foreach ($fixture in $Snapshots) {
        $stem = [IO.Path]::GetFileNameWithoutExtension($fixture)
        foreach ($api in 'ia2', 'uia') {
            $out = Join-Path $folder "$stem.chrome.$api.json"
            $said = & powershell -NoProfile -ExecutionPolicy Bypass -File $capture `
                -Fixture (Join-Path 'C:\Users\ailabs\corpus\fixtures' $fixture) `
                -OutputPath $out -Okur $okur -Api $api 2>&1 | Select-Object -Last 1
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
if ($FreshJaws) { $batch.FreshJaws = $true }
& (Join-Path $PSScriptRoot 'run-batch.ps1') @batch

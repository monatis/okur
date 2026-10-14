<#
.SYNOPSIS
    Stages harness files for the reference machine (old-pc) in the shared Dropbox, with a
    SHA-256 manifest that lab-pull.ps1 verifies before installing anything.

.DESCRIPTION
    Runs on the development machine from the repository root. Stages:
      bin/sr-cli.exe                       <- -SrCli (default target/debug/sr-cli.exe), if built
      scripts/*.ps1                        <- skills/jaws-reference/scripts/*.ps1
      fixtures/*.html                      <- corpus/fixtures/*.html
      scenarios/*.json                     <- corpus/scenarios/*.json
      recorder/sr_sapi_recorder64.dll      <- -Recorder (x64 build of crates/sr-sapi-recorder), if given
      recorder/sr_sapi_recorder32.dll      <- -Recorder32 (i686 build), if given
    then writes manifest.json last, so a partially synced folder never verifies.
#>
param(
    [string]$Dropbox = (Join-Path $env:USERPROFILE "Dropbox\sr-harness"),
    [string]$RepoRoot = (Get-Location).Path,
    # The sr-cli build to stage; use another target dir while target\debug is locked.
    [string]$SrCli = (Join-Path $RepoRoot 'target\debug\sr-cli.exe'),
    # The recording SAPI voice (crates/sr-sapi-recorder), 64- and 32-bit builds. JAWS must not
    # have the voice loaded when lab-pull installs it.
    [string]$Recorder,
    [string]$Recorder32
)
$ErrorActionPreference = 'Stop'

$sets = @(
    @{ dest = 'bin';       files = @(Get-Item $SrCli -ErrorAction SilentlyContinue) },
    @{ dest = 'scripts';   files = @(Get-ChildItem (Join-Path $RepoRoot 'skills\jaws-reference\scripts\*.ps1')) },
    @{ dest = 'fixtures';  files = @(Get-ChildItem (Join-Path $RepoRoot 'corpus\fixtures\*') -Include *.html, *.ps1) },
    @{ dest = 'scenarios'; files = @(Get-ChildItem (Join-Path $RepoRoot 'corpus\scenarios\*.json')) }
)
# The two recorder builds share a file name, so they are staged under bitness-specific names.
$recorderStage = Join-Path $env:TEMP 'sr-recorder-stage'
Remove-Item $recorderStage -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $recorderStage | Out-Null
$recorderFiles = @()
foreach ($r in @(@($Recorder, 'sr_sapi_recorder64.dll'), @($Recorder32, 'sr_sapi_recorder32.dll'))) {
    if ($r[0]) {
        Copy-Item $r[0] (Join-Path $recorderStage $r[1]) -Force
        $recorderFiles += Get-Item (Join-Path $recorderStage $r[1])
    }
}
$sets += @{ dest = 'recorder'; files = $recorderFiles }

$manifestPath = Join-Path $Dropbox 'manifest.json'
Remove-Item $manifestPath -ErrorAction SilentlyContinue
$manifest = [ordered]@{}
foreach ($set in $sets) {
    $dir = Join-Path $Dropbox $set.dest
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    foreach ($f in $set.files | Where-Object { $_ }) {
        Copy-Item $f.FullName (Join-Path $dir $f.Name) -Force
        $manifest["$($set.dest)/$($f.Name)"] = (Get-FileHash -Algorithm SHA256 $f.FullName).Hash.ToLowerInvariant()
    }
}
$pushId = [guid]::NewGuid().ToString("N")
[ordered]@{ push_id = $pushId; files = $manifest } | ConvertTo-Json | Set-Content -Path $manifestPath -Encoding UTF8
Write-Output "Staged $($manifest.Count) files in $Dropbox"
Write-Output "PushId: $pushId   (on old-pc: lab-pull.ps1 -PushId $pushId)"

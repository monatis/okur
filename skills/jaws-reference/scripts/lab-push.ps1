<#
.SYNOPSIS
    Stages harness files for the reference machine (old-pc) in the shared Dropbox, with a
    SHA-256 manifest that lab-pull.ps1 verifies before installing anything.

.DESCRIPTION
    Runs on the development machine from the repository root. Stages:
      bin/sr-cli.exe                       <- target/debug/sr-cli.exe (if built)
      scripts/*.ps1                        <- skills/jaws-reference/scripts/*.ps1
      fixtures/*.html                      <- corpus/fixtures/*.html
      scenarios/*.json                     <- corpus/scenarios/*.json
    then writes manifest.json last, so a partially synced folder never verifies.
#>
param(
    [string]$Dropbox = (Join-Path $env:USERPROFILE "Dropbox\sr-harness"),
    [string]$RepoRoot = (Get-Location).Path
)
$ErrorActionPreference = 'Stop'

$sets = @(
    @{ dest = 'bin';       files = @(Get-Item (Join-Path $RepoRoot 'target\debug\sr-cli.exe') -ErrorAction SilentlyContinue) },
    @{ dest = 'scripts';   files = @(Get-ChildItem (Join-Path $RepoRoot 'skills\jaws-reference\scripts\*.ps1')) },
    @{ dest = 'fixtures';  files = @(Get-ChildItem (Join-Path $RepoRoot 'corpus\fixtures\*') -Include *.html, *.ps1) },
    @{ dest = 'scenarios'; files = @(Get-ChildItem (Join-Path $RepoRoot 'corpus\scenarios\*.json')) }
)

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

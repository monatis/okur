<#
.SYNOPSIS
    Installs harness files staged by lab-push.ps1, after verifying every SHA-256.

.DESCRIPTION
    Runs on the reference machine (old-pc). Waits (up to -TimeoutSec) for Dropbox to finish
    syncing, i.e. until manifest.json exists and every listed file matches its hash, then
    copies them into place:
      bin/*       -> C:\Users\ailabs\sr-harness\bin\
      scripts/*   -> C:\Users\ailabs\skills\jaws-reference\scripts\
      fixtures/*  -> C:\Users\ailabs\corpus\fixtures\
      scenarios/* -> C:\Users\ailabs\corpus\scenarios\
    Never runs anything from the Dropbox folder itself.
#>
param(
    [string]$Dropbox = (Join-Path $env:USERPROFILE "Dropbox\sr-harness"),
    [string]$LabRoot = $env:USERPROFILE,
    [int]$TimeoutSec = 180
)
$ErrorActionPreference = 'Stop'

$targets = @{
    'bin'       = Join-Path $LabRoot 'sr-harness\bin'
    'scripts'   = Join-Path $LabRoot 'skills\jaws-reference\scripts'
    'fixtures'  = Join-Path $LabRoot 'corpus\fixtures'
    'scenarios' = Join-Path $LabRoot 'corpus\scenarios'
}

$deadline = (Get-Date).AddSeconds($TimeoutSec)
$manifestPath = Join-Path $Dropbox 'manifest.json'
$pending = @('manifest.json')
while ((Get-Date) -lt $deadline) {
    $pending = @()
    if (-not (Test-Path $manifestPath)) { $pending = @('manifest.json') }
    else {
        $manifest = Get-Content -Raw $manifestPath | ConvertFrom-Json
        foreach ($p in $manifest.PSObject.Properties) {
            $src = Join-Path $Dropbox ($p.Name -replace '/', '\')
            if (-not (Test-Path $src) -or (Get-FileHash -Algorithm SHA256 $src).Hash.ToLowerInvariant() -ne $p.Value) {
                $pending += $p.Name
            }
        }
    }
    if ($pending.Count -eq 0) { break }
    Start-Sleep -Seconds 2
}
if ($pending.Count -gt 0) { throw "Dropbox not synced after $TimeoutSec s; pending: $($pending -join ', ')" }

$count = 0
foreach ($p in $manifest.PSObject.Properties) {
    $dest = $targets[$p.Name.Split('/')[0]]
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    Copy-Item (Join-Path $Dropbox ($p.Name -replace '/', '\')) $dest -Force
    $count++
}
Write-Output "Installed $count verified files."

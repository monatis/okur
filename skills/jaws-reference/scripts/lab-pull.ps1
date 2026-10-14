<#
.SYNOPSIS
    Installs harness files staged by lab-push.ps1, after verifying every SHA-256.

.DESCRIPTION
    Runs on the reference machine (old-pc). Waits (up to -TimeoutSec) for Dropbox to finish
    syncing, i.e. until manifest.json carries the expected -PushId (printed by lab-push.ps1)
    and every listed file matches its hash, then
    copies them into place:
      bin/*       -> C:\Users\ailabs\sr-harness\bin\
      scripts/*   -> C:\Users\ailabs\skills\jaws-reference\scripts\
      fixtures/*  -> C:\Users\ailabs\corpus\fixtures\
      scenarios/* -> C:\Users\ailabs\corpus\scenarios\
    Never runs anything from the Dropbox folder itself.
#>
param(
    # Push id printed by lab-push.ps1. Required: without it a stale manifest that still
    # matches stale files would "verify" and silently install an old version.
    [Parameter(Mandatory = $true)][string]$PushId,
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
$files = $null
while ((Get-Date) -lt $deadline) {
    $pending = @()
    $manifest = $null
    if (Test-Path $manifestPath) {
        try { $manifest = Get-Content -Raw $manifestPath | ConvertFrom-Json } catch { $manifest = $null }
    }
    if (-not $manifest -or $manifest.push_id -ne $PushId) {
        $pending = @("manifest.json (push $PushId)")
    } else {
        $files = $manifest.files
        foreach ($p in $files.PSObject.Properties) {
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
foreach ($p in $files.PSObject.Properties) {
    $dest = $targets[$p.Name.Split('/')[0]]
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    Copy-Item (Join-Path $Dropbox ($p.Name -replace '/', '\')) $dest -Force
    # Re-verify the installed copy, not just the Dropbox copy.
    $installed = Join-Path $dest (Split-Path -Leaf $p.Name)
    if ((Get-FileHash -Algorithm SHA256 $installed).Hash.ToLowerInvariant() -ne $p.Value) {
        throw "Installed copy of $($p.Name) does not match its hash."
    }
    $count++
}
Write-Output "Installed $count verified files from push $PushId."

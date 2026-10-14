<#
.SYNOPSIS
    Captures the raw UIA tree of a fixture page in Chrome, exactly as loaded for JAWS trials.

.DESCRIPTION
    Runs on the reference machine (old-pc). Opens the fixture with the same mechanism as
    run-scenario.ps1 (Process.Start chrome <file URL>), foregrounds Chrome, verifies the
    foreground window shows the fixture's <title>, then invokes `sr-cli snapshot`, which
    records the fixture SHA-256 and Chrome version in the snapshot's provenance block.
    Output snapshots must never be edited by hand.

.EXAMPLE
    .\capture-snapshot.ps1 -Fixture C:\Users\ailabs\corpus\fixtures\doc-reading.html `
        -OutputPath C:\Users\ailabs\Dropbox\sr-harness\snapshots\doc-reading.chrome.uia.json `
        -SrCli C:\Users\ailabs\sr-harness\bin\sr-cli.exe
#>
param(
    [Parameter(Mandatory = $true)][string]$Fixture,
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [Parameter(Mandatory = $true)][string]$SrCli,
    [int]$LoadWaitMs = 2500,
    [int]$SettleMs = 750
)

$ErrorActionPreference = 'Stop'

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class SnapshotWin32 {
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder s, int n);
}
"@

$fixturePath = (Resolve-Path $Fixture).Path
$html = Get-Content -Raw -Path $fixturePath
$expectedTitle = if ($html -match '(?is)<title>\s*(.*?)\s*</title>') { $Matches[1] } else { $null }

$chromeExe = @(
    "C:\Program Files\Google\Chrome\Application\chrome.exe",
    "C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
$chromeVersion = if ($chromeExe) { (Get-Item $chromeExe).VersionInfo.FileVersion } else { $null }

$url = "file:///" + ($fixturePath -replace '\\', '/')
[System.Diagnostics.Process]::Start("chrome", $url) | Out-Null
Start-Sleep -Milliseconds $LoadWaitMs

$proc = Get-Process chrome -ErrorAction SilentlyContinue |
    Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero } | Select-Object -First 1
if (-not $proc) { throw "No Chrome window found." }
[void][SnapshotWin32]::ShowWindow($proc.MainWindowHandle, 9)
[void][SnapshotWin32]::SetForegroundWindow($proc.MainWindowHandle)
Start-Sleep -Milliseconds $SettleMs

$sb = New-Object System.Text.StringBuilder 512
[void][SnapshotWin32]::GetWindowText([SnapshotWin32]::GetForegroundWindow(), $sb, 512)
$foreground = $sb.ToString()
if ($expectedTitle -and -not $foreground.StartsWith($expectedTitle)) {
    throw "Foreground window '$foreground' does not show fixture title '$expectedTitle'."
}

$outDir = Split-Path -Parent $OutputPath
if ($outDir -and -not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir -Force | Out-Null }

$cliArgs = @('snapshot', '--out', $OutputPath, '--fixture', $fixturePath)
if ($chromeVersion) { $cliArgs += @('--process-version', $chromeVersion) }
& $SrCli @cliArgs
if ($LASTEXITCODE -ne 0) { throw "sr-cli snapshot failed with exit code $LASTEXITCODE" }

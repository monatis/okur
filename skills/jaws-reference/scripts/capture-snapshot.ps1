<#
.SYNOPSIS
    Captures the raw UIA tree of a fixture page in Chrome, exactly as loaded for JAWS trials.

.DESCRIPTION
    Runs on the reference machine (old-pc). Opens the fixture with the same mechanism as
    run-scenario.ps1 (Process.Start chrome <file URL>), foregrounds Chrome, verifies the
    foreground window shows the fixture's <title>, then invokes `okur snapshot`, which
    records the fixture SHA-256 and Chrome version in the snapshot's provenance block.
    Output snapshots must never be edited by hand.

.EXAMPLE
    .\capture-snapshot.ps1 -Fixture C:\Users\ailabs\corpus\fixtures\doc-reading.html `
        -OutputPath C:\Users\ailabs\Dropbox\sr-harness\snapshots\doc-reading.chrome.uia.json `
        -OkurLab C:\Users\ailabs\sr-harness\bin\okur-lab.exe
#>
param(
    [Parameter(Mandatory = $true)][string]$Fixture,
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [string]$OkurLab = (Join-Path $env:USERPROFILE "sr-harness\bin\okur-lab.exe"),
    [ValidateSet("uia", "ia2")][string]$Api = "uia",
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
# UTF-8 without a byte order mark, as every fixture is: the system code page would garble a
# title such as "Türkçe Metin".
$html = Get-Content -Raw -Encoding UTF8 -Path $fixturePath
$expectedTitle = if ($html -match '(?is)<title>\s*(.*?)\s*</title>') { $Matches[1] } else { $null }

$chromeExe = @(
    "C:\Program Files\Google\Chrome\Application\chrome.exe",
    "C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
$chromeVersion = if ($chromeExe) { (Get-Item $chromeExe).VersionInfo.FileVersion } else { $null }

$url = "file:///" + ($fixturePath -replace '\\', '/')
. (Join-Path $PSScriptRoot 'lab-chrome.ps1')
[void](Start-LabChrome $url)
Start-Sleep -Milliseconds $LoadWaitMs

$proc = Get-LabChromeWindow
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

$cliArgs = @('snapshot', '--api', $Api, '--out', $OutputPath, '--fixture', $fixturePath)
if ($chromeVersion) { $cliArgs += @('--process-version', $chromeVersion) }
# A Chrome started a moment ago can still be loading the page: its document is then one busy
# node (round 135's custom-label capture), or the capture fails. It is captured again, after a
# wait, up to five times.
for ($attempt = 1; $attempt -le 5; $attempt++) {
    & $OkurLab @cliArgs
    $captured = $LASTEXITCODE -eq 0
    $nodes = 0
    if ($captured) {
        try {
            $nodes = [int]((Get-Content -Raw -Encoding UTF8 $OutputPath | ConvertFrom-Json).provenance.node_count)
        } catch { $nodes = 0 }
    }
    if ($captured -and $nodes -gt 2) { break }
    if ($attempt -eq 5) {
        if (-not $captured) { throw "okur snapshot failed with exit code $LASTEXITCODE" }
        throw "The page was still loading after $attempt captures ($nodes nodes)."
    }
    Start-Sleep -Milliseconds 2000
}

# Close the tab this capture opened (every capture opens a new one), but only while the
# fixture is still the foreground tab, so no other window receives Ctrl+W.
$sb = New-Object System.Text.StringBuilder 512
[void][SnapshotWin32]::GetWindowText([SnapshotWin32]::GetForegroundWindow(), $sb, 512)
if ($expectedTitle -and $sb.ToString().StartsWith($expectedTitle)) {
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.SendKeys]::SendWait("^w")
}

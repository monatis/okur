<#
.SYNOPSIS
    The browser the web fixtures open in: Chrome with a profile of its own under sr-harness.

.DESCRIPTION
    Dot-sourced by run-scenario.ps1 and capture-snapshot.ps1. The reference machine's own Chrome
    profile is named after its owner, and Chrome adds that name to every window title, which
    reached the traces and, where JAWS speaks the title, the evidence (2026-10-19). The lab's
    Chrome is started signed out with a data directory of its own, whose single profile adds no
    name to the title. Chrome windows of any other profile are closed before it starts, so none
    of them comes forward in a trial or reaches a trace.
#>

$LabChromeDir = Join-Path $env:USERPROFILE 'sr-harness\chrome-profile'
$LabChromeExe = @("C:\Program Files\Google\Chrome\Application\chrome.exe", "C:\Program Files (x86)\Google\Chrome\Application\chrome.exe") |
    Where-Object { Test-Path $_ } | Select-Object -First 1

# Chrome's browser processes: a chrome.exe without --type= (renderers, the GPU process and
# utilities carry one), with the command line it was started with.
function Get-ChromeBrowserProcesses {
    Get-CimInstance Win32_Process -Filter "Name='chrome.exe'" |
        Where-Object { $_.CommandLine -and $_.CommandLine -notmatch '--type=' }
}

function Test-LabChrome($process) {
    $process.CommandLine.IndexOf($LabChromeDir, [System.StringComparison]::OrdinalIgnoreCase) -ge 0
}

# Closes the browsers of every other profile; returns how many there were.
function Stop-OtherChrome {
    $closed = 0
    foreach ($p in Get-ChromeBrowserProcesses) {
        if (-not (Test-LabChrome $p)) {
            Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
            $closed++
        }
    }
    if ($closed) { Start-Sleep -Milliseconds 1000 }
    return $closed
}

# Opens a page in the lab's Chrome: a new tab when it runs, else a new browser.
function Start-LabChrome([string]$url) {
    $closed = Stop-OtherChrome
    New-Item -ItemType Directory -Force -Path $LabChromeDir | Out-Null
    $chromeArgs = @(
        "--user-data-dir=`"$LabChromeDir`"",
        '--no-first-run',
        '--no-default-browser-check',
        '--disable-search-engine-choice-screen',
        $url
    )
    Start-Process -FilePath $LabChromeExe -ArgumentList $chromeArgs | Out-Null
    return $closed
}

# The lab Chrome's browser process with a window, or $null.
function Get-LabChromeWindow {
    foreach ($p in Get-ChromeBrowserProcesses) {
        if (Test-LabChrome $p) {
            $proc = Get-Process -Id $p.ProcessId -ErrorAction SilentlyContinue
            if ($proc -and $proc.MainWindowHandle -ne [IntPtr]::Zero) { return $proc }
        }
    }
    return $null
}

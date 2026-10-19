<#
.SYNOPSIS
    Starts the lab's Chrome (lab-chrome.ps1) on a blank page before a batch of web scenarios and
    names the windows it opened.

.DESCRIPTION
    Run by queue-batch.ps1 -Before. A new profile's first start can open windows of its own (a
    welcome page, a privacy notice), which would take a trial's keys; the line this prints, kept
    in queued.log, names every window the lab's Chrome shows. The blank page stays open, so the
    scenarios after it open their fixtures as new tabs of a browser already running.
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lab-chrome.ps1')
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes

$closed = Start-LabChrome 'about:blank'
$window = $null
for ($i = 0; $i -lt 100 -and -not $window; $i++) {
    Start-Sleep -Milliseconds 200
    $window = Get-LabChromeWindow
}
if (-not $window) { "lab Chrome did not open a window (other browsers closed: $closed)"; exit 1 }
Start-Sleep -Seconds 5

$labIds = @(Get-CimInstance Win32_Process -Filter "Name='chrome.exe'" |
    Where-Object { $_.CommandLine -and (Test-LabChrome $_) } | ForEach-Object { [int]$_.ProcessId })
$root = [System.Windows.Automation.AutomationElement]::RootElement
$titles = foreach ($w in $root.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)) {
    if ($labIds -contains $w.Current.ProcessId) { "'$($w.Current.Name)'" }
}
"lab Chrome running (other browsers closed: $closed); its windows: $($titles -join ', ')"

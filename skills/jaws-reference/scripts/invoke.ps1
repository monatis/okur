[CmdletBinding()]
param(
    [string]$Script,
    [string]$Function,
    [string]$TargetProcess,
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"
$sw = [System.Diagnostics.Stopwatch]::StartNew()

$result = [ordered]@{
    Success = $false
    Timestamp = (Get-Date -Format "o")
    ActionType = $null
    CommandName = $null
    DispatchResult = $false
    TargetProcess = $TargetProcess
    ForegroundWindow = $null
    ElapsedMs = 0
    Error = $null
}

if (-not $Script -and -not $Function) {
    $result.Error = "Must specify either -Script or -Function."
    Write-Output ($result | ConvertTo-Json)
    exit 1
}

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class Win32Invoke {
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")]
    public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
}
"@ -ErrorAction SilentlyContinue

if ($TargetProcess) {
    $proc = Get-Process $TargetProcess -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero } | Select-Object -First 1
    if ($proc) {
        [Win32Invoke]::ShowWindow($proc.MainWindowHandle, 9)
        [Win32Invoke]::SetForegroundWindow($proc.MainWindowHandle)
        Start-Sleep -Milliseconds 150
    }
}

$fgHwnd = [Win32Invoke]::GetForegroundWindow()
$sb = New-Object System.Text.StringBuilder 256
[void][Win32Invoke]::GetWindowText($fgHwnd, $sb, 256)
$result.ForegroundWindow = $sb.ToString()

try {
    $jaws = New-Object -ComObject "FreedomSci.JawsApi"
    if ($Script) {
        $result.ActionType = "jaws_script"
        $result.CommandName = $Script
        $dispatch = $jaws.RunScript($Script)
        $result.DispatchResult = [bool]$dispatch
    } else {
        $result.ActionType = "jaws_function"
        $result.CommandName = $Function
        $dispatch = $jaws.RunFunction($Function)
        $result.DispatchResult = [bool]$dispatch
    }
    $result.Success = $result.DispatchResult
} catch {
    $result.Error = "COM dispatch error: $($_.Exception.Message)"
}

$sw.Stop()
$result.ElapsedMs = $sw.ElapsedMilliseconds

$jsonOut = $result | ConvertTo-Json -Depth 4
if (-not $Quiet) {
    Write-Output $jsonOut
}

if ($result.Success) {
    exit 0
} else {
    exit 1
}

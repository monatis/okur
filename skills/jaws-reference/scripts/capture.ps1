[CmdletBinding()]
param(
    [string]$Script,
    [string]$Function,
    [string]$Keystroke,
    [string]$TargetProcess,
    [int]$PreActionDelayMs = 120,
    [int]$SettleTimeoutMs = 400,
    [int]$Retries = 3,
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"
$sw = [System.Diagnostics.Stopwatch]::StartNew()

$result = [ordered]@{
    Success = $false
    Timestamp = (Get-Date -Format "o")
    ActionType = $null
    ActionCommand = $null
    InvocationMethod = $null
    TargetProcess = $TargetProcess
    ForegroundWindow = $null
    DispatchOk = $false
    RawTranscript = $null
    StaleDetected = $false
    ElapsedMs = 0
    Error = $null
}

if (-not $Script -and -not $Function -and -not $Keystroke) {
    $result.Error = "Must specify -Script, -Function, or -Keystroke."
    Write-Output ($result | ConvertTo-Json)
    exit 1
}

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class Win32Capture {
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

if ($TargetProcess -and $TargetProcess -ne "") {
    $proc = Get-Process $TargetProcess -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero } | Select-Object -First 1
    if ($proc) {
        [void][Win32Capture]::ShowWindow($proc.MainWindowHandle, 9)
        [void][Win32Capture]::SetForegroundWindow($proc.MainWindowHandle)
        Start-Sleep -Milliseconds 150
    }
}

$fgHwnd = [Win32Capture]::GetForegroundWindow()
$sb = New-Object System.Text.StringBuilder 256
[void][Win32Capture]::GetWindowText($fgHwnd, $sb, 256)
$result.ForegroundWindow = $sb.ToString()

$jaws = $null
try {
    $jaws = New-Object -ComObject "FreedomSci.JawsApi"
} catch {
    $result.Error = "Failed instantiating FreedomSci.JawsApi: $($_.Exception.Message)"
    Write-Output ($result | ConvertTo-Json)
    exit 1
}

[void]$jaws.RunScript("ClearSpeechHistory")
Start-Sleep -Milliseconds $PreActionDelayMs

$sentinel = "__JAWS_SENTINEL_$([guid]::NewGuid().ToString().Substring(0,8))__"
Set-Clipboard -Value $sentinel

if ($Script) {
    $result.ActionType = "jaws_script"
    $result.ActionCommand = $Script
    $result.InvocationMethod = "FSAPI_RunScript"
    $dispatch = $jaws.RunScript($Script)
    $result.DispatchOk = [bool]$dispatch
} elseif ($Function) {
    $result.ActionType = "jaws_function"
    $result.ActionCommand = $Function
    $result.InvocationMethod = "FSAPI_RunFunction"
    $dispatch = $jaws.RunFunction($Function)
    $result.DispatchOk = [bool]$dispatch
} elseif ($Keystroke) {
    $result.ActionType = "keyboard"
    $result.ActionCommand = $Keystroke
    $result.InvocationMethod = "SendWait"
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.SendKeys]::SendWait($Keystroke)
    $result.DispatchOk = $true
}

Start-Sleep -Milliseconds $SettleTimeoutMs
[void]$jaws.RunScript("CopySpeechHistoryToClipboard")
Start-Sleep -Milliseconds 200

$capturedText = $null
for ($i = 0; $i -lt $Retries; $i++) {
    try {
        $clip = Get-Clipboard -Raw -ErrorAction SilentlyContinue
        if ($clip -ne $null -and $clip -ne $sentinel) {
            $capturedText = $clip
            break
        }
    } catch {}
    Start-Sleep -Milliseconds 100
}

if ($capturedText -eq $sentinel) {
    $result.StaleDetected = $true
    $result.Error = "Clipboard unchanged; still contains sentinel. Speech history capture failed."
} elseif ($capturedText -ne $null) {
    $result.RawTranscript = $capturedText
    $result.Success = $true
} else {
    $result.Error = "Clipboard read returned empty after $Retries retries."
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

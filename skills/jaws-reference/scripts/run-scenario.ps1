[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [string]$ScenarioFile,
    [int]$Repetitions = 1,
    [string]$OutputPath,
    [string]$TargetProcess,
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"
$sw = [System.Diagnostics.Stopwatch]::StartNew()

if (-not (Test-Path $ScenarioFile)) {
    Write-Error "Scenario file not found: $ScenarioFile"
    exit 1
}

$scenario = Get-Content $ScenarioFile -Raw | ConvertFrom-Json
$scenarioId = if ($scenario.id) { $scenario.id } else { [System.IO.Path]::GetFileNameWithoutExtension($ScenarioFile) }
$targetProc = if ($TargetProcess) { $TargetProcess } elseif ($scenario.target_app) { $scenario.target_app } else { "chrome" }

$jfwExePath = "C:\Program Files\Freedom Scientific\JAWS\2025\jfw.exe"
$jawsVer = "Unknown"
if (Test-Path $jfwExePath) {
    $vi = (Get-Item $jfwExePath).VersionInfo
    $jawsVer = "$($vi.ProductVersion) (File: $($vi.FileVersion))"
}

if (-not ([System.Management.Automation.PSTypeName]"Win32Harness").Type) {
    Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class Win32Harness {
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, int dwExtraInfo);

    public static void SendKey(byte vk, byte scan) {
        keybd_event(vk, scan, 0, 0);
        System.Threading.Thread.Sleep(50);
        keybd_event(vk, scan, 2, 0);
    }
}
"@ -ErrorAction SilentlyContinue
}

function Focus-Target {
    param([string]$procName)
    $proc = Get-Process $procName -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero } | Select-Object -First 1
    if ($proc) {
        [void][Win32Harness]::ShowWindow($proc.MainWindowHandle, 9)
        [void][Win32Harness]::SetForegroundWindow($proc.MainWindowHandle)
        Start-Sleep -Milliseconds 250
    }
}

function Get-ForegroundTitle {
    $hwnd = [Win32Harness]::GetForegroundWindow()
    $sb = New-Object System.Text.StringBuilder 256
    [void][Win32Harness]::GetWindowText($hwnd, $sb, 256)
    return $sb.ToString()
}

function Reset-NavigationState {
    param(
        [string]$procName,
        [object]$resetConfig,
        [bool]$reloadPage = $true
    )
    Focus-Target -procName $procName
    [Win32Harness]::SendKey(0x1B, 0x01) # ESC
    Start-Sleep -Milliseconds 150
    [Win32Harness]::SendKey(0x1B, 0x01) # ESC
    Start-Sleep -Milliseconds 150

    if ($reloadPage) {
        [System.Windows.Forms.SendKeys]::SendWait("^{r}")
        Start-Sleep -Milliseconds 600
        [Win32Harness]::SendKey(0x1B, 0x01) # ESC
        Start-Sleep -Milliseconds 150
    }

    if ($resetConfig -and $resetConfig.keystrokes) {
        foreach ($k in $resetConfig.keystrokes) {
            [System.Windows.Forms.SendKeys]::SendWait($k)
            Start-Sleep -Milliseconds 150
        }
    } else {
        [System.Windows.Forms.SendKeys]::SendWait("^{HOME}")
        Start-Sleep -Milliseconds 300
    }
    [void]$jaws.RunScript("ClearSpeechHistory")
    Start-Sleep -Milliseconds 150
}

$evidence = [ordered]@{
    scenario_id = $scenarioId
    scenario_version = if ($scenario.version) { $scenario.version } else { "1.1.0" }
    timestamp = (Get-Date -Format "o")
    environment = [ordered]@{
        os_version = [System.Environment]::OSVersion.VersionString
        jaws_version = $jawsVer
        target_process = $targetProc
        interactive_session = ((Get-Process -Id $PID).SessionId -ne 0)
    }
    fixture = $scenario.fixture
    repetitions_requested = $Repetitions
    trials = @()
    summary = [ordered]@{
        total_trials = $Repetitions
        successful_trials = 0
        deterministic_match = $true
        stability = "deterministic"
        total_elapsed_ms = 0
    }
}

$jaws = New-Object -ComObject "FreedomSci.JawsApi"
Add-Type -AssemblyName System.Windows.Forms

$trialTranscripts = @()

if ($scenario.fixture) {
    [System.Diagnostics.Process]::Start($targetProc, $scenario.fixture) | Out-Null
    Start-Sleep -Milliseconds 1500
}

for ($t = 1; $t -le $Repetitions; $t++) {
    Reset-NavigationState -procName $targetProc -resetConfig $scenario.reset -reloadPage ($t -gt 1)
    
    $trialResult = [ordered]@{
        trial_index = $t
        timestamp = (Get-Date -Format "o")
        foreground_window_start = (Get-ForegroundTitle)
        step_results = @()
        trial_success = $true
    }
    
    $trialStepTexts = @()
    
    foreach ($step in $scenario.steps) {
        $stepId = $step.id
        $actionType = $step.type
        $command = $step.command
        $settleMs = if ($step.settle_ms) { [int]$step.settle_ms } else { 350 }
        
        [void]$jaws.RunScript("ClearSpeechHistory")
        Start-Sleep -Milliseconds 100
        
        $sentinel = "__SENTINEL_$([guid]::NewGuid().ToString().Substring(0,8))__"
        Set-Clipboard -Value $sentinel

        $dispatchOk = $false
        if ($actionType -eq "jaws_script") {
            $dispatchOk = [bool]$jaws.RunScript($command)
        } elseif ($actionType -eq "keyboard") {
            if ($command -eq "{SPACE}" -or $command -eq " ") {
                [Win32Harness]::SendKey(0x20, 0x39)
                $dispatchOk = $true
            } elseif ($command -eq "{ENTER}") {
                [Win32Harness]::SendKey(0x0D, 0x1C)
                $dispatchOk = $true
            } elseif ($command -eq "{ESC}") {
                [Win32Harness]::SendKey(0x1B, 0x01)
                $dispatchOk = $true
            } elseif ($command -eq "{TAB}") {
                [Win32Harness]::SendKey(0x09, 0x0F)
                $dispatchOk = $true
            } else {
                [System.Windows.Forms.SendKeys]::SendWait($command)
                $dispatchOk = $true
            }
        }
        
        Start-Sleep -Milliseconds $settleMs
        [void]$jaws.RunScript("CopySpeechHistoryToClipboard")
        Start-Sleep -Milliseconds 150
        
        $transcript = $null
        for ($retry = 0; $retry -lt 3; $retry++) {
            $clip = Get-Clipboard -Raw -ErrorAction SilentlyContinue
            if ($clip -ne $null -and $clip -ne $sentinel) {
                $transcript = $clip
                break
            }
            Start-Sleep -Milliseconds 100
        }

        # Quality Gate Check 1: Explicitly reject unknown/unhandled script calls
        if ($transcript -match "unknown script call" -or $transcript -match "unhandled script call") {
            $dispatchOk = $false
        }

        # Quality Gate Check 2: Semantic assertions evaluation
        $assertionPassed = $true
        if ($step.assertions) {
            if ($step.assertions.contains_speech) {
                foreach ($phrase in $step.assertions.contains_speech) {
                    if ($transcript -notmatch [regex]::Escape($phrase)) {
                        $assertionPassed = $false
                        break
                    }
                }
            }
        }
        
        $stepSuccess = ($dispatchOk -and ($transcript -ne $null) -and ($transcript -ne $sentinel) -and $assertionPassed)
        if (-not $stepSuccess) {
            $trialResult.trial_success = $false
        }
        
        $trialStepTexts += $transcript
        $trialResult.step_results += [ordered]@{
            step_id = $stepId
            action = [ordered]@{
                type = $actionType
                command = $command
            }
            observation = [ordered]@{
                speech_raw = $transcript
                dispatch_ok = $dispatchOk
            }
            assertions = $step.assertions
            assertions_passed = $assertionPassed
            step_success = $stepSuccess
        }
    }

    if ($trialResult.trial_success) {
        $evidence.summary.successful_trials++
    }
    
    $evidence.trials += $trialResult
    $trialTranscripts += ($trialStepTexts -join " ||| ")
}

if ($trialTranscripts.Count -gt 1) {
    $first = $trialTranscripts[0]
    for ($k = 1; $k -lt $trialTranscripts.Count; $k++) {
        if ($trialTranscripts[$k] -ne $first) {
            $evidence.summary.deterministic_match = $false
            $evidence.summary.stability = "variable"
            break
        }
    }
}

$sw.Stop()
$evidence.summary.total_elapsed_ms = $sw.ElapsedMilliseconds

$jsonEvidence = $evidence | ConvertTo-Json -Depth 6

if ($OutputPath) {
    $outDir = [System.IO.Path]::GetDirectoryName($OutputPath)
    if ($outDir -and -not (Test-Path $outDir)) {
        New-Item -ItemType Directory -Path $outDir -Force | Out-Null
    }
    [System.IO.File]::WriteAllText($OutputPath, $jsonEvidence, [System.Text.Encoding]::UTF8)
}

if (-not $Quiet) {
    Write-Output $jsonEvidence
}

if ($evidence.summary.successful_trials -eq $Repetitions) {
    exit 0
} else {
    exit 1
}

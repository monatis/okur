<#
.SYNOPSIS
    Registers (or removes) the harness's recording SAPI 5 voice, "SR Speech Recorder", on
    the reference machine. Run once, from an elevated PowerShell.

.DESCRIPTION
    JAWS's speech history holds text from before JAWS's symbol and punctuation processing,
    so it cannot show what the user hears ("$" instead of "dollar"). This voice
    (crates/sr-sapi-recorder) appends every text fragment it is asked to speak to a
    JSON-lines log, then lets a real voice (-Delegate, Microsoft David by default) speak it.
    run-scenario.ps1 reads the log per step into the evidence's "spoken" field.

    SAPI lists voices only from HKLM, and JAWS runs elevated (which ignores per-user COM
    registrations), so registration needs administrator rights. The DLLs stay in the lab
    folder lab-pull.ps1 installs them to, so updating the recorder later needs no
    elevation (stop JAWS first: it keeps the DLL loaded).

    Registers the 64-bit build for 64-bit SAPI and, when present, the 32-bit build for
    32-bit SAPI (JAWS hosts some synthesizers in its 32-bit fsSynth32 process).

.EXAMPLE
    # Elevated, on old-pc, after lab-pull.ps1 installed the recorder:
    powershell -ExecutionPolicy Bypass -File C:\Users\ailabs\skills\jaws-reference\scripts\install-speech-recorder.ps1
#>
#Requires -RunAsAdministrator
param(
    [string]$RecorderDir = 'C:\Users\ailabs\sr-harness\recorder',
    [string]$LogFile = 'C:\Users\ailabs\sr-harness\recorder\spoken.jsonl',
    [string]$Delegate = 'HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech\Voices\Tokens\TTS_MS_EN-US_DAVID_11.0',
    [switch]$Uninstall
)
$ErrorActionPreference = 'Stop'

$clsid = '{C7ABFA41-1B9F-4C8C-A9E6-56E2D350D16D}'
$token = 'SR_SpeechRecorder'
$name = 'SR Speech Recorder'
$views = @(
    @{ dll = 'sr_sapi_recorder64.dll'; classes = 'HKLM:\SOFTWARE\Classes\CLSID'; voices = 'HKLM:\SOFTWARE\Microsoft\Speech\Voices\Tokens' },
    @{ dll = 'sr_sapi_recorder32.dll'; classes = 'HKLM:\SOFTWARE\WOW6432Node\Classes\CLSID'; voices = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Speech\Voices\Tokens' }
)

foreach ($view in $views) {
    $classKey = Join-Path $view.classes $clsid
    $tokenKey = Join-Path $view.voices $token
    if ($Uninstall) {
        Remove-Item $classKey, $tokenKey -Recurse -Force -ErrorAction SilentlyContinue
        Write-Output "Removed $classKey and $tokenKey"
        continue
    }
    $dll = Join-Path $RecorderDir $view.dll
    if (-not (Test-Path $dll)) {
        Write-Output "Skipped $($view.dll): not found in $RecorderDir"
        continue
    }
    New-Item -Path (Join-Path $classKey 'InprocServer32') -Force | Out-Null
    Set-Item -Path $classKey -Value $name
    Set-Item -Path (Join-Path $classKey 'InprocServer32') -Value $dll
    Set-ItemProperty -Path (Join-Path $classKey 'InprocServer32') -Name ThreadingModel -Value 'Both'

    New-Item -Path (Join-Path $tokenKey 'Attributes') -Force | Out-Null
    Set-Item -Path $tokenKey -Value $name
    Set-ItemProperty -Path $tokenKey -Name '409' -Value $name
    Set-ItemProperty -Path $tokenKey -Name CLSID -Value $clsid
    Set-ItemProperty -Path $tokenKey -Name LogFile -Value $LogFile
    Set-ItemProperty -Path $tokenKey -Name Delegate -Value $Delegate
    $attributes = Join-Path $tokenKey 'Attributes'
    Set-ItemProperty -Path $attributes -Name Name -Value $name
    Set-ItemProperty -Path $attributes -Name Language -Value '409'
    Set-ItemProperty -Path $attributes -Name Gender -Value 'Male'
    Set-ItemProperty -Path $attributes -Name Age -Value 'Adult'
    Set-ItemProperty -Path $attributes -Name Vendor -Value 'sr-harness'
    Write-Output "Registered $name ($($view.dll)) in $($view.voices)"
}

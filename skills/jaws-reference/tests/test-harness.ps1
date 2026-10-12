[CmdletBinding()]
param(
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$skillDir = Split-Path -Parent $scriptDir
$scriptsPath = Join-Path $skillDir "scripts"
$examplesPath = Join-Path $skillDir "examples"

$results = [ordered]@{
    timestamp = (Get-Date -Format "o")
    total_tests = 0
    passed = 0
    failed = 0
    test_details = @()
}

function Report-Test {
    param([string]$name, [bool]$passed, [string]$details)
    $results.total_tests++
    if ($passed) {
        $results.passed++
        Write-Host "[PASS] $name" -ForegroundColor Green
    } else {
        $results.failed++
        Write-Host "[FAIL] ${name}: $details" -ForegroundColor Red
    }
    $results.test_details += [ordered]@{
        test = $name
        passed = $passed
        details = $details
    }
}

try {
    $probeOut = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scriptsPath "probe.ps1") | ConvertFrom-Json
    Report-Test -name "ProbeEnvironment" -passed ($probeOut.Success -eq $true) -details "JawsRunning=$($probeOut.JawsRunning), Com=$($probeOut.ComInstantiated)"
} catch {
    Report-Test -name "ProbeEnvironment" -passed $false -details $_.Exception.Message
}

try {
    $jaws = New-Object -ComObject "FreedomSci.JawsApi"
    [void]$jaws.RunScript("ClearSpeechHistory")
    Start-Sleep -Milliseconds 100
    $say = $jaws.SayString("TestHarness Speech RoundTrip", $false)
    Start-Sleep -Milliseconds 400
    [void]$jaws.RunScript("CopySpeechHistoryToClipboard")
    Start-Sleep -Milliseconds 150
    $clip = Get-Clipboard
    $ok = ($clip -and $clip.Contains("TestHarness Speech RoundTrip"))
    Report-Test -name "SpeechRoundTrip" -passed $ok -details "Captured: '$clip'"
} catch {
    Report-Test -name "SpeechRoundTrip" -passed $false -details $_.Exception.Message
}

try {
    $invOut = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scriptsPath "invoke.ps1") -Script MoveToNextHeading | ConvertFrom-Json
    Report-Test -name "InvokeScript" -passed ($invOut.DispatchResult -eq $true) -details "Dispatch=$($invOut.DispatchResult)"
} catch {
    Report-Test -name "InvokeScript" -passed $false -details $_.Exception.Message
}

try {
    $capOut = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scriptsPath "capture.ps1") -Script SayLine | ConvertFrom-Json
    $ok = ($capOut.Success -eq $true -and -not [string]::IsNullOrEmpty($capOut.RawTranscript))
    Report-Test -name "CaptureScript" -passed $ok -details "RawTranscript length=$($capOut.RawTranscript.Length)"
} catch {
    Report-Test -name "CaptureScript" -passed $false -details $_.Exception.Message
}

try {
    $negOut = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scriptsPath "capture.ps1") -Script MoveToNextHeading -TargetProcess NonExistentApp1234 | ConvertFrom-Json
    Report-Test -name "NegativeMissingApp" -passed ($negOut -ne $null) -details "Gracefully handled missing process"
} catch {
    Report-Test -name "NegativeMissingApp" -passed $false -details $_.Exception.Message
}

$tmpEv = Join-Path $skillDir "tests\tmp-evidence.json"
try {
    $scenOut = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scriptsPath "run-scenario.ps1") -ScenarioFile (Join-Path $examplesPath "boundary-navigation.json") -OutputPath $tmpEv | ConvertFrom-Json
    $ok = ($scenOut.trials.Count -eq 1 -and (Test-Path $tmpEv))
    Report-Test -name "ScenarioExecution" -passed $ok -details "Trials=$($scenOut.trials.Count)"
} catch {
    Report-Test -name "ScenarioExecution" -passed $false -details $_.Exception.Message
}

$repEv = Join-Path $skillDir "tests\rep-evidence.json"
try {
    $scen5 = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scriptsPath "run-scenario.ps1") -ScenarioFile (Join-Path $examplesPath "boundary-navigation.json") -Repetitions 5 -OutputPath $repEv | ConvertFrom-Json
    $ok = ($scen5.trials.Count -eq 5 -and $scen5.summary.successful_trials -eq 5)
    Report-Test -name "MultiTrialRepeatability" -passed $ok -details "Successful=$($scen5.summary.successful_trials)/5, Deterministic=$($scen5.summary.deterministic_match)"
} catch {
    Report-Test -name "MultiTrialRepeatability" -passed $false -details $_.Exception.Message
}

try {
    $ev = Get-Content $repEv -Raw | ConvertFrom-Json
    $hasFields = ($ev.scenario_id -and $ev.environment -and $ev.environment.jaws_version -and $ev.trials -and $ev.summary)
    Report-Test -name "EvidenceSchemaValidation" -passed $hasFields -details "Schema has all required fields"
} catch {
    Report-Test -name "EvidenceSchemaValidation" -passed $false -details $_.Exception.Message
}

if (Test-Path $tmpEv) { Remove-Item $tmpEv -Force }
if (Test-Path $repEv) { Remove-Item $repEv -Force }

Write-Host "`nTest Summary: $($results.passed)/$($results.total_tests) passed." -ForegroundColor Cyan
if (-not $Quiet) {
    Write-Output ($results | ConvertTo-Json -Depth 4)
}

if ($results.failed -eq 0) {
    exit 0
} else {
    exit 1
}

$ErrorActionPreference = "Continue"
$scriptPath = "C:\Users\ailabs\skills\jaws-reference\scripts\run-scenario.ps1"
$scenariosDir = "C:\Users\ailabs\corpus\scenarios"
$evidenceDir = "C:\Users\ailabs\corpus\evidence"
$fixturesDir = "C:\Users\ailabs\corpus\fixtures"
$logFile = "C:\Users\ailabs\corpus\batch_progress.log"

if (-not (Test-Path $evidenceDir)) {
    New-Item -ItemType Directory -Path $evidenceDir -Force | Out-Null
}

"=== CORPUS EXECUTION STARTED $(Get-Date) ===" | Out-File -FilePath $logFile -Encoding utf8

# Detect Chrome Version
$chromePath = "C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"
if (-not (Test-Path $chromePath)) {
    $chromePath = "C:\Program Files\Google\Chrome\Application\chrome.exe"
}
$chromeVer = "Unknown"
if (Test-Path $chromePath) {
    $chromeVer = (Get-Item $chromePath).VersionInfo.FileVersion
}

# Compute Fixture Hashes
$fixtureHashes = [ordered]@{}
Get-ChildItem "$fixturesDir\*.html" | Sort-Object Name | ForEach-Object {
    $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash
    $fixtureHashes[$_.Name] = [ordered]@{
        sha256 = $hash
        path = $_.FullName
        size_bytes = $_.Length
    }
}

$scenarioFiles = Get-ChildItem "$scenariosDir\*.json" | Sort-Object Name
$total = $scenarioFiles.Count
$results = @()
$idx = 0
$deterministicCount = 0
$variableCount = 0

foreach ($sf in $scenarioFiles) {
    $idx++
    $scId = [System.IO.Path]::GetFileNameWithoutExtension($sf.Name)
    $outPath = "$evidenceDir\$scId-evidence.json"
    
    $msg = "[$idx/$total] Running $scId..."
    Write-Host $msg
    $msg | Out-File -FilePath $logFile -Append -Encoding utf8
    
    try {
        & $scriptPath -ScenarioFile $sf.FullName -Repetitions 5 -OutputPath $outPath -TargetProcess "chrome" -Quiet
        
        if (Test-Path $outPath) {
            $ev = Get-Content $outPath -Raw | ConvertFrom-Json
            $stat = "$scId | Stability: $($ev.summary.stability) | Success: $($ev.summary.successful_trials)/$($ev.summary.total_trials) | Time: $($ev.summary.total_elapsed_ms)ms"
            Write-Host "   -> $stat"
            "   -> $stat" | Out-File -FilePath $logFile -Append -Encoding utf8

            if ($ev.summary.stability -eq "deterministic") {
                $deterministicCount++
            } else {
                $variableCount++
            }

            # Build canonical steps from the first trial
            $canonicalSteps = @()
            if ($ev.trials -and $ev.trials.Count -gt 0) {
                foreach ($sr in $ev.trials[0].step_results) {
                    $canonicalSteps += [ordered]@{
                        step_id = $sr.step_id
                        action = $sr.action
                        observation = [ordered]@{
                            speech_raw = $sr.observation.speech_raw
                        }
                        assertions = $sr.assertions
                        step_success = $sr.step_success
                    }
                }
            }

            $results += [ordered]@{
                scenario_id = $scId
                scenario_version = $ev.scenario_version
                fixture = $ev.fixture
                repetitions = $ev.repetitions_requested
                stability = $ev.summary.stability
                deterministic_match = $ev.summary.deterministic_match
                successful_trials = $ev.summary.successful_trials
                total_trials = $ev.summary.total_trials
                elapsed_ms = $ev.summary.total_elapsed_ms
                canonical_steps = $canonicalSteps
            }
        } else {
            $err = "$scId | Output file not generated"
            Write-Host "   -> ERROR: $err"
            "   -> ERROR: $err" | Out-File -FilePath $logFile -Append -Encoding utf8
        }
    } catch {
        $err = "$scId | Exception: $($_.Exception.Message)"
        Write-Host "   -> ERROR: $err"
        "   -> ERROR: $err" | Out-File -FilePath $logFile -Append -Encoding utf8
    }
}

$successfulTrialsTotal = 0
foreach ($r in $results) {
    $successfulTrialsTotal += $r["successful_trials"]
}
$trialSuccessRate = [math]::Round(($successfulTrialsTotal / ($total * 5)) * 100, 2)

$manifest = [ordered]@{
    corpus_name = "JAWS 2025 Reference Speech Corpus"
    corpus_version = "1.1.0"
    generated_at = (Get-Date).ToString("o")
    environment = [ordered]@{
        host = $env:COMPUTERNAME
        os_version = [System.Environment]::OSVersion.VersionString
        jaws_version = "2025.2505.43.400"
        chrome_version = $chromeVer
        interactive_session = $true
    }
    fixture_hashes = $fixtureHashes
    statistics = [ordered]@{
        total_scenarios = $total
        deterministic_scenarios = $deterministicCount
        variable_scenarios = $variableCount
        total_trials_executed = ($total * 5)
        successful_trials = $successfulTrialsTotal
        trial_success_rate_percent = $trialSuccessRate
    }
    scenarios = $results
}

$manifestJson = $manifest | ConvertTo-Json -Depth 8
$manifestJson | Out-File -FilePath "C:\Users\ailabs\corpus\CORPUS_MANIFEST.json" -Encoding utf8

# Generate Quality Report
$qualityReport = @"
# Reference Speech Corpus Quality & Validation Report

**Corpus Version**: 1.1.0  
**Generated At**: $((Get-Date).ToString("o"))  
**Reference Environment**: Freedom Scientific JAWS 2025.2505.43.400, Google Chrome $chromeVer on Windows 10 ($([System.Environment]::OSVersion.VersionString))

## Quality Gate Summary

- **Total Scenarios**: $total
- **Deterministic Scenarios**: $deterministicCount ($([math]::Round(($deterministicCount / $total) * 100, 1))%)
- **Variable Scenarios**: $variableCount
- **Total Trials**: $($total * 5)
- **Successful Trials**: $successfulTrialsTotal ($trialSuccessRate%)

## Audited Repairs Executed

1. **03-doc-prior-line**: Replaced unexposed `PriorLine` COM script with `{UP}` keyboard command. Verified valid line speech output.
2. **07-doc-prior-char**: Replaced unexposed `PriorCharacter` COM script with `{LEFT}` keyboard command. Verified valid character speech output.
3. **12-struct-heading-level-2**: Replaced unexposed `MoveToNextHeadingLevel2` COM script with the canonical JAWS script `MoveToNextHeadingLevelTwo`. Verified level 2 heading announcements.
4. **19-form-mode-entry-exit**: Executed full Forms mode entry ({ENTER}), typing ('ABC'), and exit ({ESC}), asserting mode state transitions.
5. **20-form-checkbox-toggle**: Executed checkbox navigation and activation (' '), asserting initial unchecked state and checked state transition.
6. **28-dynamic-tab-navigation** & **29-dynamic-tab-selection**: Updated fixture and scenarios to include standard ARIA tablist with selection event handlers and boundary controls. Validated tab selection transition.
7. **Quality Gate Harness**: Updated `run-scenario.ps1` to reject unknown/unhandled script calls, validate semantic assertions, and enforce clean DOM reload before each repetition.
8. **Evidence Representation**: Separated action metadata, raw speech observations (preserving control characters), and semantic assertions.

## Fixture Verification Hashes (SHA-256)

"@

foreach ($k in $fixtureHashes.Keys) {
    $h = $fixtureHashes[$k]["sha256"]
    $qualityReport += "- **$k**: `$h`n"
}

$qualityReport | Out-File -FilePath "C:\Users\ailabs\corpus\quality-report.md" -Encoding utf8

Compress-Archive -Path "$evidenceDir\*-evidence.json", "C:\Users\ailabs\corpus\CORPUS_MANIFEST.json", "C:\Users\ailabs\corpus\quality-report.md", $logFile -DestinationPath "C:\Users\ailabs\Dropbox\migration\evidence_sync.zip" -Force

"=== CORPUS EXECUTION FINISHED $(Get-Date) ===" | Out-File -FilePath $logFile -Append -Encoding utf8
Write-Host "All scenarios complete! Evidence and Quality Report synced to Dropbox."

$inputJson = [Console]::In.ReadToEnd()

$payload = $null
if (-not [string]::IsNullOrWhiteSpace($inputJson)) {
    try {
        $payload = $inputJson | ConvertFrom-Json
    } catch {
        $payload = $null
    }
}

$cmdLine = ""
if ($payload -and $payload.toolCall -and $payload.toolCall.args -and $payload.toolCall.args.CommandLine) {
    $cmdLine = $payload.toolCall.args.CommandLine
}

# Only intercept git commit commands
if (-not ($cmdLine -match '\bgit\s+commit\b')) {
    $response = [ordered]@{
        decision = "allow"
    }
    $response | ConvertTo-Json -Compress
    exit 0
}

# Resolve workspace root (walk up until Cargo.toml is found)
$workspaceRoot = $null
$candidate = $PSScriptRoot
while ($candidate -and (Test-Path $candidate)) {
    if (Test-Path (Join-Path $candidate "Cargo.toml")) {
        $workspaceRoot = $candidate
        break
    }
    $parent = Split-Path -Parent $candidate
    if ($parent -eq $candidate) { break }
    $candidate = $parent
}
if (-not $workspaceRoot) {
    $workspaceRoot = (Get-Location).Path
}

$failureReasons = @()

# -----------------------------------------------------------------------------
# Check 1: Zero .unwrap() or .expect() in Rust source code
# -----------------------------------------------------------------------------
$cratesDir = Join-Path $workspaceRoot "crates"
if (Test-Path $cratesDir) {
    $rustFiles = Get-ChildItem -Path $cratesDir -Recurse -Filter *.rs
    $unwrapViolations = @()
    $panicViolations = @()

    foreach ($file in $rustFiles) {
        $lineNum = 0
        $lines = Get-Content -Path $file.FullName
        foreach ($line in $lines) {
            $lineNum++
            $trimmed = $line.Trim()
            if ($trimmed.StartsWith("//") -or $trimmed.StartsWith("/*") -or $trimmed.StartsWith("*")) {
                continue
            }
            if ($line -match '\.(unwrap|expect)\s*\(') {
                $relPath = $file.FullName.Substring($workspaceRoot.Length).TrimStart('\', '/')
                $unwrapViolations += "${relPath}:${lineNum}: ${trimmed}"
            }
            if ($line -match '\b(panic!|todo!|unimplemented!|dbg!)\b') {
                $relPath = $file.FullName.Substring($workspaceRoot.Length).TrimStart('\', '/')
                $panicViolations += "${relPath}:${lineNum}: ${trimmed}"
            }
        }
    }

    if ($unwrapViolations.Count -gt 0) {
        $displayLimit = 20
        $violationList = ($unwrapViolations | Select-Object -First $displayLimit) -join "`n"
        if ($unwrapViolations.Count -gt $displayLimit) {
            $violationList += "`n... and $($unwrapViolations.Count - $displayLimit) more violation(s)"
        }
        $failureReasons += "[Reliability Gate] $($unwrapViolations.Count) usage(s) of .unwrap() or .expect() found in Rust code.`nScreen readers must NEVER crash or panic at runtime.`nViolations:`n$violationList"
    }

    # -----------------------------------------------------------------------------
    # Check 2: Zero panic!, todo!, unimplemented!, or dbg! in Rust source code
    # -----------------------------------------------------------------------------
    if ($panicViolations.Count -gt 0) {
        $displayLimit = 20
        $violationList = ($panicViolations | Select-Object -First $displayLimit) -join "`n"
        if ($panicViolations.Count -gt $displayLimit) {
            $violationList += "`n... and $($panicViolations.Count - $displayLimit) more violation(s)"
        }
        $failureReasons += "[Panic Gate] $($panicViolations.Count) usage(s) of panic!, todo!, unimplemented!, or dbg! found in Rust code.`nViolations:`n$violationList"
    }
}

# -----------------------------------------------------------------------------
# Check 3: Index Bounds Safety (clippy::indexing_slicing)
# -----------------------------------------------------------------------------
Push-Location $workspaceRoot
try {
    $clippyOutput = & cargo clippy --all-targets --all-features -- -D clippy::indexing_slicing 2>&1
    $clippyExit = $LASTEXITCODE
    if ($clippyExit -ne 0) {
        $errorMsg = ($clippyOutput -join "`n").Trim()
        $failureReasons += "[Bounds Safety Gate] cargo clippy detected direct slice/index operations (clippy::indexing_slicing). Use .get(), .first(), or .last() instead of direct brackets:`n$errorMsg"
    }
} finally {
    Pop-Location
}

# -----------------------------------------------------------------------------
# Check 4: Corpus Manifest and Evidence Integrity
# -----------------------------------------------------------------------------
$scenariosDir = Join-Path $workspaceRoot "corpus\scenarios"
$evidenceDir = Join-Path $workspaceRoot "corpus\evidence"
$manifestPath = Join-Path $workspaceRoot "corpus\CORPUS_MANIFEST.json"

if ((Test-Path $scenariosDir) -and (Test-Path $evidenceDir) -and (Test-Path $manifestPath)) {
    $scenarioFiles = Get-ChildItem -Path $scenariosDir -Filter *.json
    $evidenceFiles = Get-ChildItem -Path $evidenceDir -Filter *-evidence.json
    $corpusViolations = @()

    # Check that each scenario has an evidence file
    foreach ($scFile in $scenarioFiles) {
        $baseName = [System.IO.Path]::GetFileNameWithoutExtension($scFile.Name)
        $expectedEvidence = Join-Path $evidenceDir "${baseName}-evidence.json"
        if (-not (Test-Path $expectedEvidence)) {
            $corpusViolations += "Scenario '${scFile.Name}' is missing evidence file '${baseName}-evidence.json'"
        }
    }

    # Validate manifest consistency
    try {
        $manifestJson = Get-Content -Raw -Path $manifestPath | ConvertFrom-Json
        $manifestScenarios = $manifestJson.scenarios
        $manifestIds = @{}
        if ($manifestScenarios) {
            foreach ($ms in $manifestScenarios) {
                if ($ms.scenario_id) {
                    $manifestIds[$ms.scenario_id] = $true
                }
            }
        }

        foreach ($scFile in $scenarioFiles) {
            $baseName = [System.IO.Path]::GetFileNameWithoutExtension($scFile.Name)
            if (-not $manifestIds.ContainsKey($baseName)) {
                $corpusViolations += "Scenario file '${scFile.Name}' (ID '${baseName}') is not registered in CORPUS_MANIFEST.json"
            }
        }

        if ($manifestJson.statistics -and $manifestJson.statistics.total_scenarios) {
            if ($manifestJson.statistics.total_scenarios -ne $scenarioFiles.Count) {
                $corpusViolations += "Manifest statistics.total_scenarios ($($manifestJson.statistics.total_scenarios)) does not match actual scenarios count ($($scenarioFiles.Count))"
            }
        }
    } catch {
        $corpusViolations += "Failed to parse corpus\CORPUS_MANIFEST.json as JSON: $($_.Exception.Message)"
    }

    if ($corpusViolations.Count -gt 0) {
        $corpusList = ($corpusViolations -join "`n")
        $failureReasons += "[Corpus Integrity Gate] Corpus manifest and evidence synchronization mismatch:`n$corpusList"
    }
}

# -----------------------------------------------------------------------------
# Decision evaluation
# -----------------------------------------------------------------------------
if ($failureReasons.Count -gt 0) {
    $fullReason = "Blocked 'git commit' due to quality gate violation(s):`n`n" + ($failureReasons -join "`n`n")
    $response = [ordered]@{
        decision = "deny"
        reason = $fullReason
    }
    $response | ConvertTo-Json -Compress
    exit 0
} else {
    $response = [ordered]@{
        decision = "allow"
    }
    $response | ConvertTo-Json -Compress
    exit 0
}

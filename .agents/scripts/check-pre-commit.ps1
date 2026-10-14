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
    $clippyOutput = & cargo clippy --all-targets --all-features -- -D warnings -D clippy::indexing_slicing 2>&1
    $clippyExit = $LASTEXITCODE
    if ($clippyExit -ne 0) {
        $errorMsg = ($clippyOutput -join "`n").Trim()
        $failureReasons += "[Bounds Safety Gate] cargo clippy detected direct slice/index operations (clippy::indexing_slicing). Use .get(), .first(), or .last() instead of direct brackets:`n$errorMsg"
    }
} finally {
    Pop-Location
}

# -----------------------------------------------------------------------------
# Check 4: Full test suite (includes the generated-corpus-manifest freshness test)
# -----------------------------------------------------------------------------
Push-Location $workspaceRoot
try {
    $testOutput = & cargo test --workspace 2>&1
    if ($LASTEXITCODE -ne 0) {
        $tail = ($testOutput | Select-Object -Last 60) -join "`n"
        $failureReasons += "[Test Gate] cargo test --workspace failed:`n$tail"
    }
} finally {
    Pop-Location
}

# -----------------------------------------------------------------------------
# Check 5: Evidence separation. Reference evidence is produced only by the capture
# tooling and must never be added or modified in the same commit as engine code,
# so an implementation can never be "verified" by evidence written alongside it.
# Renames (e.g. quarantine moves) are allowed.
# -----------------------------------------------------------------------------
Push-Location $workspaceRoot
try {
    $changes = @(& git diff HEAD --name-status -M 2>$null) + @(& git ls-files --others --exclude-standard 2>$null | ForEach-Object { "A`t$_" })
    $evidenceEdits = @($changes | Where-Object { $_ -match '^[AM]\s+corpus/(unverified/)?evidence/' })
    $codeEdits = @($changes | Where-Object { $_ -match '^[AMD]\s+crates/' })
    if ($evidenceEdits.Count -gt 0 -and $codeEdits.Count -gt 0) {
        $failureReasons += "[Evidence Separation Gate] Evidence files are added/modified together with crates/ changes. Commit recorded evidence separately from engine code:`n" + (($evidenceEdits | Select-Object -First 10) -join "`n")
    }
} finally {
    Pop-Location
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

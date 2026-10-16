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
# Checks 1, 1b and 2: no panicking calls (check-sources.ps1, which CI runs too)
# -----------------------------------------------------------------------------
$sourceReport = & (Join-Path $PSScriptRoot 'check-sources.ps1') -WorkspaceRoot $workspaceRoot
if ($LASTEXITCODE -ne 0) {
    $failureReasons += ($sourceReport -join "`n")
}

# -----------------------------------------------------------------------------
# Check 3: Index Bounds Safety (clippy::indexing_slicing, clippy::string_slice)
# -----------------------------------------------------------------------------
Push-Location $workspaceRoot
try {
    $clippyOutput = & cargo clippy --all-targets --all-features -- -D warnings -D clippy::indexing_slicing -D clippy::string_slice 2>&1
    $clippyExit = $LASTEXITCODE
    if ($clippyExit -ne 0) {
        $errorMsg = ($clippyOutput -join "`n").Trim()
        $failureReasons += "[Bounds Safety Gate] cargo clippy detected direct slice/index operations (clippy::indexing_slicing, clippy::string_slice). Use .get(), .first(), .last() or str::get() instead of direct brackets:`n$errorMsg"
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
# Check 5: Reference-data separation. Evidence and UIA snapshots are produced only by the
# capture tooling on the reference machine and must never be added or modified in the same
# commit as engine code, so an implementation can never be "verified" by reference data
# written alongside it. Renames (e.g. quarantine moves) are allowed.
# Examines what the commit will contain: the index, plus tracked changes for `commit -a`.
# -----------------------------------------------------------------------------
Push-Location $workspaceRoot
try {
    $changes = @(& git diff --cached --name-status -M 2>$null)
    # `commit -a` and same-command `git add ... && git commit` stage after this hook runs,
    # so conservatively include unstaged and untracked changes in those cases.
    if ($cmdLine -match '\scommit\b[^\r\n;|&]*\s(-[a-zA-Z]*a[a-zA-Z]*|--all)\b' -or $cmdLine -match '\bgit\s+add\b') {
        $changes += @(& git diff --name-status -M 2>$null)
        $changes += @(& git ls-files --others --exclude-standard 2>$null | ForEach-Object { "A`t$_" })
    }
    $referenceEdits = @($changes | Where-Object { $_ -match '^[AM]\s+corpus/(unverified/)?(evidence|snapshots|traces)/' })
    $codeEdits = @($changes | Where-Object { $_ -match '^[AMD]\s+crates/' })
    if ($referenceEdits.Count -gt 0 -and $codeEdits.Count -gt 0) {
        $failureReasons += "[Reference Data Separation Gate] Evidence/snapshot files are added or modified in the same commit as crates/ changes. Commit captured reference data separately from engine code:`n" + (($referenceEdits | Select-Object -First 10) -join "`n")
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

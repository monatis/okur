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
    # Vendored xa11y (crates\xa11y) predates these rules; it is ratcheted by Check 1b instead.
    $vendoredDir = Join-Path $cratesDir "xa11y"
    $vendoredUnwraps = 0
    $vendoredPanics = 0

    foreach ($file in $rustFiles) {
        $isVendored = $file.FullName.StartsWith($vendoredDir + [System.IO.Path]::DirectorySeparatorChar)
        $lineNum = 0
        $lines = Get-Content -Path $file.FullName
        foreach ($line in $lines) {
            $lineNum++
            $trimmed = $line.Trim()
            if ($trimmed.StartsWith("//") -or $trimmed.StartsWith("/*") -or $trimmed.StartsWith("*")) {
                continue
            }
            if ($isVendored) {
                if ($line -match '\.(unwrap|expect)\s*\(') { $vendoredUnwraps++ }
                if ($line -match '\b(panic|todo|unimplemented|dbg)!') { $vendoredPanics++ }
                continue
            }
            if ($line -match '\.(unwrap|expect)\s*\(') {
                $relPath = $file.FullName.Substring($workspaceRoot.Length).TrimStart('\', '/')
                $unwrapViolations += "${relPath}:${lineNum}: ${trimmed}"
            }
            if ($line -match '\b(panic|todo|unimplemented|dbg)!') {
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

    # -------------------------------------------------------------------------
    # Check 1b: Vendored xa11y ratchet. Counts may only go down; lower them in
    # crates\xa11y\gate-baseline.json as the code is hardened.
    # -------------------------------------------------------------------------
    $baselinePath = Join-Path $vendoredDir "gate-baseline.json"
    if (Test-Path $vendoredDir) {
        if (-not (Test-Path $baselinePath)) {
            $failureReasons += "[Vendored Ratchet] Missing $baselinePath (current: unwrap_expect=$vendoredUnwraps, panic_macros=$vendoredPanics)."
        } else {
            $baseline = Get-Content -Raw $baselinePath | ConvertFrom-Json
            if ($vendoredUnwraps -gt $baseline.unwrap_expect -or $vendoredPanics -gt $baseline.panic_macros) {
                $failureReasons += "[Vendored Ratchet] crates\xa11y gained panicking calls: unwrap/expect $vendoredUnwraps (baseline $($baseline.unwrap_expect)), panic macros $vendoredPanics (baseline $($baseline.panic_macros)). New code must not panic."
            }
        }
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
    $referenceEdits = @($changes | Where-Object { $_ -match '^[AM]\s+corpus/(unverified/)?(evidence|snapshots)/' })
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

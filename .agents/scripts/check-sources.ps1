# The commit gate's source checks, shared with CI (.github/workflows/ci.yml):
# - no .unwrap() or .expect(), and no panic!, todo!, unimplemented! or dbg!, in the screen
#   reader's own crates, tests included (comments excepted);
# - the vendored xa11y crates (crates\xa11y) may only lose such calls: their counts are
#   ratcheted by crates\xa11y\gate-baseline.json.
# Prints each failure and exits 1; prints nothing and exits 0 when the sources are clean.
param(
    [string]$WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
)
$workspaceRoot = $WorkspaceRoot
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

if ($failureReasons.Count -gt 0) {
    Write-Output ($failureReasons -join "`n`n")
    exit 1
}
exit 0

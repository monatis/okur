# Claude Code Stop adapter for the shared code-quality gate.
#
# Gate logic lives in .agents/scripts/stop-hook.ps1 (single source of truth, shared
# with the Antigravity harness: cargo fmt + clippy -D warnings -D clippy::indexing_slicing -D clippy::string_slice).
# This adapter translates Antigravity {decision: continue} into Claude {decision: block},
# skips the gate when no Rust/Cargo files are dirty, and caps consecutive blocks so an
# unfixable failure cannot trap the session in an infinite Stop loop.

$ErrorActionPreference = 'Stop'
$raw = [Console]::In.ReadToEnd()
try { $payload = $raw | ConvertFrom-Json } catch { $payload = $null }

$projectDir = $env:CLAUDE_PROJECT_DIR
if (-not $projectDir) { $projectDir = Split-Path -Parent (Split-Path -Parent $PSScriptRoot) }

$sessionId = 'unknown'
if ($payload -and $payload.session_id) { $sessionId = [string]$payload.session_id }
$counterFile = Join-Path ([System.IO.Path]::GetTempPath()) "sr-stop-gate-$sessionId.count"
$maxConsecutiveBlocks = 3

# Only gate when Rust sources or manifests have uncommitted changes.
$dirty = & git -C $projectDir status --porcelain -- '*.rs' '*Cargo.toml' 'Cargo.lock' 2>$null
if (-not $dirty) {
    Remove-Item $counterFile -ErrorAction SilentlyContinue
    exit 0
}

$gate = Join-Path $projectDir '.agents\scripts\stop-hook.ps1'
$gateOut = '{}' | & powershell -NoProfile -ExecutionPolicy Bypass -File $gate 2>&1 | Out-String
$jsonLine = ($gateOut -split "`r?`n" | Where-Object { $_.Trim().StartsWith('{') } | Select-Object -Last 1)
try { $result = $jsonLine | ConvertFrom-Json } catch { $result = $null }

if ($result -and $result.decision -eq 'allow') {
    Remove-Item $counterFile -ErrorAction SilentlyContinue
    exit 0
}

$count = 0
if (Test-Path $counterFile) { $count = [int](Get-Content $counterFile -Raw) }
$count++
Set-Content -Path $counterFile -Value $count

$reason = if ($result) { [string]$result.reason } else { "Stop quality gate returned no decision. Raw output:`n$gateOut" }

if ($count -gt $maxConsecutiveBlocks) {
    Remove-Item $counterFile -ErrorAction SilentlyContinue
    @{
        systemMessage = "Quality gate still failing after $maxConsecutiveBlocks fix attempts; stopping anyway. Report the remaining clippy/fmt failures to the user explicitly."
    } | ConvertTo-Json -Compress
    exit 0
}

@{
    decision = 'block'
    reason   = "Quality gate failed (attempt $count/$maxConsecutiveBlocks). Fix these before finishing:`n$reason"
} | ConvertTo-Json -Compress
exit 0

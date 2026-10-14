# Claude Code PreToolUse adapter for the shared pre-commit quality gate.
#
# Gate logic lives in .agents/scripts/check-pre-commit.ps1 (single source of truth,
# shared with the Antigravity harness). This adapter only translates protocols:
#   Claude stdin  {tool_name, tool_input.command}  ->  Antigravity {toolCall.args.CommandLine}
#   Antigravity   {decision: allow|deny, reason}    ->  Claude hookSpecificOutput.permissionDecision

$ErrorActionPreference = 'Stop'
$raw = [Console]::In.ReadToEnd()
try { $payload = $raw | ConvertFrom-Json } catch { exit 0 }

$command = ''
if ($payload.tool_input -and $payload.tool_input.command) { $command = [string]$payload.tool_input.command }

# Fast path: anything that is not a git commit passes without spawning the gate.
# Matches 'git commit' with optional global options in between (-C dir, -c k=v, --no-pager).
if ($command -notmatch '(^|[\s;&|(])git(\s+(-[Cc]\s+\S+|--[\w-]+(=\S+)?))*\s+commit\b') { exit 0 }

$projectDir = $env:CLAUDE_PROJECT_DIR
if (-not $projectDir) { $projectDir = Split-Path -Parent (Split-Path -Parent $PSScriptRoot) }
$gate = Join-Path $projectDir '.agents\scripts\check-pre-commit.ps1'

# The shared gate only matches '\bgit\s+commit\b'; normalize so forms like
# 'git -C dir commit' are still gated.
$agPayload = @{ toolCall = @{ args = @{ CommandLine = 'git commit' } } } | ConvertTo-Json -Compress -Depth 5
$gateOut = $agPayload | & powershell -NoProfile -ExecutionPolicy Bypass -File $gate 2>&1 | Out-String

$jsonLine = ($gateOut -split "`r?`n" | Where-Object { $_.Trim().StartsWith('{') } | Select-Object -Last 1)
try { $result = $jsonLine | ConvertFrom-Json } catch { $result = $null }

if (-not $result) {
    $reason = "Pre-commit quality gate did not return a decision. Raw output:`n$gateOut"
    $decision = 'deny'
} elseif ($result.decision -eq 'deny') {
    $reason = [string]$result.reason
    $decision = 'deny'
} else {
    exit 0
}

@{
    hookSpecificOutput = @{
        hookEventName            = 'PreToolUse'
        permissionDecision       = $decision
        permissionDecisionReason = $reason
    }
} | ConvertTo-Json -Compress -Depth 5
exit 0

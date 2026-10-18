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

# The repository the commit is made in: its directory is the command's ('git -C <dir>', or a
# leading 'cd <dir>'), else the session's working directory. A git worktree (a subagent's)
# is gated with its own code and its own copy of the gate, which finds its workspace from
# where it lies, not with the main checkout's (2026-10-18: a worktree's commits were tested
# against the main checkout's tree).
function Get-CommitTree {
    $dir = if ($payload.cwd) { [string]$payload.cwd } else { (Get-Location).Path }
    $target = $null
    if ($command -match 'git\s+-C\s+(?:"([^"]+)"|''([^'']+)''|(\S+))') {
        $target = @($Matches[1], $Matches[2], $Matches[3]) | Where-Object { $_ } | Select-Object -First 1
    } elseif ($command -match '(?:^|[;&|(]\s*)cd\s+(?:"([^"]+)"|''([^'']+)''|([^\s;&|]+))') {
        $target = @($Matches[1], $Matches[2], $Matches[3]) | Where-Object { $_ } | Select-Object -First 1
    }
    if ($target) {
        # Git Bash's drive form, /c/Users/..., as Windows names it.
        if ($target -match '^/([a-zA-Z])(/.*)?$') { $target = "$($Matches[1]):$($Matches[2])" }
        $dir = if ([System.IO.Path]::IsPathRooted($target)) { $target } else { Join-Path $dir $target }
    }
    if (-not (Test-Path -LiteralPath $dir)) { return $null }
    $top = & git -C $dir rev-parse --show-toplevel 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $top) { return $null }
    return ([string]$top).Trim()
}

$tree = Get-CommitTree
$gate = Join-Path $projectDir '.agents\scripts\check-pre-commit.ps1'
if ($tree) {
    $treeGate = Join-Path $tree '.agents\scripts\check-pre-commit.ps1'
    if (Test-Path -LiteralPath $treeGate) { $gate = $treeGate }
}

# The shared gate only matches '\bgit\s+commit\b'; prefix it so forms like
# 'git -C dir commit' are still gated, while keeping the real command for flag checks.
$agPayload = @{ toolCall = @{ args = @{ CommandLine = "git commit $command" } } } | ConvertTo-Json -Compress -Depth 5
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

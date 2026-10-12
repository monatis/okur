$inputJson = [Console]::In.ReadToEnd()

# Set working directory to workspace root
$workspaceRoot = (Resolve-Path "$PSScriptRoot\..").Path
Set-Location $workspaceRoot

# Auto-format with cargo fmt
& cargo fmt 2>&1 | Out-Null

# Run clippy with warnings denied
$clippyOutput = & cargo clippy --all-targets --all-features -- -D warnings 2>&1
$clippyExitCode = $LASTEXITCODE

if ($clippyExitCode -ne 0) {
    $errorMsg = ($clippyOutput -join "`n").Trim()
    $response = [ordered]@{
        decision = "continue"
        reason = "Automated Quality Gate failed on Stop event. cargo clippy reported errors or warnings:`n$errorMsg"
    }
    $response | ConvertTo-Json -Compress
} else {
    $response = [ordered]@{
        decision = "allow"
    }
    $response | ConvertTo-Json -Compress
}

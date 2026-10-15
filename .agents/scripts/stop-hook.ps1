$inputJson = [Console]::In.ReadToEnd()

# Set working directory to workspace root (walk up until Cargo.toml is found)
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
Set-Location $workspaceRoot

# Auto-format with cargo fmt
& cargo fmt 2>&1 | Out-Null

# Run clippy with warnings, unchecked indexing and string slicing denied
$clippyOutput = & cargo clippy --all-targets --all-features -- -D warnings -D clippy::indexing_slicing -D clippy::string_slice 2>&1
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

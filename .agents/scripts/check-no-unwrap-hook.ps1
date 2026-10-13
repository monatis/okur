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

$cratesDir = Join-Path $workspaceRoot "crates"
if (-not (Test-Path $cratesDir)) {
    $response = [ordered]@{
        decision = "allow"
    }
    $response | ConvertTo-Json -Compress
    exit 0
}

$rustFiles = Get-ChildItem -Path $cratesDir -Recurse -Filter *.rs
$violations = @()

foreach ($file in $rustFiles) {
    $lineNum = 0
    $lines = Get-Content -Path $file.FullName
    foreach ($line in $lines) {
        $lineNum++
        # Match .unwrap() or .expect() calls, excluding comments
        $trimmed = $line.Trim()
        if ($trimmed.StartsWith("//") -or $trimmed.StartsWith("/*") -or $trimmed.StartsWith("*")) {
            continue
        }
        if ($line -match '\.(unwrap|expect)\s*\(') {
            $relPath = $file.FullName.Substring($workspaceRoot.Length).TrimStart('\', '/')
            $violations += "${relPath}:${lineNum}: ${trimmed}"
        }
    }
}

if ($violations.Count -gt 0) {
    $displayLimit = 25
    $violationList = ($violations | Select-Object -First $displayLimit) -join "`n"
    if ($violations.Count -gt $displayLimit) {
        $violationList += "`n... and $($violations.Count - $displayLimit) more violation(s)"
    }

    $reason = "Production reliability gate: Blocked 'git commit' because $($violations.Count) usage(s) of .unwrap() or .expect() were found in the codebase.`n`nScreen readers are an essential accessibility layer for the blind and must NEVER panic or crash at runtime.`n`nPlease clean up all .unwrap() and .expect() calls (replace with unwrap_or, unwrap_or_else, unwrap_or_default, pattern matching, or returning Result/Option) before committing:`n`n$violationList"

    $response = [ordered]@{
        decision = "deny"
        reason = $reason
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

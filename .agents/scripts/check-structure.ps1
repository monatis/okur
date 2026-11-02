# The workspace's structure, counted in its sources and held to .agents/structure-baseline.json
# (docs/references/crate-structure-proposal.md, section 10, rules R4 to R6):
# - neutral_platform_cfg (R4): `cfg` on the platform in a neutral crate's sources;
# - platform_literals_in_core (R5): `okur-core` comparing a `PlatformIds` field with a literal;
# - engine_app_references (R6): the engine naming application modules (`apps::windows::...`
#   outside `apps/`, `word::` in `editing.rs`);
# - uia_outside_xa11y (R6): lines naming `IUIAutomation` outside crates/xa11y;
# - engine_fields (R6): the fields of `ScreenReaderEngine`;
# - platform_cfg_sites (R6): `cfg` on the platform in the Windows driver crate;
# - large_files (R6): files of the okur crates over the limit in production lines (the lines
#   before a file's first inline `#[cfg(test)] mod`), which may not grow.
# A count in "block" mode may not rise above its baseline, and once it falls the baseline is
# lowered with it; a count in "report" mode is printed when it differs, with -Report always.
# Prints each failure and exits 1; prints nothing and exits 0 otherwise.
param(
    [string]$WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path,
    [switch]$Report
)
$baselinePath = Join-Path $WorkspaceRoot '.agents\structure-baseline.json'
$baseline = Get-Content -Raw -Path $baselinePath | ConvertFrom-Json
$layers = Get-Content -Raw -Path (Join-Path $WorkspaceRoot '.agents\crate-layers.json') | ConvertFrom-Json
$crates = Join-Path $WorkspaceRoot 'crates'
$platformCfg = '(cfg!?\s*\(|cfg_attr\s*\().*\b(windows|unix|target_os|target_family)\b'

function Get-CratePath([string]$name) {
    $direct = Join-Path $crates $name
    if (Test-Path $direct) { return $direct }
    $vendored = Join-Path (Join-Path $crates 'xa11y') $name
    if (Test-Path $vendored) { return $vendored }
    return $null
}

function Get-CodeLines([string]$dir) {
    if (-not $dir -or -not (Test-Path $dir)) { return @() }
    Get-ChildItem -Path $dir -Recurse -Filter *.rs | ForEach-Object {
        $path = $_.FullName
        $n = 0
        foreach ($line in [System.IO.File]::ReadAllLines($path)) {
            $n++
            $trimmed = $line.TrimStart()
            if ($trimmed.StartsWith('//')) { continue }
            [pscustomobject]@{ Path = $path; Number = $n; Text = $line }
        }
    }
}

function Relative([string]$path) {
    $path.Substring($WorkspaceRoot.Length).TrimStart('\', '/').Replace('\', '/')
}

$counts = [ordered]@{}

# R4
$neutral = @($layers.crates.PSObject.Properties | Where-Object { $_.Value.kind -eq 'neutral' } | ForEach-Object { $_.Name })
$counts['neutral_platform_cfg'] = @($neutral | ForEach-Object { Get-CodeLines (Get-CratePath $_) } |
    Where-Object { $_.Text -match $platformCfg }).Count

# R5 and the engine's references to application modules
$core = Get-CodeLines (Join-Path $crates 'okur-core\src')
$counts['platform_literals_in_core'] = @($core | Where-Object { $_.Text -match 'platform\.\w+.*==\s*Some\("' }).Count
$apps = [regex]'apps::windows::[a-z_]+'
$word = [regex]'\bword::'
$appsDir = Join-Path $crates 'okur-core\src\apps'
$editing = Join-Path $crates 'okur-core\src\editing.rs'
$references = 0
foreach ($line in $core) {
    if ($line.Path.StartsWith($appsDir)) { continue }
    $references += $apps.Matches($line.Text).Count
    if ($line.Path -eq $editing) { $references += $word.Matches($line.Text).Count }
}
$counts['engine_app_references'] = $references

# R6
$okurCrates = Get-ChildItem -Path $crates -Directory | Where-Object { $_.Name -ne 'xa11y' }
$counts['uia_outside_xa11y'] = @($okurCrates | ForEach-Object { Get-CodeLines $_.FullName } |
    Where-Object { $_.Text -match 'IUIAutomation' }).Count
$engine = [System.IO.File]::ReadAllLines((Join-Path $crates 'okur-core\src\engine.rs'))
$inside = $false
$fields = 0
foreach ($line in $engine) {
    if ($line -match '^pub struct ScreenReaderEngine\b') { $inside = $true; continue }
    if ($inside -and $line -match '^\}') { break }
    if ($inside -and $line -match '^    (pub(\([a-z]+\))?\s+)?[a-z_][a-z0-9_]*\s*:') { $fields++ }
}
$counts['engine_fields'] = $fields
$counts['platform_cfg_sites'] = @(Get-CodeLines (Get-CratePath $baseline.platform_crate) |
    Where-Object { $_.Text -match $platformCfg }).Count

$failures = @()
$notes = @()
foreach ($key in $counts.Keys) {
    $entry = $baseline.ratchets.$key
    if (-not $entry) {
        $failures += "[Structure] $key has no baseline in .agents/structure-baseline.json (now $($counts[$key]))"
        continue
    }
    $now = $counts[$key]
    $was = [int]$entry.count
    if ($entry.mode -eq 'block' -and $now -gt $was) {
        $failures += "[Structure] $key rose to $now (baseline $was): $($entry.why)"
    } elseif ($entry.mode -eq 'block' -and $now -lt $was) {
        $failures += "[Structure] $key fell to $now (baseline $was): lower its count in .agents/structure-baseline.json"
    } elseif ($Report -or $now -ne $was) {
        $notes += "[report] $key $now (baseline $was)"
    }
}

# Large files
$limit = [int]$baseline.large_files.limit
$large = [ordered]@{}
foreach ($dir in $okurCrates) {
    Get-ChildItem -Path $dir.FullName -Recurse -Filter *.rs | Where-Object {
        $_.Name -ne 'tests.rs' -and $_.Name -notlike '*_tests.rs' -and $_.FullName -notmatch '[\\/]tests[\\/]'
    } | ForEach-Object {
        $lines = [System.IO.File]::ReadAllLines($_.FullName)
        $production = $lines.Count
        for ($i = 0; $i -lt $lines.Count - 1; $i++) {
            if ($lines[$i] -match '^#\[cfg\(test\)\]\s*$' -and $lines[$i + 1] -match '^(pub(\([a-z]+\))?\s+)?mod\s+\w+\s*\{') {
                $production = $i
                break
            }
        }
        if ($production -gt $limit) { $large[(Relative $_.FullName)] = $production }
    }
}
foreach ($file in $large.Keys) {
    $was = $baseline.large_files.files.$file
    if ($null -eq $was) {
        $notes += "[report] $file has $($large[$file]) production lines, over $limit"
    } elseif ($large[$file] -gt [int]$was) {
        $notes += "[report] $file grew to $($large[$file]) production lines (baseline $was)"
    } elseif ($Report) {
        $notes += "[report] $file $($large[$file]) production lines (baseline $was)"
    }
}
foreach ($file in $baseline.large_files.files.PSObject.Properties.Name) {
    if (-not $large.Contains($file)) { $notes += "[report] $file is no longer over $limit production lines" }
}
if ($baseline.large_files.mode -eq 'block') {
    $failures += @($notes | Where-Object { $_ -match 'grew to|, over \d+$' })
}

if ($Report -or $failures.Count -gt 0) { $notes }
if ($failures.Count -gt 0) {
    $failures
    exit 1
}
exit 0

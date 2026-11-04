# The workspace's layers, checked on `cargo metadata` (no build needed), against
# .agents/crate-layers.json (docs/references/crate-structure-proposal.md, section 10):
# - R0: every workspace crate is listed there;
# - R1: a crate depends (normal, build or dev) only on the internal crates it may, or on a
#   known edge the migration has not yet removed; a known edge that is gone must be unlisted;
# - R2: a neutral crate declares no platform API crate, for any target;
# - R3: no program depends, directly or through others, on a lab crate; nothing depends on
#   okur-lab.
# - R4: a portable crate (neutral, or marked "portable") depends (normal or build) on no
#   platform, program or lab crate but a portable one, so that the portable CI job can build
#   and test it on macOS and Linux (a vendored crate builds there: okur-windows-model takes
#   xa11y-windows's IA2 data types).
# Prints each failure and exits 1; prints nothing and exits 0 when the layers hold.
#   -Report    also print the known edges still to remove
#   -Neutral   print the neutral crates' names, one per line
#   -Portable  print the portable crates' names, one per line: the portable CI job tests them
param(
    [string]$WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path,
    [switch]$Report,
    [switch]$Neutral,
    [switch]$Portable
)
$layers = Get-Content -Raw -Path (Join-Path $WorkspaceRoot '.agents/crate-layers.json') | ConvertFrom-Json
$crates = @{}
foreach ($p in $layers.crates.PSObject.Properties) { $crates[$p.Name] = $p.Value }

$portableCrates = @($crates.Keys | Where-Object { $crates[$_].kind -eq 'neutral' -or $crates[$_].portable -eq $true })
if ($Neutral) {
    $crates.Keys | Where-Object { $crates[$_].kind -eq 'neutral' } | Sort-Object
    exit 0
}
if ($Portable) {
    $portableCrates | Sort-Object
    exit 0
}

Push-Location $WorkspaceRoot
try {
    $json = & cargo metadata --no-deps --format-version 1 --offline 2>$null
    if ($LASTEXITCODE -ne 0) { $json = & cargo metadata --no-deps --format-version 1 2>$null }
} finally {
    Pop-Location
}
if (-not $json) {
    "[Crate Layers] cargo metadata failed"
    exit 1
}
$meta = ($json -join "`n") | ConvertFrom-Json

$failures = @()
$edges = @{}
foreach ($package in $meta.packages) {
    $name = $package.name
    if (-not $crates.ContainsKey($name)) {
        $failures += "[Crate Layers] R0: $name is not in .agents/crate-layers.json: give it a kind and its allowed dependencies"
        continue
    }
    $rule = $crates[$name]
    $internal = @($package.dependencies | Where-Object { $_.path -and $_.name -ne $name })
    $edges[$name] = @($internal | Where-Object { $_.kind -ne 'dev' } | ForEach-Object { $_.name } | Sort-Object -Unique)
    foreach ($dep in ($internal | ForEach-Object { $_.name } | Sort-Object -Unique)) {
        if ($rule.allow -contains $dep) { continue }
        $known = @($layers.known_edges | Where-Object { $_.from -eq $name -and $_.to -eq $dep })
        if ($known.Count -gt 0) {
            if ($Report) { "[report] known edge $name -> $dep, until $($known[0].until)" }
            continue
        }
        $failures += "[Crate Layers] R1: $name may not depend on $dep (allowed: $($rule.allow -join ', '))"
    }
    if ($rule.kind -eq 'neutral') {
        foreach ($dep in $package.dependencies) {
            $base = $dep.name -replace '-.*$', ''
            if ($layers.forbidden_in_neutral -contains $dep.name -or ($base -eq 'objc2')) {
                $failures += "[Crate Layers] R2: $name is neutral and declares $($dep.name) (target $($dep.target))"
            }
        }
    }
}
foreach ($known in $layers.known_edges) {
    $present = $meta.packages | Where-Object { $_.name -eq $known.from } |
        ForEach-Object { $_.dependencies } | Where-Object { $_.path -and $_.name -eq $known.to }
    if (-not $present) {
        $failures += "[Crate Layers] R1: the known edge $($known.from) -> $($known.to) is gone: remove it from known_edges"
    }
}

# R3: the dependency closure of each program, its dev dependencies left out.
$lab = @($crates.Keys | Where-Object { $crates[$_].kind -eq 'lab' })
foreach ($program in ($crates.Keys | Where-Object { $crates[$_].kind -eq 'program' })) {
    $seen = @{}
    $queue = New-Object System.Collections.Queue
    $queue.Enqueue($program)
    while ($queue.Count -gt 0) {
        $current = $queue.Dequeue()
        foreach ($dep in @($edges[$current])) {
            if (-not $dep -or $seen.ContainsKey($dep)) { continue }
            $seen[$dep] = $current
            $queue.Enqueue($dep)
        }
    }
    foreach ($crate in $lab) {
        if ($seen.ContainsKey($crate)) {
            $failures += "[Crate Layers] R3: the program $program depends on the lab crate $crate (through $($seen[$crate]))"
        }
    }
}
foreach ($crate in $portableCrates) {
    foreach ($dep in @($edges[$crate])) {
        $kind = if ($crates.ContainsKey($dep)) { $crates[$dep].kind } else { '' }
        if (@('platform', 'program', 'lab') -contains $kind -and -not ($portableCrates -contains $dep)) {
            $failures += "[Crate Layers] R4: $crate is portable and depends on $dep, which is not"
        }
    }
}
foreach ($package in $meta.packages) {
    if ($package.name -ne 'okur-lab' -and ($package.dependencies | Where-Object { $_.path -and $_.name -eq 'okur-lab' })) {
        $failures += "[Crate Layers] R3: $($package.name) depends on okur-lab, which nothing may"
    }
}

if ($failures.Count -gt 0) {
    $failures
    exit 1
}
exit 0

<#
.SYNOPSIS
    Sets JAWS's user settings for a recording, and puts them back afterwards. Lab only.

.DESCRIPTION
    A scenario that needs a setting other than old-pc's (a punctuation level, typing echo)
    names it in "jaws_settings", in the shape the evidence's provenance records them:

        "jaws_settings": {
            "DEFAULT.JCF": { "Options": { "TypingEcho": "1" } },
            "voiceprofiles/SAPI 5X 64.VPF": { "ENU-PCCursor": { "Punctuation": "3" } }
        }

    A file is named as in the provenance: under Settings\enu, or "voiceprofiles/<name>". A value
    of null removes the key, so JAWS's default applies.

    -Apply writes those keys into the user's files, after copying every file it touches to
    sr-harness\settings-backup (once: a second -Apply first puts the first one's files back).
    -Restore puts the copies back, deletes the files -Apply created, and removes the copies.
    -Check says whether the files hold the values (exit 0) or not (exit 1).

    JAWS reads its settings when it starts and may write them when it exits, so run-batch.ps1
    stops JAWS (killed, so it writes nothing), applies or restores, then starts it again. Never
    on the dev box, where JAWS and its settings are the user's.
#>
param(
    # The settings, as JSON (an object as above), or the path of a scenario file holding them.
    [string]$Apply,
    [switch]$Restore,
    [string]$Check,
    [string]$Root = (Join-Path $env:APPDATA 'Freedom Scientific\JAWS\2025\Settings'),
    [string]$Backup = (Join-Path $env:USERPROFILE 'sr-harness\settings-backup')
)
$ErrorActionPreference = 'Stop'

# The settings object from JSON text or a scenario file ($null when the scenario has none).
function Get-Wanted([string]$source) {
    $json = if (Test-Path -LiteralPath $source) { Get-Content -LiteralPath $source -Raw -Encoding UTF8 } else { $source }
    $value = $json | ConvertFrom-Json
    if ($value.PSObject.Properties.Name -contains 'jaws_settings') { return $value.jaws_settings }
    if ($value.PSObject.Properties.Name -contains 'steps') { return $null }
    return $value
}

function Get-SettingsPath([string]$name) {
    if ($name -match '^voiceprofiles/(.+)$') { return Join-Path $Root "voiceprofiles\$($Matches[1])" }
    if ($name -match '[\\/]') { throw "A settings file is named 'NAME.JCF' or 'voiceprofiles/NAME.VPF': '$name'" }
    return Join-Path $Root "enu\$name"
}

# A file's lines and whether it starts with a UTF-8 byte-order mark (DEFAULT.JCF does on old-pc).
function Read-Lines([string]$path) {
    if (-not (Test-Path -LiteralPath $path)) { return @{ bom = $false; lines = @() } }
    $bytes = [IO.File]::ReadAllBytes($path)
    $bom = $bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF
    $text = (New-Object Text.UTF8Encoding $false).GetString($bytes, $(if ($bom) { 3 } else { 0 }), $bytes.Length - $(if ($bom) { 3 } else { 0 }))
    $lines = @($text -split "`r?`n")
    if ($lines.Count -gt 0 -and $lines[-1] -eq '') { $lines = @($lines | Select-Object -First ($lines.Count - 1)) }
    return @{ bom = $bom; lines = $lines }
}

function Write-Lines([string]$path, $file) {
    $dir = Split-Path -Parent $path
    if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    $text = (($file.lines -join "`r`n") + "`r`n")
    [IO.File]::WriteAllText($path, $text, (New-Object Text.UTF8Encoding $file.bom))
}

# Sets (or, with $null, removes) section/key in the lines, keeping everything else as it was.
function Set-IniValue($lines, [string]$section, [string]$key, $value) {
    $out = New-Object System.Collections.Generic.List[string]
    $inSection = $false
    $sectionSeen = $false
    $done = $false
    foreach ($line in $lines) {
        $t = $line.Trim()
        if ($t -match '^\[(.+)\]$') {
            if ($inSection -and -not $done -and $null -ne $value) { $out.Add("$key=$value"); $done = $true }
            $inSection = $Matches[1] -eq $section
            if ($inSection) { $sectionSeen = $true }
            $out.Add($line)
            continue
        }
        if ($inSection -and $t -match '^([^=;]+)=' -and $Matches[1].Trim() -eq $key) {
            if (-not $done -and $null -ne $value) { $out.Add("$key=$value") }
            $done = $true
            continue
        }
        $out.Add($line)
    }
    if (-not $done -and $null -ne $value) {
        if (-not $sectionSeen) { $out.Add("[$section]") }
        $out.Add("$key=$value")
    }
    return @($out)
}

function Get-IniValue($lines, [string]$section, [string]$key) {
    $inSection = $false
    foreach ($line in $lines) {
        $t = $line.Trim()
        if ($t -match '^\[(.+)\]$') { $inSection = $Matches[1] -eq $section; continue }
        if ($inSection -and $t -match '^([^=;]+)=(.*)$' -and $Matches[1].Trim() -eq $key) { return $Matches[2] }
    }
    return $null
}

# Each wanted value as file, section, key, value.
function Get-Entries($wanted) {
    foreach ($file in $wanted.PSObject.Properties) {
        foreach ($section in $file.Value.PSObject.Properties) {
            foreach ($key in $section.Value.PSObject.Properties) {
                [pscustomobject]@{ file = $file.Name; section = $section.Name; key = $key.Name; value = $key.Value }
            }
        }
    }
}

function Invoke-Restore {
    $manifest = Join-Path $Backup 'manifest.json'
    if (-not (Test-Path -LiteralPath $manifest)) { return }
    foreach ($entry in (Get-Content -LiteralPath $manifest -Raw -Encoding UTF8 | ConvertFrom-Json)) {
        $path = Get-SettingsPath $entry.file
        if ($entry.existed) { Copy-Item -LiteralPath (Join-Path $Backup $entry.copy) -Destination $path -Force }
        elseif (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }
    }
    Remove-Item -LiteralPath $Backup -Recurse -Force
}

if ($Restore) { Invoke-Restore; 'restored'; exit 0 }

if ($Check) {
    $wanted = Get-Wanted $Check
    if ($null -eq $wanted) { 'no settings asked for'; exit 0 }
    $wrong = @(Get-Entries $wanted | Where-Object {
        $have = Get-IniValue (Read-Lines (Get-SettingsPath $_.file)).lines $_.section $_.key
        $have -ne $_.value
    })
    if ($wrong.Count -eq 0) { 'settings in place'; exit 0 }
    $wrong | ForEach-Object { "not set: $($_.file) [$($_.section)] $($_.key)=$($_.value)" }
    exit 1
}

if ($Apply) {
    $wanted = Get-Wanted $Apply
    Invoke-Restore
    if ($null -eq $wanted) { 'no settings asked for'; exit 0 }
    $entries = @(Get-Entries $wanted)
    New-Item -ItemType Directory -Force -Path $Backup | Out-Null
    $manifest = @()
    $n = 0
    foreach ($name in @($entries | ForEach-Object { $_.file } | Sort-Object -Unique)) {
        $path = Get-SettingsPath $name
        $n++
        $copy = "file$n"
        $existed = Test-Path -LiteralPath $path
        if ($existed) { Copy-Item -LiteralPath $path -Destination (Join-Path $Backup $copy) }
        $manifest += [pscustomobject]@{ file = $name; existed = $existed; copy = $copy }
    }
    # The manifest is written before any file changes, so -Restore can always undo.
    ConvertTo-Json @($manifest) | Out-File (Join-Path $Backup 'manifest.json') -Encoding utf8
    foreach ($group in ($entries | Group-Object file)) {
        $path = Get-SettingsPath $group.Name
        $file = Read-Lines $path
        foreach ($e in $group.Group) { $file.lines = Set-IniValue $file.lines $e.section $e.key $e.value }
        Write-Lines $path $file
    }
    $entries | ForEach-Object { if ($null -eq $_.value) { "removed: $($_.file) [$($_.section)] $($_.key)" } else { "set: $($_.file) [$($_.section)] $($_.key)=$($_.value)" } }
    exit 0
}

Get-Help $PSCommandPath

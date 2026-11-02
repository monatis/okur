<#
.SYNOPSIS
    Builds Okur's MSIX package for the Microsoft Store (unsigned; signed only when a certificate
    is given).

.DESCRIPTION
    docs/references/microsoft-store-release-research.md §8: a full-trust package holding the
    screen reader (okur-sr.exe) and Okur's windows (okur-ui.exe), starting after sign-in through
    its startup task. Without -UiAccess the reader is the build without UI Access
    (okur-sr-no-uiaccess.exe, under the name okur-sr.exe); with it, okur-sr.exe and the uiAccess
    restricted capability, which experiment E1 on old-pc must show working before a submission
    declares it.

    The identity is Partner Center's, from its Product identity page (the owner, 2026-11-01:
    Monatis.okur, published by Monatis; the package family name Monatis.okur_rfy6yt361pz56,
    Store ID 9NV0FM5H54X8). The Store signs what it publishes; -CertificateThumbprint signs a
    local test package with a certificate from the current user's store, whose subject must
    equal the publisher. Trusting that certificate on a machine is a system change, its owner's.

    Logos: Okur's mark (draw-logo.ps1, okur-logo.svg) is drawn into the package's Assets at the
    sizes the manifest names, unless -AssetsDir gives others.

.EXAMPLE
    powershell -NoProfile -File packaging\msix\build-msix.ps1 -Version 1.0.0.0
#>
param(
    # Four numbers, each up to 65535: the first not 0 and the last 0, as the Store requires
    # (research R-store-23).
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$Name = 'Monatis.okur',
    [string]$Publisher = 'CN=4A33A3E2-13C6-458B-B8DF-B58C4CA3CA30',
    [string]$PublisherDisplayName = 'Monatis',
    [switch]$UiAccess,
    [string]$AssetsDir,
    [string]$OutDir,
    [string]$CertificateThumbprint,
    # Signs the programs in the package (Authenticode) with this certificate from the current
    # user's store before packing. UI Access needs it: Windows checks okur-sr.exe's own
    # signature, and with only the package signed it refuses to start it ("A referral was
    # returned from the server", E1 on old-pc, 2026-11-02).
    [string]$ProgramCertificateThumbprint,
    # Use the release binaries already built.
    [switch]$SkipBuild
)
$ErrorActionPreference = 'Stop'
# Run as a script, or as an encoded command from the repository's root (where scripts may not
# run), which has no script root.
$here = if ($PSScriptRoot) { $PSScriptRoot } else { Join-Path (Get-Location) 'packaging\msix' }
$repo = Resolve-Path (Join-Path $here '..\..')
if (-not $OutDir) { $OutDir = Join-Path $repo 'target\msix' }

# The version, as the Store takes it.
$parts = $Version.Split('.')
$numbers = @($parts | ForEach-Object { $n = 0; if ([int]::TryParse($_, [ref]$n) -and $n -ge 0 -and $n -le 65535) { $n } })
if ($parts.Count -ne 4 -or $numbers.Count -ne 4 -or $numbers[0] -eq 0 -or $numbers[3] -ne 0) {
    throw "Version '$Version': four numbers up to 65535, the first not 0 and the last 0 (e.g. 1.0.0.0)"
}

# The Windows SDK's packaging tools, the newest found.
$kits = 'C:\Program Files (x86)\Windows Kits\10\bin'
$makeAppx = Get-ChildItem $kits -Recurse -Filter makeappx.exe -ErrorAction SilentlyContinue |
    Where-Object { $_.Directory.Name -eq 'x64' } | Sort-Object FullName | Select-Object -Last 1
if (-not $makeAppx) { throw "MakeAppx.exe not found under ${kits}: install the Windows SDK" }

if (-not $SkipBuild) {
    # The dev box is shared: cargo's jobs as the environment sets them (CARGO_BUILD_JOBS).
    & cargo build --release -p okur --bin okur-sr --bin okur-sr-no-uiaccess
    if ($LASTEXITCODE -ne 0) { throw 'cargo build of okur-sr failed' }
    & cargo build --release -p okur-ui
    if ($LASTEXITCODE -ne 0) { throw 'cargo build of okur-ui failed' }
}
$release = Join-Path $repo 'target\release'
$reader = if ($UiAccess) { 'okur-sr.exe' } else { 'okur-sr-no-uiaccess.exe' }
foreach ($file in @($reader, 'okur-ui.exe')) {
    if (-not (Test-Path (Join-Path $release $file))) { throw "$file is not in $release" }
}

# The package's files.
$stage = Join-Path $OutDir 'stage'
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force (Join-Path $stage 'Assets') | Out-Null
Copy-Item (Join-Path $release $reader) (Join-Path $stage 'okur-sr.exe')
Copy-Item (Join-Path $release 'okur-ui.exe') $stage
Copy-Item (Join-Path $repo 'LICENSE'), (Join-Path $repo 'NOTICE') $stage

# The programs' own signatures, in the staged copies (the release binaries stay as built).
if ($ProgramCertificateThumbprint) {
    $signTool = Join-Path $makeAppx.Directory.FullName 'signtool.exe'
    & $signTool sign /fd SHA256 /sha1 $ProgramCertificateThumbprint /s My `
        (Join-Path $stage 'okur-sr.exe') (Join-Path $stage 'okur-ui.exe') | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'SignTool failed on the programs' }
} elseif ($UiAccess) {
    Write-Warning ('okur-sr.exe is not signed: Windows starts a UI Access program only with a ' +
        'trusted signature of its own, which the package''s signature was not on old-pc ' +
        '(E1, 2026-11-02). Whether the Store''s signature counts is not known yet.')
}

# The logos the manifest names, at their sizes: Okur's mark (draw-logo.ps1), run as a script
# block, as this script may be where script files do not run; and the Store listing's logos
# beside the package.
$logos = [ordered]@{ 'StoreLogo.png' = 50; 'Square44x44Logo.png' = 44; 'Square150x150Logo.png' = 150 }
$drawLogo = [scriptblock]::Create((Get-Content -Raw -Encoding UTF8 (Join-Path $here 'draw-logo.ps1')))
if ($AssetsDir) {
    foreach ($logo in $logos.Keys) { Copy-Item (Join-Path $AssetsDir $logo) (Join-Path $stage 'Assets') }
} else {
    foreach ($logo in $logos.Keys) {
        & $drawLogo -Size $logos[$logo] -Path (Join-Path $stage "Assets\$logo")
    }
}
foreach ($size in 300, 1080) {
    & $drawLogo -Size $size -Path (Join-Path $OutDir "listing\okur-logo-$size.png")
}

# The manifest. UI Access in a package needs Windows 10 build 18990 or later (R-store-10);
# without it, 1809 (R-store-33).
$manifest = Get-Content -Raw -Encoding UTF8 (Join-Path $here 'AppxManifest.xml.in')
$escape = { param($text) [System.Security.SecurityElement]::Escape($text) }
$values = @{
    '@NAME@' = & $escape $Name
    '@PUBLISHER@' = & $escape $Publisher
    '@PUBLISHER_DISPLAY_NAME@' = & $escape $PublisherDisplayName
    '@VERSION@' = $Version
    '@MIN_VERSION@' = if ($UiAccess) { '10.0.18990.0' } else { '10.0.17763.0' }
    '@UI_ACCESS@' = if ($UiAccess) { '    <rescap:Capability Name="uiAccess" />' } else { '' }
}
foreach ($key in $values.Keys) { $manifest = $manifest.Replace($key, $values[$key]) }
if ($manifest -match '@[A-Z_]+@') { throw "the manifest still holds $($Matches[0])" }
[System.IO.File]::WriteAllText((Join-Path $stage 'AppxManifest.xml'), $manifest, (New-Object System.Text.UTF8Encoding $false))

# The package.
$variant = if ($UiAccess) { '-uiaccess' } else { '' }
$package = Join-Path $OutDir "okur-$Version-x64$variant.msix"
& $makeAppx.FullName pack /o /d $stage /p $package | Out-Host
if ($LASTEXITCODE -ne 0) { throw 'MakeAppx failed' }

if ($CertificateThumbprint) {
    $signTool = Join-Path $makeAppx.Directory.FullName 'signtool.exe'
    & $signTool sign /fd SHA256 /sha1 $CertificateThumbprint $package | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'SignTool failed' }
}
"$package $((Get-FileHash $package).Hash.ToLower())"

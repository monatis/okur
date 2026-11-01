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

    The identity comes from Partner Center's Product identity page (-Name, -Publisher,
    -PublisherDisplayName); without them the package carries a test identity, for local
    installs only. The Store signs what it publishes; -CertificateThumbprint signs a local test
    package with a certificate from the current user's store, whose subject must equal
    -Publisher. Trusting that certificate on a machine is a system change, its owner's.

    Logos: Okur has none yet (the owner's to choose); placeholder squares are drawn into the
    package's Assets unless -AssetsDir gives real ones.

.EXAMPLE
    powershell -NoProfile -File packaging\msix\build-msix.ps1 -Version 1.0.0.0
#>
param(
    # Four numbers, each up to 65535: the first not 0 and the last 0, as the Store requires
    # (research R-store-23).
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$Name = 'Okur.Test',
    [string]$Publisher = 'CN=Okur Test',
    [string]$PublisherDisplayName = 'Okur (test package)',
    [switch]$UiAccess,
    [string]$AssetsDir,
    [string]$OutDir,
    [string]$CertificateThumbprint,
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

# The logos the manifest names, at their sizes.
$logos = [ordered]@{ 'StoreLogo.png' = 50; 'Square44x44Logo.png' = 44; 'Square150x150Logo.png' = 150 }
if ($AssetsDir) {
    foreach ($logo in $logos.Keys) { Copy-Item (Join-Path $AssetsDir $logo) (Join-Path $stage 'Assets') }
} else {
    Add-Type -AssemblyName System.Drawing
    foreach ($logo in $logos.Keys) {
        $size = $logos[$logo]
        $bitmap = New-Object System.Drawing.Bitmap $size, $size
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $graphics.Clear([System.Drawing.Color]::FromArgb(255, 32, 64, 96))
        $font = New-Object System.Drawing.Font 'Segoe UI', ([float]($size * 0.55)), ([System.Drawing.FontStyle]::Bold), ([System.Drawing.GraphicsUnit]::Pixel)
        $format = New-Object System.Drawing.StringFormat
        $format.Alignment = 'Center'
        $format.LineAlignment = 'Center'
        $graphics.DrawString('O', $font, [System.Drawing.Brushes]::White, (New-Object System.Drawing.RectangleF 0, 0, $size, $size), $format)
        $bitmap.Save((Join-Path $stage "Assets\$logo"), [System.Drawing.Imaging.ImageFormat]::Png)
        $graphics.Dispose(); $bitmap.Dispose(); $font.Dispose()
    }
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

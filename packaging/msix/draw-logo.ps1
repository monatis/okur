<#
.SYNOPSIS
    Draws Okur's mark as a square PNG of the given size: the geometry of okur-logo.svg.

.DESCRIPTION
    A white letter O with two amber arcs spreading from its right like sound, on a deep navy
    rounded square. Drawn with GDI+ (System.Drawing), anti-aliased, on a 100-unit grid scaled to
    the size: the rounded square (corner radius 22), the ring (center 36, 50; radius 21; 10
    wide), the arcs on the same center (radius 35 from -30 to 30 degrees, radius 47.5 from -26
    to 26; 7 wide, with round ends).

.EXAMPLE
    powershell -NoProfile -File packaging\msix\draw-logo.ps1 -Size 150 -Path Square150x150Logo.png
#>
param(
    [Parameter(Mandatory = $true)][int]$Size,
    [Parameter(Mandatory = $true)][string]$Path
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$scale = $Size / 100.0
$navy = [System.Drawing.Color]::FromArgb(255, 0x14, 0x21, 0x3D)
$white = [System.Drawing.Color]::FromArgb(255, 0xFF, 0xFF, 0xFF)
$amber = [System.Drawing.Color]::FromArgb(255, 0xFC, 0xA3, 0x11)

$bitmap = New-Object System.Drawing.Bitmap $Size, $Size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
$graphics.Clear([System.Drawing.Color]::Transparent)

# The rounded square.
$corner = 2 * 22 * $scale
$square = New-Object System.Drawing.Drawing2D.GraphicsPath
$square.AddArc(0, 0, $corner, $corner, 180, 90)
$square.AddArc($Size - $corner, 0, $corner, $corner, 270, 90)
$square.AddArc($Size - $corner, $Size - $corner, $corner, $corner, 0, 90)
$square.AddArc(0, $Size - $corner, $corner, $corner, 90, 90)
$square.CloseFigure()
$fill = New-Object System.Drawing.SolidBrush $navy
$graphics.FillPath($fill, $square)

# An arc of the circle of radius $r around (36, 50), from $start degrees, $sweep long,
# clockwise, $width wide with round ends.
function Draw-Arc($color, [double]$width, [double]$r, [double]$start, [double]$sweep) {
    $pen = New-Object System.Drawing.Pen $color, ([single]($width * $scale))
    $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $box = New-Object System.Drawing.RectangleF ([single]((36 - $r) * $scale)), ([single]((50 - $r) * $scale)), ([single](2 * $r * $scale)), ([single](2 * $r * $scale))
    $graphics.DrawArc($pen, $box, [single]$start, [single]$sweep)
    $pen.Dispose()
}
Draw-Arc $white 10 21 0 360
Draw-Arc $amber 7 35 -30 60
Draw-Arc $amber 7 47.5 -26 52

$folder = Split-Path -Parent $Path
if ($folder -and -not (Test-Path $folder)) { New-Item -ItemType Directory -Force $folder | Out-Null }
$bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
$fill.Dispose(); $square.Dispose(); $graphics.Dispose(); $bitmap.Dispose()

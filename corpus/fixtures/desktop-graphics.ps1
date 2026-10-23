<#
.SYNOPSIS
    Deterministic native desktop fixture for small pictures (WinForms): what JAWS's cursor says
    of graphics its screen capture keeps (DEFAULT.JCF [OSM] GraphicMinX to GraphicMaxY, 10 to
    64 pixels a side) at each IncludeGraphics value.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). Top to bottom:
    a label; a row of three 32-pixel picture buttons without names or text (a square, a
    circle, a triangle); a 48-pixel picture of a square beside a label; a single-line edit,
    which has the focus. The file is ASCII only. Do not change it without re-recording every
    scenario that uses it (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$font = New-Object System.Drawing.Font('Segoe UI', 14)
$white = [System.Drawing.Color]::White

function Icon([string]$shape, [int]$size) {
    $bitmap = New-Object System.Drawing.Bitmap($size, $size)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.Clear($white)
    $brush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::DarkBlue)
    $inset = [int]($size / 6)
    $side = $size - 2 * $inset
    switch ($shape) {
        'square' { $graphics.FillRectangle($brush, $inset, $inset, $side, $side) }
        'circle' { $graphics.FillEllipse($brush, $inset, $inset, $side, $side) }
        'triangle' {
            $points = @(
                (New-Object System.Drawing.Point([int]($size / 2), $inset)),
                (New-Object System.Drawing.Point(($size - $inset), ($size - $inset))),
                (New-Object System.Drawing.Point($inset, ($size - $inset))))
            $graphics.FillPolygon($brush, $points)
        }
    }
    $brush.Dispose()
    $graphics.Dispose()
    $bitmap
}

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Graphics Fixture'
$form.ClientSize = New-Object System.Drawing.Size(560, 300)
$form.StartPosition = 'CenterScreen'
$form.BackColor = $white

$top = New-Object System.Windows.Forms.Label -Property @{ Text = 'Tools below'; AutoSize = $true }
$top.Font = $font
$top.Location = New-Object System.Drawing.Point(20, 16)
$controls = @($top)

$left = 20
$index = 1
foreach ($shape in 'square', 'circle', 'triangle') {
    $button = New-Object System.Windows.Forms.Button -Property @{ TabIndex = $index; FlatStyle = 'Flat' }
    $button.Location = New-Object System.Drawing.Point($left, 64)
    $button.Size = New-Object System.Drawing.Size(40, 40)
    $button.Image = Icon $shape 32
    $controls += $button
    $left += 56
    $index += 1
}

$picture = New-Object System.Windows.Forms.PictureBox
$picture.Location = New-Object System.Drawing.Point(20, 128)
$picture.Size = New-Object System.Drawing.Size(48, 48)
$picture.Image = Icon 'square' 48
$beside = New-Object System.Windows.Forms.Label -Property @{ Text = 'Status'; AutoSize = $true }
$beside.Font = $font
$beside.Location = New-Object System.Drawing.Point(84, 138)
$controls += $picture, $beside

$box = New-Object System.Windows.Forms.TextBox -Property @{ Text = 'Focus here'; TabIndex = 0 }
$box.Font = $font
$box.Location = New-Object System.Drawing.Point(20, 210)
$box.Size = New-Object System.Drawing.Size(300, 36)
$controls += $box

$form.Controls.AddRange($controls)

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $box.Focus() | Out-Null; $box.Select(0, 0); $form.Activate() })
[void]$form.ShowDialog()

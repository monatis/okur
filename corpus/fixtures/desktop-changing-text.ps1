<#
.SYNOPSIS
    Deterministic native desktop fixture for Background OCR (WinForms): a picture of text that
    changes every two seconds.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). Top to bottom:
    a picture of a fixed line ("Status ready"), a picture of a counter ("Count 1", "Count 2",
    ...) drawn again every two seconds from the window's first showing, and a single-line edit,
    which has the focus. The pictures are pixels only, in no accessibility API. The file is
    ASCII only. Do not change it without re-recording every scenario that uses it (the manifest
    pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$font = New-Object System.Drawing.Font('Segoe UI', 16)
$black = [System.Drawing.Color]::Black
$white = [System.Drawing.Color]::White

function Picture([string]$text, [int]$width, [int]$height) {
    $bitmap = New-Object System.Drawing.Bitmap($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.Clear($white)
    $graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAlias
    $brush = New-Object System.Drawing.SolidBrush($black)
    $graphics.DrawString($text, $font, $brush, 4, 4)
    $brush.Dispose()
    $graphics.Dispose()
    $bitmap
}

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Changing Text Fixture'
$form.ClientSize = New-Object System.Drawing.Size(640, 260)
$form.StartPosition = 'CenterScreen'
$form.BackColor = $white

$status = New-Object System.Windows.Forms.PictureBox
$status.Location = New-Object System.Drawing.Point(20, 16)
$status.Size = New-Object System.Drawing.Size(600, 48)
$status.Image = Picture 'Status ready' 600 48

$counter = New-Object System.Windows.Forms.PictureBox
$counter.Location = New-Object System.Drawing.Point(20, 96)
$counter.Size = New-Object System.Drawing.Size(600, 48)
$script:count = 1
$counter.Image = Picture "Count $($script:count)" 600 48

$box = New-Object System.Windows.Forms.TextBox -Property @{ Text = 'Focus here'; TabIndex = 0 }
$box.Font = $font
$box.Location = New-Object System.Drawing.Point(20, 180)
$box.Size = New-Object System.Drawing.Size(300, 40)

$form.Controls.AddRange(@($status, $counter, $box))

$tick = New-Object System.Windows.Forms.Timer
$tick.Interval = 2000
$tick.Add_Tick({
    $script:count += 1
    $old = $counter.Image
    $counter.Image = Picture "Count $($script:count)" 600 48
    if ($old) { $old.Dispose() }
})

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $box.Focus() | Out-Null; $box.Select(0, 0); $form.Activate(); $tick.Start() })
[void]$form.ShowDialog()

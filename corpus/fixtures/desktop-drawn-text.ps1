<#
.SYNOPSIS
    Deterministic native desktop fixture for text a screen reader gets in different ways
    (WinForms): what JAWS's cursor reads of each, and what OCR adds.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). One row each,
    top to bottom:
    - a label, its text drawn by GDI (TextRenderer) and named in UI Automation;
    - a panel painted with GDI+ (Graphics.DrawString), its text in no accessibility API;
    - a panel painted with GDI (TextRenderer.DrawText), its text in no accessibility API;
    - a picture of text: pixels only;
    - a button whose only content is a picture of a word, without an accessible name;
    - a single-line edit, which has the focus.
    The file is ASCII only. Do not change it without re-recording every scenario that uses it
    (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$font = New-Object System.Drawing.Font('Segoe UI', 16)
$black = [System.Drawing.Color]::Black
$white = [System.Drawing.Color]::White

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Drawn Text Fixture'
$form.ClientSize = New-Object System.Drawing.Size(640, 420)
$form.StartPosition = 'CenterScreen'
$form.BackColor = $white

$label = New-Object System.Windows.Forms.Label -Property @{ Text = 'Label drawn by GDI'; AutoSize = $true }
$label.Font = $font
$label.Location = New-Object System.Drawing.Point(20, 16)

$gdiPlus = New-Object System.Windows.Forms.Panel
$gdiPlus.Location = New-Object System.Drawing.Point(20, 76)
$gdiPlus.Size = New-Object System.Drawing.Size(600, 40)
$gdiPlus.Add_Paint({
    param($sender, $e)
    $brush = New-Object System.Drawing.SolidBrush($black)
    $e.Graphics.DrawString('Painted with GDI plus', $font, $brush, 0, 0)
    $brush.Dispose()
})

$gdi = New-Object System.Windows.Forms.Panel
$gdi.Location = New-Object System.Drawing.Point(20, 136)
$gdi.Size = New-Object System.Drawing.Size(600, 40)
$gdi.Add_Paint({
    param($sender, $e)
    [System.Windows.Forms.TextRenderer]::DrawText($e.Graphics, 'Painted with GDI text out', $font, (New-Object System.Drawing.Point(0, 0)), $black)
})

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

$picture = New-Object System.Windows.Forms.PictureBox
$picture.Location = New-Object System.Drawing.Point(20, 196)
$picture.Size = New-Object System.Drawing.Size(600, 48)
$picture.Image = Picture 'Picture of text forty two' 600 48

$button = New-Object System.Windows.Forms.Button -Property @{ TabIndex = 1 }
$button.Location = New-Object System.Drawing.Point(20, 264)
$button.Size = New-Object System.Drawing.Size(120, 56)
$button.Image = Picture 'Start' 100 44
$button.Add_Click({ $form.Text = 'SR Drawn Text Fixture - started' })

$box = New-Object System.Windows.Forms.TextBox -Property @{ Text = 'Focus here'; TabIndex = 0 }
$box.Font = $font
$box.Location = New-Object System.Drawing.Point(20, 340)
$box.Size = New-Object System.Drawing.Size(300, 40)

$form.Controls.AddRange(@($label, $gdiPlus, $gdi, $picture, $button, $box))

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $box.Focus() | Out-Null; $box.Select(0, 0); $form.Activate() })
[void]$form.ShowDialog()

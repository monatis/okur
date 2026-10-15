<#
.SYNOPSIS
    Deterministic native desktop fixture: a RichEdit control holding a few lines of code.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "SR RichEdit Fixture"). A WinForms RichTextBox is a RichEdit control, the family Windows
    11 Notepad and WordPad build on: it separates lines with a bare carriage return, unlike
    the classic Win32 edit. Consolas 11 point, left aligned. ASCII only. Do not change it
    without re-recording every scenario that uses it (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR RichEdit Fixture'
$form.ClientSize = New-Object System.Drawing.Size(560, 220)
$form.StartPosition = 'CenterScreen'

$box = New-Object System.Windows.Forms.RichTextBox -Property @{ WordWrap = $false; TabIndex = 0 }
$box.Font = New-Object System.Drawing.Font('Consolas', 11)
$box.Location = New-Object System.Drawing.Point(20, 20)
$box.Size = New-Object System.Drawing.Size(520, 180)
$box.Text = (@(
        'fn main() {',
        '    let total = price * 1.5;',
        '',
        '    println!("{total}");',
        '}'
    ) -join "`n")
$form.Controls.Add($box)

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $box.Focus() | Out-Null; $box.Select(0, 0); $form.Activate() })
[void]$form.ShowDialog()

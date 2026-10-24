<#
.SYNOPSIS
    Deterministic native desktop fixture: a RichEdit control holding a path, a web address and
    an email address on one line.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "SR RichEdit Paths"). A WinForms RichTextBox is a RichEdit control, the family Windows 11
    Notepad builds on, whose own word unit stops at punctuation: words with punctuation in them
    read by Ctrl+Right and Say Word at each punctuation level, as desktop-notepad-paths.ps1 has
    them in a classic Win32 edit. Consolas 11 point, left aligned. ASCII only; the fixtures'
    "Ada" and a reserved domain only. Do not change it without re-recording every scenario that
    uses it (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR RichEdit Paths'
$form.ClientSize = New-Object System.Drawing.Size(640, 120)
$form.StartPosition = 'CenterScreen'

$box = New-Object System.Windows.Forms.RichTextBox -Property @{ WordWrap = $false; TabIndex = 0 }
$box.Font = New-Object System.Drawing.Font('Consolas', 11)
$box.Location = New-Object System.Drawing.Point(20, 20)
$box.Size = New-Object System.Drawing.Size(600, 80)
$box.Text = 'C:\Users\Ada\Downloads or https://example.org/a-b or ada@example.org'
$form.Controls.Add($box)

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $box.Focus() | Out-Null; $box.Select(0, 0); $form.Activate() })
[void]$form.ShowDialog()

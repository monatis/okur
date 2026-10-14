<#
.SYNOPSIS
    Deterministic native desktop fixture for reviewing text character by character (WinForms).

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). One
    multiline edit holds three lines: every ASCII punctuation mark, a line of code-like text
    (tab, doubled space, decimal and dotted names), and non-ASCII symbols. The file is ASCII
    only, so Windows PowerShell reads it the same in every code page; the non-ASCII
    characters are built from their code points. Do not change it without re-recording every
    scenario that uses it (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$punctuation = 'a !"#$%&''()*+,-./:;<=>?@[\]^_`{|}~ B'
$code = [string][char]9 + 'x  = 1.5; a.b end.'
$symbols = -join (@(0xE9, 0x20AC, 0x2014, 0x201C, 0x201D, 0x2018, 0x2019, 0x2026, 0x2022, 0x2192,
        0xA9, 0xB0, 0xBD, 0xE7, 0x15F, 0x11F, 0x131, 0x130, 0xD7, 0xF7, 0xB1, 0x2260, 0x2264,
        0x3BB, 0xA0) | ForEach-Object { [string][char]$_ }) + [char]::ConvertFromUtf32(0x1F600)

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Editor Fixture'
$form.ClientSize = New-Object System.Drawing.Size(560, 200)
$form.StartPosition = 'CenterScreen'

$label = New-Object System.Windows.Forms.Label -Property @{ Text = '&Code'; TabIndex = 0; AutoSize = $true }
$label.Location = New-Object System.Drawing.Point(20, 20)
$box = New-Object System.Windows.Forms.TextBox -Property @{
    Multiline = $true; WordWrap = $false; ScrollBars = 'Both'; TabIndex = 1; AcceptsTab = $true
}
$box.Font = New-Object System.Drawing.Font('Consolas', 11)
$box.Location = New-Object System.Drawing.Point(20, 44)
$box.Size = New-Object System.Drawing.Size(520, 130)
$box.Text = ($punctuation, $code, $symbols) -join "`r`n"
$form.Controls.AddRange(@($label, $box))

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $box.Focus() | Out-Null; $box.Select(0, 0); $form.Activate() })
[void]$form.ShowDialog()

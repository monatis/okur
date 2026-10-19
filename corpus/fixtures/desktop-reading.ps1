<#
.SYNOPSIS
    Deterministic native desktop fixture for JAWS's reading settings (WinForms).

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). One
    multiline edit holds the 18 lines of reading-settings.html: punctuation in sentences,
    addresses, paths, math and code; quotes and dashes; repeated symbols; mixed case and words
    with digits; numbers and dates; capitals; spelling; a blank, an indented and a tabbed line;
    a paragraph for Say All; misspelled words. The file is ASCII only, so Windows PowerShell
    reads it the same in every code page; the non-ASCII characters are built from their code
    points. Do not change it without re-recording every scenario that uses it (the manifest
    pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

function Chars([int[]]$codes) { -join ($codes | ForEach-Object { [string][char]$_ }) }

$lines = @(
    'Hello, world. Is it done? Yes! Wait; maybe: no.'
    'Mail ada@example.org or see https://example.org/a-b_c?x=1&y=2#top'
    'C:\Users\Ada\notes.txt and /usr/local/bin'
    'Price $9.95, 50% off; total = 3 * 4 + 1 - 2 / 5 ^ 2 ~ 7'
    'fn main() { let v = [1, 2]; v.len() } // done <ok> | a && b'
    ('"double" ''single'' ' + (Chars 0x201C) + 'curly' + (Chars 0x201D) + ' ' + (Chars 0x2018) + 'curly' +
        (Chars 0x2019) + ' ' + (Chars 0x2014) + ' em ' + (Chars 0x2013) + ' en ' + (Chars 0x2026) +
        ' ellipsis ' + (Chars 0x2022) + ' bullet')
    '===== ----- ..... !!!!! ##### ***'
    'MenuBar getElementById XMLHttpRequest iPhone McDonald PowerShell'
    'System32 x86_64 COVID19 123RON 1st 2nd 3rd'
    '7 42 1234 54321 123456 1-800-444-4443 2.5 1,000,000 10:30'
    '03-16-00 3/2/2002 1/3 $0.02 $1,250.50'
    'Ada READ me B x'
    ('JAWS sister-in-law 2:00 P.M. aB9 ' + (Chars 0xE9) + ' ' + (Chars 0x20AC))
    ''
    '    four spaces'
    ([string][char]9 + 'one tab')
    'Say All reads this. It has three sentences. The last one ends here.'
    'teh quick brown fxo'
)

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Reading Fixture'
$form.ClientSize = New-Object System.Drawing.Size(760, 460)
$form.StartPosition = 'CenterScreen'

$label = New-Object System.Windows.Forms.Label -Property @{ Text = '&Text'; TabIndex = 0; AutoSize = $true }
$label.Location = New-Object System.Drawing.Point(20, 20)
$box = New-Object System.Windows.Forms.TextBox -Property @{
    Multiline = $true; WordWrap = $false; ScrollBars = 'Both'; TabIndex = 1; AcceptsTab = $true
}
$box.Font = New-Object System.Drawing.Font('Consolas', 11)
$box.Location = New-Object System.Drawing.Point(20, 44)
$box.Size = New-Object System.Drawing.Size(720, 396)
$box.Text = $lines -join "`r`n"
$form.Controls.AddRange(@($label, $box))

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $box.Focus() | Out-Null; $box.Select(0, 0); $form.Activate() })
[void]$form.ShowDialog()

<#
.SYNOPSIS
    Deterministic native desktop fixture for tool tips (WinForms): two buttons, each with a
    tool tip shown when the mouse rests on it.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). Left to right:
    a "Save" button, which has the focus, its tool tip "Save the document (Ctrl+S)"; a "Print"
    button, its tool tip "Print the document". The tool tips show half a second after the mouse
    comes to rest on a button (tooltips_class32, the system's own). The file is ASCII only. Do
    not change it without re-recording every scenario that uses it (the manifest pins its
    SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Tool Tip Fixture'
$form.ClientSize = New-Object System.Drawing.Size(420, 140)
$form.StartPosition = 'CenterScreen'
$form.Font = New-Object System.Drawing.Font('Segoe UI', 12)

$save = New-Object System.Windows.Forms.Button -Property @{ Text = 'Save'; TabIndex = 0 }
$save.Location = New-Object System.Drawing.Point(30, 40)
$save.Size = New-Object System.Drawing.Size(150, 50)

$print = New-Object System.Windows.Forms.Button -Property @{ Text = 'Print'; TabIndex = 1 }
$print.Location = New-Object System.Drawing.Point(230, 40)
$print.Size = New-Object System.Drawing.Size(150, 50)

$tips = New-Object System.Windows.Forms.ToolTip
$tips.InitialDelay = 500
$tips.AutoPopDelay = 10000
$tips.SetToolTip($save, 'Save the document (Ctrl+S)')
$tips.SetToolTip($print, 'Print the document')

$form.Controls.AddRange(@($save, $print))

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $save.Focus() | Out-Null; $form.Activate() })
[void]$form.ShowDialog()

<#
.SYNOPSIS
    Deterministic native desktop fixture for tool tips (WinForms): two buttons, each with a
    tool tip the fixture shows itself when the button takes the focus.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). Left to right:
    a "Save" button, which has the focus, its tool tip "Save the document (Ctrl+S)"; a "Print"
    button, its tool tip "Print the document". A button taking the focus shows its tool tip
    300 ms later, below it, for four seconds, wherever the mouse is: WinForms' own tool tip
    window ("WindowsForms10.tooltips_class32..."), not comctl32's tooltips_class32 itself. The
    file is ASCII only. Do not change it without re-recording every scenario that uses it (the
    manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Tool Tip Fixture'
$form.ClientSize = New-Object System.Drawing.Size(420, 160)
$form.StartPosition = 'CenterScreen'
$form.Font = New-Object System.Drawing.Font('Segoe UI', 12)

$save = New-Object System.Windows.Forms.Button -Property @{ Text = 'Save'; TabIndex = 0 }
$save.Location = New-Object System.Drawing.Point(30, 30)
$save.Size = New-Object System.Drawing.Size(150, 50)

$print = New-Object System.Windows.Forms.Button -Property @{ Text = 'Print'; TabIndex = 1 }
$print.Location = New-Object System.Drawing.Point(230, 30)
$print.Size = New-Object System.Drawing.Size(150, 50)

$tips = New-Object System.Windows.Forms.ToolTip
$tips.SetToolTip($save, 'Save the document (Ctrl+S)')
$tips.SetToolTip($print, 'Print the document')

# The button whose tool tip shows next, 300 ms after it took the focus.
$script:tipFor = $null
$showTip = New-Object System.Windows.Forms.Timer
$showTip.Interval = 300
$showTip.Add_Tick({
    $showTip.Stop()
    if ($script:tipFor) {
        $tips.Show($tips.GetToolTip($script:tipFor), $script:tipFor, 0, $script:tipFor.Height + 4, 4000)
    }
})
$save.Add_GotFocus({ $script:tipFor = $save; $showTip.Stop(); $showTip.Start() })
$print.Add_GotFocus({ $script:tipFor = $print; $showTip.Stop(); $showTip.Start() })

$form.Controls.AddRange(@($save, $print))

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $save.Focus() | Out-Null; $form.Activate() })
[void]$form.ShowDialog()

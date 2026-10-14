<#
.SYNOPSIS
    Deterministic native desktop fixture for dialogs and window switching (WinForms).

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). The main
    window offers a message box, a second (modeless) window, and a modal dialog with text.
    Every run builds the identical windows, texts and focus order. Do not change it without
    re-recording every scenario that uses it (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Dialog Fixture'
$form.ClientSize = New-Object System.Drawing.Size(420, 220)
$form.StartPosition = 'CenterScreen'

function Add-Button([string]$text, [int]$y, [int]$tab) {
    $b = New-Object System.Windows.Forms.Button -Property @{ Text = $text; TabIndex = $tab }
    $b.Location = New-Object System.Drawing.Point(20, $y)
    $b.Size = New-Object System.Drawing.Size(180, 30)
    $form.Controls.Add($b)
    $b
}

$save = Add-Button '&Save...' 20 1
$second = Add-Button 'Open &second window' 60 2
$about = Add-Button '&About...' 100 3

$save.Add_Click({
        [void][System.Windows.Forms.MessageBox]::Show(
            $form, 'Save changes to the document?', 'SR Dialog Fixture',
            [System.Windows.Forms.MessageBoxButtons]::YesNoCancel,
            [System.Windows.Forms.MessageBoxIcon]::Question)
    })

$second.Add_Click({
        $w = New-Object System.Windows.Forms.Form
        $w.Text = 'SR Second Window'
        $w.ClientSize = New-Object System.Drawing.Size(300, 120)
        $w.StartPosition = 'CenterScreen'
        $note = New-Object System.Windows.Forms.TextBox -Property @{ Text = 'Second window notes'; TabIndex = 1 }
        $note.Location = New-Object System.Drawing.Point(20, 20)
        $note.Size = New-Object System.Drawing.Size(250, 22)
        $close = New-Object System.Windows.Forms.Button -Property @{ Text = 'Close'; TabIndex = 2 }
        $close.Location = New-Object System.Drawing.Point(20, 60)
        $close.Add_Click({ $w.Close() })
        $w.Controls.AddRange(@($note, $close))
        $w.Show()
    })

$about.Add_Click({
        $d = New-Object System.Windows.Forms.Form
        $d.Text = 'About SR Dialog Fixture'
        $d.ClientSize = New-Object System.Drawing.Size(320, 140)
        $d.FormBorderStyle = 'FixedDialog'
        $d.StartPosition = 'CenterParent'
        $d.MinimizeBox = $false
        $d.MaximizeBox = $false
        $text = New-Object System.Windows.Forms.Label -Property @{ Text = 'Version 1.0. A fixture for screen reader tests.'; AutoSize = $true }
        $text.Location = New-Object System.Drawing.Point(20, 20)
        $ok = New-Object System.Windows.Forms.Button -Property @{ Text = 'OK'; DialogResult = 'OK' }
        $ok.Location = New-Object System.Drawing.Point(20, 80)
        $d.Controls.AddRange(@($text, $ok))
        $d.AcceptButton = $ok
        [void]$d.ShowDialog($form)
    })

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $save.Focus() | Out-Null; $form.Activate() })
[void]$form.ShowDialog()

<#
.SYNOPSIS
    Deterministic native desktop fixture for a Win32 dialog box: a message box with Yes, No and
    Cancel, No its default button (WinForms over the Win32 MessageBox).

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "SR Message Box Fixture"). "Remove the draft" opens a real Win32 message box (class #32770),
    titled "SR Confirm", asking "Remove the draft sr-draft.txt?", with Yes, No and Cancel and No
    as its default button. Its answer is shown in the label below the button. ASCII only. Do not
    change it without re-recording every scenario that uses it (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Message Box Fixture'
$form.ClientSize = New-Object System.Drawing.Size(360, 140)
$form.StartPosition = 'CenterScreen'

$remove = New-Object System.Windows.Forms.Button -Property @{ Text = '&Remove the draft'; TabIndex = 0 }
$remove.Location = New-Object System.Drawing.Point(20, 20)
$remove.Size = New-Object System.Drawing.Size(180, 30)

$answer = New-Object System.Windows.Forms.Label -Property @{ Text = 'No answer yet'; AutoSize = $true }
$answer.Location = New-Object System.Drawing.Point(20, 70)

$remove.Add_Click({
    $result = [System.Windows.Forms.MessageBox]::Show($form, 'Remove the draft sr-draft.txt?', 'SR Confirm',
        [System.Windows.Forms.MessageBoxButtons]::YesNoCancel,
        [System.Windows.Forms.MessageBoxIcon]::Question,
        [System.Windows.Forms.MessageBoxDefaultButton]::Button2)
    $answer.Text = "Answer: $result"
})

$form.Controls.AddRange(@($remove, $answer))

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $remove.Focus() | Out-Null; $form.Activate() })
[void]$form.ShowDialog()

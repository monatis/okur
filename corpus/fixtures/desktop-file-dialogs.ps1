<#
.SYNOPSIS
    Deterministic native desktop fixture for Windows' Open and Save As dialogs, opened in a
    folder of their own (WinForms).

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "SR File Dialog Fixture"). Makes %TEMP%\sr-dialogs anew on every run with two text files in
    it, then shows two buttons: "Open..." opens Windows' Open dialog and "Save As..." its Save As
    dialog, both set to that folder (the dialog's folder is set, not remembered) with a file name
    filled in (sr-dialog-notes.txt to open, sr-dialog-copy.txt to save), so a dialog shows these
    files and nothing of the machine's own folders. The file chosen is shown in the label below.
    ASCII only. Do not change it without re-recording every scenario that uses it (the manifest
    pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$folder = Join-Path $env:TEMP 'sr-dialogs'
Remove-Item -Recurse -Force $folder -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $folder | Out-Null
foreach ($name in 'sr-dialog-notes.txt', 'sr-dialog-plan.txt') {
    [System.IO.File]::WriteAllText((Join-Path $folder $name), "Dialog fixture`r`n", (New-Object System.Text.UTF8Encoding $false))
}

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR File Dialog Fixture'
$form.ClientSize = New-Object System.Drawing.Size(360, 160)
$form.StartPosition = 'CenterScreen'

$open = New-Object System.Windows.Forms.Button -Property @{ Text = '&Open...'; TabIndex = 0 }
$open.Location = New-Object System.Drawing.Point(20, 20)
$open.Size = New-Object System.Drawing.Size(150, 30)

$save = New-Object System.Windows.Forms.Button -Property @{ Text = '&Save As...'; TabIndex = 1 }
$save.Location = New-Object System.Drawing.Point(190, 20)
$save.Size = New-Object System.Drawing.Size(150, 30)

$chosen = New-Object System.Windows.Forms.Label -Property @{ Text = 'No file chosen'; AutoSize = $true }
$chosen.Location = New-Object System.Drawing.Point(20, 70)

$filter = 'Text Documents (*.txt)|*.txt|All Files (*.*)|*.*'
$open.Add_Click({
    $dialog = New-Object System.Windows.Forms.OpenFileDialog -Property @{
        InitialDirectory = $folder; Filter = $filter; FileName = 'sr-dialog-notes.txt'; RestoreDirectory = $true
    }
    if ($dialog.ShowDialog($form) -eq 'OK') { $chosen.Text = "Opened $([IO.Path]::GetFileName($dialog.FileName))" }
})
$save.Add_Click({
    $dialog = New-Object System.Windows.Forms.SaveFileDialog -Property @{
        InitialDirectory = $folder; Filter = $filter; FileName = 'sr-dialog-copy.txt'; RestoreDirectory = $true
    }
    if ($dialog.ShowDialog($form) -eq 'OK') { $chosen.Text = "Saved $([IO.Path]::GetFileName($dialog.FileName))" }
})

$form.Controls.AddRange(@($open, $save, $chosen))

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $open.Focus() | Out-Null; $form.Activate() })
[void]$form.ShowDialog()

<#
.SYNOPSIS
    Deterministic native desktop fixture for progress bars (WinForms): a Win32 progress bar
    that a button starts, on a fixed schedule.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). Top to bottom:
    a "Start" button, which has the focus, a label "Copying files" and the progress bar it
    names (msctls_progress32), at 0. Space on the button starts the schedule, from that moment:
    10 at 0.5 s, 20 at 1.0 s and 30 at 1.5 s (a burst within five seconds), 40 at 8 s and 100 at
    14 s. The button's text then reads "Done". A second Space does nothing more. The file is
    ASCII only. Do not change it without re-recording every scenario that uses it (the manifest
    pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$font = New-Object System.Drawing.Font('Segoe UI', 12)

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Progress Fixture'
$form.ClientSize = New-Object System.Drawing.Size(520, 200)
$form.StartPosition = 'CenterScreen'
$form.Font = $font

$start = New-Object System.Windows.Forms.Button -Property @{ Text = 'Start'; TabIndex = 0 }
$start.Location = New-Object System.Drawing.Point(20, 20)
$start.Size = New-Object System.Drawing.Size(140, 40)

$label = New-Object System.Windows.Forms.Label -Property @{ Text = 'Copying files'; TabIndex = 1 }
$label.Location = New-Object System.Drawing.Point(20, 80)
$label.Size = New-Object System.Drawing.Size(300, 28)

$bar = New-Object System.Windows.Forms.ProgressBar -Property @{ TabIndex = 2 }
$bar.Location = New-Object System.Drawing.Point(20, 115)
$bar.Size = New-Object System.Drawing.Size(460, 30)
$bar.Minimum = 0
$bar.Maximum = 100
$bar.Value = 0
# Without the theme's animation the value shown is the value set.
$bar.Style = [System.Windows.Forms.ProgressBarStyle]::Continuous
$bar.AccessibleName = 'Copying files'

$form.Controls.AddRange(@($start, $label, $bar))

# The schedule, in half-second ticks from the start: tick 1 is 0.5 s.
$schedule = @{ 1 = 10; 2 = 20; 3 = 30; 16 = 40; 28 = 100 }
$script:ticks = 0
$script:started = $false
$tick = New-Object System.Windows.Forms.Timer
$tick.Interval = 500
$tick.Add_Tick({
    $script:ticks += 1
    if ($schedule.ContainsKey($script:ticks)) {
        $bar.Value = $schedule[$script:ticks]
    }
    if ($script:ticks -ge 28) {
        $tick.Stop()
        $start.Text = 'Done'
    }
})
$start.Add_Click({
    if (-not $script:started) {
        $script:started = $true
        $tick.Start()
    }
})

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $start.Focus() | Out-Null; $form.Activate() })
[void]$form.ShowDialog()

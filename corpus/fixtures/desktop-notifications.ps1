<#
.SYNOPSIS
    Deterministic native desktop fixture for notifications: a Windows toast and a progress bar
    (WinForms).

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "SR Notification Fixture"). "Show notification" raises a Windows toast with fixed text,
    sent as Windows PowerShell (the application the fixture runs in); "Start progress" moves the
    progress bar from 0 to 100 in steps of 10, one every 500 ms, with its percentage in the
    label beside it. ASCII only. Do not change it without re-recording every scenario that uses
    it (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

function Show-Toast {
    [void][Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime]
    [void][Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime]
    $xml = New-Object Windows.Data.Xml.Dom.XmlDocument
    $xml.LoadXml('<toast><visual><binding template="ToastGeneric"><text>SR Fixture</text><text>Build finished in 12 seconds</text></binding></visual></toast>')
    $toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
    $toast.ExpirationTime = [DateTimeOffset]::Now.AddMinutes(2)
    $app = '{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe'
    [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($app).Show($toast)
}

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Notification Fixture'
$form.ClientSize = New-Object System.Drawing.Size(420, 180)
$form.StartPosition = 'CenterScreen'

$notify = New-Object System.Windows.Forms.Button -Property @{ Text = 'Show notification'; TabIndex = 0 }
$notify.Location = New-Object System.Drawing.Point(20, 20)
$notify.Size = New-Object System.Drawing.Size(180, 30)
$notify.Add_Click({ Show-Toast })

$start = New-Object System.Windows.Forms.Button -Property @{ Text = 'Start progress'; TabIndex = 1 }
$start.Location = New-Object System.Drawing.Point(20, 70)
$start.Size = New-Object System.Drawing.Size(180, 30)

$bar = New-Object System.Windows.Forms.ProgressBar -Property @{ Minimum = 0; Maximum = 100; Value = 0 }
$bar.Location = New-Object System.Drawing.Point(20, 120)
$bar.Size = New-Object System.Drawing.Size(280, 24)
$bar.AccessibleName = 'Copying files'

$percent = New-Object System.Windows.Forms.Label -Property @{ Text = '0%'; AutoSize = $true }
$percent.Location = New-Object System.Drawing.Point(310, 124)

$steps = New-Object System.Windows.Forms.Timer
$steps.Interval = 500
$steps.Add_Tick({
    if ($bar.Value -ge 100) { $steps.Stop(); return }
    $bar.Value = [Math]::Min(100, $bar.Value + 10)
    $percent.Text = "$($bar.Value)%"
})
$start.Add_Click({ $bar.Value = 0; $percent.Text = '0%'; $steps.Start() })

$form.Controls.AddRange(@($notify, $start, $bar, $percent))

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $notify.Focus() | Out-Null; $form.Activate() })
[void]$form.ShowDialog()

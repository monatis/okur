<#
.SYNOPSIS
    Deterministic native desktop fixture for an application's own UI Automation notification
    (WinForms).

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "SR UIA Notification Fixture"). "Send update" raises a UI Automation notification
    (UiaRaiseNotificationEvent, through WinForms' AccessibleObject.RaiseAutomationNotification
    of .NET Framework 4.8) with fixed text, as applications that do not use toasts notify
    (Teams). ASCII only. Do not change it without re-recording every scenario that uses it (the
    manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

# WinForms raises UIA notifications only with .NET Framework 4.8's accessibility features on;
# Windows PowerShell's configuration targets an older framework, so they are switched on
# before WinForms loads.
foreach ($switch in 'Switch.UseLegacyAccessibilityFeatures', 'Switch.UseLegacyAccessibilityFeatures.2', 'Switch.UseLegacyAccessibilityFeatures.3') {
    [AppContext]::SetSwitch($switch, $false)
}
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR UIA Notification Fixture'
$form.ClientSize = New-Object System.Drawing.Size(420, 120)
$form.StartPosition = 'CenterScreen'

$send = New-Object System.Windows.Forms.Button -Property @{ Text = 'Send update'; TabIndex = 0 }
$send.Location = New-Object System.Drawing.Point(20, 20)
$send.Size = New-Object System.Drawing.Size(180, 30)
$send.Add_Click({
    [void]$send.AccessibilityObject.RaiseAutomationNotification(
        [System.Windows.Forms.Automation.AutomationNotificationKind]::ActionCompleted,
        [System.Windows.Forms.Automation.AutomationNotificationProcessing]::ImportantMostRecent,
        'Upload finished: report.pdf')
})

$form.Controls.Add($send)

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $send.Focus() | Out-Null; $form.Activate() })
[void]$form.ShowDialog()

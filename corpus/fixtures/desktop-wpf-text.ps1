<#
.SYNOPSIS
    Deterministic native desktop fixture for a WPF window, whose text is drawn by WPF's own
    renderer (DirectX), not by GDI: what JAWS's cursor reads there.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). Top to bottom:
    a text block, a line of two text blocks side by side, a button, and a single-line edit,
    which has the focus. Every one is in UI Automation. The file is ASCII only. Do not change
    it without re-recording every scenario that uses it (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName PresentationFramework
Add-Type -AssemblyName PresentationCore
Add-Type -AssemblyName WindowsBase

$window = New-Object System.Windows.Window
$window.Title = 'SR WPF Text Fixture'
$window.Width = 640
$window.Height = 360
$window.WindowStartupLocation = 'CenterScreen'
$window.FontSize = 21

$panel = New-Object System.Windows.Controls.StackPanel
$panel.Margin = New-Object System.Windows.Thickness(20)

$first = New-Object System.Windows.Controls.TextBlock -Property @{ Text = 'WPF text block' }
$row = New-Object System.Windows.Controls.StackPanel -Property @{ Orientation = 'Horizontal' }
$row.Margin = New-Object System.Windows.Thickness(0, 12, 0, 12)
$left = New-Object System.Windows.Controls.TextBlock -Property @{ Text = 'Left words' }
$right = New-Object System.Windows.Controls.TextBlock -Property @{ Text = 'Right words' }
$right.Margin = New-Object System.Windows.Thickness(40, 0, 0, 0)
[void]$row.Children.Add($left)
[void]$row.Children.Add($right)
$button = New-Object System.Windows.Controls.Button -Property @{ Content = 'WPF button'; HorizontalAlignment = 'Left' }
$button.Padding = New-Object System.Windows.Thickness(12, 4, 12, 4)
$box = New-Object System.Windows.Controls.TextBox -Property @{ Text = 'Focus here'; Width = 300; HorizontalAlignment = 'Left' }
$box.Margin = New-Object System.Windows.Thickness(0, 12, 0, 0)

foreach ($child in @($first, $row, $button, $box)) { [void]$panel.Children.Add($child) }
$window.Content = $panel

$timer = New-Object System.Windows.Threading.DispatcherTimer
$timer.Interval = [TimeSpan]::FromSeconds($ExitAfterSeconds)
$timer.Add_Tick({ $window.Close() })
$timer.Start()

$window.Add_ContentRendered({ [void]$box.Focus(); $box.Select(0, 0); [void]$window.Activate() })
[void]$window.ShowDialog()

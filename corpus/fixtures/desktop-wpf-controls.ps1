<#
.SYNOPSIS
    Deterministic native desktop fixture for JAWS's Scan Cursor in a WPF window: how it reads
    each kind of control, one element at a time.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). Top to bottom:
    a text block; a button named "Save"; a button whose name ends in its role ("Open file
    button"); an unchecked check box; two radio buttons side by side, the first checked; a combo
    box with Red selected; a list box of two items; a single-line edit, which has the focus.
    Every one is in UI Automation. The file is ASCII only. Do not change it without
    re-recording every scenario that uses it (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName PresentationFramework
Add-Type -AssemblyName PresentationCore
Add-Type -AssemblyName WindowsBase

$window = New-Object System.Windows.Window
$window.Title = 'SR WPF Controls Fixture'
$window.Width = 640
$window.Height = 620
$window.WindowStartupLocation = 'CenterScreen'
$window.FontSize = 18

$panel = New-Object System.Windows.Controls.StackPanel
$panel.Margin = New-Object System.Windows.Thickness(20)
$gap = New-Object System.Windows.Thickness(0, 6, 0, 6)

$text = New-Object System.Windows.Controls.TextBlock -Property @{ Text = 'Plain text line' }
$save = New-Object System.Windows.Controls.Button -Property @{ Content = 'Save'; HorizontalAlignment = 'Left' }
$open = New-Object System.Windows.Controls.Button -Property @{ Content = 'Open file button'; HorizontalAlignment = 'Left' }
$check = New-Object System.Windows.Controls.CheckBox -Property @{ Content = 'Remember me' }
$sizes = New-Object System.Windows.Controls.StackPanel -Property @{ Orientation = 'Horizontal' }
$small = New-Object System.Windows.Controls.RadioButton -Property @{ Content = 'Small'; GroupName = 'size'; IsChecked = $true }
$large = New-Object System.Windows.Controls.RadioButton -Property @{ Content = 'Large'; GroupName = 'size' }
$large.Margin = New-Object System.Windows.Thickness(40, 0, 0, 0)
[void]$sizes.Children.Add($small)
[void]$sizes.Children.Add($large)
$colors = New-Object System.Windows.Controls.ComboBox -Property @{ Width = 200; HorizontalAlignment = 'Left' }
foreach ($color in 'Red', 'Green') { [void]$colors.Items.Add($color) }
$colors.SelectedIndex = 0
$list = New-Object System.Windows.Controls.ListBox -Property @{ Width = 200; HorizontalAlignment = 'Left' }
foreach ($item in 'First item', 'Second item') { [void]$list.Items.Add($item) }
$box = New-Object System.Windows.Controls.TextBox -Property @{ Text = 'Focus here'; Width = 300; HorizontalAlignment = 'Left' }

foreach ($child in @($text, $save, $open, $check, $sizes, $colors, $list, $box)) {
    $child.Margin = $gap
    [void]$panel.Children.Add($child)
}
$window.Content = $panel

$timer = New-Object System.Windows.Threading.DispatcherTimer
$timer.Interval = [TimeSpan]::FromSeconds($ExitAfterSeconds)
$timer.Add_Tick({ $window.Close() })
$timer.Start()

$window.Add_ContentRendered({ [void]$box.Focus(); $box.Select(0, 0); [void]$window.Activate() })
[void]$window.ShowDialog()

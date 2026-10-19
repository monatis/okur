<#
.SYNOPSIS
    Deterministic native desktop fixture for a data grid: three columns and three rows of a
    WinForms DataGridView.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "SR Data Grid Fixture"). The grid, labelled "Orders", has the columns Item, Quantity and Price
    and the rows Pens 12 2.50, Paper 3 4.75, Ink 1 18.00; it has the focus on its first cell when
    the window opens. A Close button follows it. ASCII only. Do not change it without
    re-recording every scenario that uses it (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Data Grid Fixture'
$form.ClientSize = New-Object System.Drawing.Size(460, 220)
$form.StartPosition = 'CenterScreen'

$grid = New-Object System.Windows.Forms.DataGridView -Property @{
    TabIndex = 0; AllowUserToAddRows = $false; AllowUserToDeleteRows = $false; ReadOnly = $true
    RowHeadersVisible = $false; SelectionMode = 'CellSelect'; MultiSelect = $false
}
$grid.AccessibleName = 'Orders'
$grid.Location = New-Object System.Drawing.Point(20, 20)
$grid.Size = New-Object System.Drawing.Size(420, 140)
foreach ($column in 'Item', 'Quantity', 'Price') { [void]$grid.Columns.Add($column, $column) }
[void]$grid.Rows.Add('Pens', '12', '2.50')
[void]$grid.Rows.Add('Paper', '3', '4.75')
[void]$grid.Rows.Add('Ink', '1', '18.00')

$close = New-Object System.Windows.Forms.Button -Property @{ Text = 'Close'; TabIndex = 1 }
$close.Location = New-Object System.Drawing.Point(20, 175)
$close.Add_Click({ $form.Close() })

$form.Controls.AddRange(@($grid, $close))

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $grid.Focus() | Out-Null; $grid.CurrentCell = $grid.Rows[0].Cells[0]; $form.Activate() })
[void]$form.ShowDialog()

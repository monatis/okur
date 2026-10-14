<#
.SYNOPSIS
    Deterministic native desktop fixture (WinForms / Win32 common controls).

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). Every run
    builds the identical window, control order, values and focus, so JAWS trials and our
    engine's event replay see the same UI. Do not change it without re-recording every
    scenario that uses it (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Desktop Fixture'
$form.ClientSize = New-Object System.Drawing.Size(560, 520)
$form.StartPosition = 'CenterScreen'

$menu = New-Object System.Windows.Forms.MenuStrip
foreach ($spec in @(
        @('&File', @('&New', '&Open...', 'E&xit')),
        @('&Edit', @('&Copy', '&Paste')),
        @('&Help', @('&About'))
    )) {
    $top = New-Object System.Windows.Forms.ToolStripMenuItem($spec[0])
    foreach ($item in $spec[1]) { [void]$top.DropDownItems.Add($item) }
    [void]$menu.Items.Add($top)
}
$form.MainMenuStrip = $menu
$form.Controls.Add($menu)

function Add-Control($control, [int]$x, [int]$y, [int]$w, [int]$h) {
    $control.Location = New-Object System.Drawing.Point($x, $y)
    $control.Size = New-Object System.Drawing.Size($w, $h)
    $form.Controls.Add($control)
    $control
}

$tab = 0
function Next-Tab { $script:tab++; $script:tab }

$nameLabel = Add-Control (New-Object System.Windows.Forms.Label -Property @{ Text = '&Name:' }) 12 36 90 20
$name = Add-Control (New-Object System.Windows.Forms.TextBox -Property @{ Text = 'Ada'; TabIndex = (Next-Tab) }) 110 34 200 22

$notesLabel = Add-Control (New-Object System.Windows.Forms.Label -Property @{ Text = 'N&otes:' }) 12 66 90 20
$notes = Add-Control (New-Object System.Windows.Forms.TextBox -Property @{ Multiline = $true; TabIndex = (Next-Tab) }) 110 64 200 44

$subscribe = Add-Control (New-Object System.Windows.Forms.CheckBox -Property @{ Text = '&Subscribe to updates'; Checked = $true; TabIndex = (Next-Tab) }) 12 116 220 22
$remember = Add-Control (New-Object System.Windows.Forms.CheckBox -Property @{ Text = '&Remember me'; TabIndex = (Next-Tab) }) 240 116 160 22

$plan = Add-Control (New-Object System.Windows.Forms.GroupBox -Property @{ Text = 'Plan'; TabIndex = (Next-Tab) }) 12 144 300 56
$i = 0
foreach ($p in 'Free', 'Pro', 'Team') {
    $radio = New-Object System.Windows.Forms.RadioButton -Property @{ Text = $p; Checked = ($p -eq 'Pro') }
    $radio.Location = New-Object System.Drawing.Point((10 + 90 * $i), 22)
    $radio.Size = New-Object System.Drawing.Size(80, 22)
    $plan.Controls.Add($radio)
    $i++
}

$countryLabel = Add-Control (New-Object System.Windows.Forms.Label -Property @{ Text = '&Country:' }) 12 212 90 20
$country = Add-Control (New-Object System.Windows.Forms.ComboBox -Property @{ DropDownStyle = 'DropDownList'; TabIndex = (Next-Tab) }) 110 210 200 22
[void]$country.Items.AddRange(@('Turkey', 'Germany', 'Brazil'))
$country.SelectedIndex = 1

$fruitLabel = Add-Control (New-Object System.Windows.Forms.Label -Property @{ Text = '&Fruit:' }) 330 36 90 20
$fruit = Add-Control (New-Object System.Windows.Forms.ListBox -Property @{ TabIndex = (Next-Tab) }) 330 58 200 60
[void]$fruit.Items.AddRange(@('Apples', 'Bananas', 'Cherries'))
$fruit.SelectedIndex = 1

$tree = Add-Control (New-Object System.Windows.Forms.TreeView -Property @{ TabIndex = (Next-Tab) }) 330 130 200 110
$docs = $tree.Nodes.Add('Documents')
[void]$docs.Nodes.Add('Reports')
[void]$docs.Nodes.Add('Invoices')
[void]$tree.Nodes.Add('Pictures')
$tree.SelectedNode = $docs

$list = Add-Control (New-Object System.Windows.Forms.ListView -Property @{ View = 'Details'; FullRowSelect = $true; TabIndex = (Next-Tab) }) 12 250 518 100
[void]$list.Columns.Add('Name', 200)
[void]$list.Columns.Add('Size', 100)
foreach ($row in @(@('budget.xlsx', '24 KB'), @('notes.txt', '2 KB'), @('photo.jpg', '1.2 MB'))) {
    $item = New-Object System.Windows.Forms.ListViewItem($row[0])
    [void]$item.SubItems.Add($row[1])
    [void]$list.Items.Add($item)
}
$list.Items[0].Selected = $true
$list.Items[0].Focused = $true

$tabs = Add-Control (New-Object System.Windows.Forms.TabControl -Property @{ TabIndex = (Next-Tab) }) 12 360 518 90
foreach ($t in 'General', 'Advanced') {
    $page = New-Object System.Windows.Forms.TabPage($t)
    $page.Controls.Add((New-Object System.Windows.Forms.Label -Property @{ Text = "$t settings"; AutoSize = $true; Location = (New-Object System.Drawing.Point(8, 8)) }))
    [void]$tabs.TabPages.Add($page)
}

$ok = Add-Control (New-Object System.Windows.Forms.Button -Property @{ Text = 'OK'; TabIndex = (Next-Tab) }) 284 462 76 28
$cancel = Add-Control (New-Object System.Windows.Forms.Button -Property @{ Text = 'Cancel'; TabIndex = (Next-Tab) }) 368 462 76 28
$apply = Add-Control (New-Object System.Windows.Forms.Button -Property @{ Text = '&Apply'; Enabled = $false; TabIndex = (Next-Tab) }) 452 462 76 28
$form.AcceptButton = $ok
$form.CancelButton = $cancel

$status = New-Object System.Windows.Forms.StatusStrip
[void]$status.Items.Add('Ready')
$form.Controls.Add($status)

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({ $name.Focus() | Out-Null; $form.Activate() })
[void]$form.ShowDialog()

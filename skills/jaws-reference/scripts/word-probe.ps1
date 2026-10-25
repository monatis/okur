# Research, never evidence: what Word's UI Automation reports of a fixture's table cells and
# paragraphs, for Okur's Word support. Opens a Word fixture (desktop-word.ps1 or
# desktop-word-formats.ps1), sends each probe's keys to its window, and after each prints:
#   - for a table cell: the cell's row and column, its row and column header items, and its
#     table's row and column headers (UI Automation's TableItem and Table patterns);
#   - the paragraph's text attributes (okur-lab text-attributes --unit 4), line spacing among
#     them (40040);
#   - with -Ia2, what the document window and Word's window offer through IAccessible2
#     (okur-lab ia2-probe): whether Word answers IA2 at all, and its text attributes there;
#   - with -Fixture ribbon, only the ribbon's focus, from MSAA's accFocus and UI Automation's
#     element for it (okur-lab msaa-focus): its help text, toggle and expand state; Alt, Down,
#     then Tab across the Home tab's first commands, and Escape back to the document.
# Then closes the fixture's window with Ctrl+W, after checking it is in front: Word's process
# can hold the user's own documents. Run on old-pc with no batch recording.
param([ValidateSet('word', 'formats', 'ribbon')][string]$Fixture = 'formats',
      [string]$Out = (Join-Path $env:USERPROFILE "sr-harness\probe\word-probe.txt"),
      [switch]$Ia2)
$lab = Join-Path $env:USERPROFILE 'sr-harness\bin\okur-lab.exe'
$fixtures = Join-Path $env:USERPROFILE 'corpus\fixtures'
$script, $title, $probes = switch ($Fixture) {
    'word' {
        'desktop-word.ps1', 'sr-word-fixture.docx - Word (Product Activation Failed)',
        @(@('Ctrl+Home'), @('Down', 'Down', 'Down', 'Down', 'Down', 'Down', 'Down'), @('Tab'), @('Tab'))
    }
    'ribbon' {
        'desktop-word.ps1', 'sr-word-fixture.docx - Word (Product Activation Failed)',
        (@(@('Alt'), @('Down')) + @(1..10 | ForEach-Object { , @('Tab') }) + @(, @('Escape', 'Escape', 'Escape')))
    }
    'formats' {
        'desktop-word-formats.ps1', 'sr-word-formats.docx - Word (Product Activation Failed)',
        @(@('Ctrl+Home'), @('Down', 'Down'), @('Tab'), @('Tab'), @('Ctrl+End', 'Up'), @('Up'))
    }
}
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type -Namespace WP -Name W -MemberDefinition '[DllImport("user32.dll")] public static extern System.IntPtr GetForegroundWindow(); [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(System.IntPtr h, System.Text.StringBuilder s, int n);'
function Front-Title {
    $sb = New-Object System.Text.StringBuilder 256
    [void][WP.W]::GetWindowText([WP.W]::GetForegroundWindow(), $sb, 256)
    $sb.ToString()
}
function Text-Of($element) {
    try {
        $text = $element.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
        $text.DocumentRange.GetText(100).Trim()
    } catch { $element.Current.Name }
}
# The table cell around the caret: its place and headers, as Word reports them.
function Cell-Report {
    $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
    try { $pattern = $focused.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern) } catch { return 'no text pattern' }
    $element = $pattern.GetSelection()[0].GetEnclosingElement()
    $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
    for ($i = 0; $i -lt 8 -and $element; $i++) {
        $item = $null
        try { $item = $element.GetCurrentPattern([System.Windows.Automation.TableItemPattern]::Pattern) } catch {}
        if ($item) {
            $rowHeaders = @($item.Current.GetRowHeaderItems() | ForEach-Object { $_.Current.Name })
            $columnHeaders = @($item.Current.GetColumnHeaderItems() | ForEach-Object { $_.Current.Name })
            $grid = $item.Current.ContainingGrid
            $table = $null
            try { $table = $grid.GetCurrentPattern([System.Windows.Automation.TablePattern]::Pattern) } catch {}
            $tableRows = if ($table) { @($table.Current.GetRowHeaders() | ForEach-Object { $_.Current.Name }) -join ', ' } else { 'no table pattern' }
            $tableColumns = if ($table) { @($table.Current.GetColumnHeaders() | ForEach-Object { $_.Current.Name }) -join ', ' } else { '' }
            $major = if ($table) { $table.Current.RowOrColumnMajor } else { '' }
            return "cell '$($element.Current.Name)' row $($item.Current.Row) column $($item.Current.Column) | row headers [$($rowHeaders -join ', ')] | column headers [$($columnHeaders -join ', ')] | table row headers [$tableRows] | table column headers [$tableColumns] | $major"
        }
        $element = $walker.GetParent($element)
    }
    'not in a table cell'
}
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $fixtures $script)
$lines = @()
for ($i = 0; $i -lt 40 -and (Front-Title) -ne $title; $i++) { Start-Sleep -Milliseconds 500; & $lab front $title | Out-Null }
if ((Front-Title) -ne $title) { "the fixture's window did not come to the front: $(Front-Title)" | Set-Content $Out; return }
Start-Sleep -Seconds 2
foreach ($keys in $probes) {
    & $lab keys --allow WINWORD @($keys + 'wait:1200') | Out-Null
    $lines += "== $($keys -join ',') =>"
    if ($Fixture -eq 'ribbon') { $lines += & $lab msaa-focus | ForEach-Object { "   $_" }; continue }
    $lines += '   ' + (Cell-Report)
    $lines += & $lab text-attributes --unit 4 | ForEach-Object { "   $_" }
    if ($Ia2) { $lines += & $lab ia2-probe | ForEach-Object { "   ia2: $_" } }
}
# The fixture's window alone: Ctrl+W closes its document, unchanged, without a question.
if ((Front-Title) -eq $title) { & $lab keys --allow WINWORD 'Ctrl+W' 'wait:1500' | Out-Null }
$lines += "closed; in front now: $(Front-Title)"
$lines | Set-Content $Out
$lines

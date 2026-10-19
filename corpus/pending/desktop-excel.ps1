<#
.SYNOPSIS
    Deterministic Excel fixture: a small table with a header row, a total row with a formula,
    and a second sheet, opened in Microsoft Excel.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "sr-excel-fixture.xlsx - Excel"). Builds the workbook with Excel's own automation into
    %TEMP% on every run (the same cells each time), closes that automation copy of Excel, then
    opens the file in Excel as a user would. ASCII only. Do not change it without re-recording
    every scenario that uses it (the manifest pins its SHA-256).
#>
$path = Join-Path $env:TEMP 'sr-excel-fixture.xlsx'
$excel = New-Object -ComObject Excel.Application
$excel.Visible = $false
$excel.DisplayAlerts = $false
$book = $excel.Workbooks.Add()
while ($book.Worksheets.Count -lt 2) { [void]$book.Worksheets.Add([Type]::Missing, $book.Worksheets.Item($book.Worksheets.Count)) }
$hours = $book.Worksheets.Item(1)
$hours.Name = 'Hours'
$rows = @(
    @('Branch', 'Weekdays', 'Weekends'),
    @('Central', 12, 8),
    @('Northside', 10, 6)
)
for ($r = 0; $r -lt $rows.Count; $r++) {
    for ($c = 0; $c -lt 3; $c++) {
        $hours.Cells.Item($r + 1, $c + 1).Value2 = $rows[$r][$c]
    }
}
$hours.Cells.Item(4, 1).Value2 = 'Total'
$hours.Cells.Item(4, 2).Formula = '=SUM(B2:B3)'
$hours.Cells.Item(4, 3).Formula = '=SUM(C2:C3)'
$notes = $book.Worksheets.Item(2)
$notes.Name = 'Notes'
$notes.Cells.Item(1, 1).Value2 = 'Hours change in April.'
[void]$hours.Activate()
[void]$hours.Cells.Item(1, 1).Select()
$book.SaveAs($path, 51)
$book.Close($false)
$excel.Quit()
[void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($excel)
Start-Process 'C:\Program Files\Microsoft Office\Office16\EXCEL.EXE' -ArgumentList "`"$path`""

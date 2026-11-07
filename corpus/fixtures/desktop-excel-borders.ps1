<#
.SYNOPSIS
    Deterministic Excel fixture: cells with borders of several styles, weights and colors, one
    of them filled and in a bold red font, each alone in its row, opened in Microsoft Excel.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block with
    "close_window", as desktop-excel.ps1). Writes the workbook itself, a SpreadsheetML package,
    to %TEMP%\sr-excel-borders.xlsx on every run, with no Excel automation, then opens it
    through Windows so that it opens in the Excel already running. Column A holds the cells, a
    blank row between each two, so that no border is shared with a neighbour: A1 none, A3 thin
    automatic on all four sides, A5 a thick red bottom, A7 a double top, A9 a dashed blue left,
    A11 a thin left, a medium green right, a dotted top and a double bottom, filled yellow, in
    bold red. ASCII only. Do not change it without re-recording every scenario that uses it (the
    manifest pins its SHA-256).
#>
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem

$path = Join-Path $env:TEMP 'sr-excel-borders.xlsx'
$main = 'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"'
$rel = 'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'

function Text([string]$ref, [string]$text, [int]$style) { "<c r=`"$ref`" s=`"$style`" t=`"inlineStr`"><is><t>$text</t></is></c>" }
function Row([int]$n, [string[]]$cells) { "<row r=`"$n`">" + ($cells -join '') + '</row>' }

$parts = [ordered]@{}
$parts['[Content_Types].xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
</Types>
"@
$parts['_rels/.rels'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>
"@
$parts['xl/workbook.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook $main $rel><bookViews><workbookView activeTab="0"/></bookViews><sheets><sheet name="Borders" sheetId="1" r:id="rId1"/></sheets></workbook>
"@
$parts['xl/_rels/workbook.xml.rels'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>
"@
# Excel's defaults (Calibri 11, General) and five borders: the cell format s="n" uses border n.
$auto = '<color auto="1"/>'
$borders = @(
    '<border><left/><right/><top/><bottom/><diagonal/></border>',
    "<border><left style=`"thin`">$auto</left><right style=`"thin`">$auto</right><top style=`"thin`">$auto</top><bottom style=`"thin`">$auto</bottom><diagonal/></border>",
    '<border><left/><right/><top/><bottom style="thick"><color rgb="FFFF0000"/></bottom><diagonal/></border>',
    "<border><left/><right/><top style=`"double`">$auto</top><bottom/><diagonal/></border>",
    '<border><left style="dashed"><color rgb="FF0000FF"/></left><right/><top/><bottom/><diagonal/></border>',
    "<border><left style=`"thin`">$auto</left><right style=`"medium`"><color rgb=`"FF00B050`"/></right><top style=`"dotted`">$auto</top><bottom style=`"double`">$auto</bottom><diagonal/></border>"
)
$xfs = @(
    '<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>',
    '<xf numFmtId="0" fontId="0" fillId="0" borderId="1" xfId="0" applyBorder="1"/>',
    '<xf numFmtId="0" fontId="0" fillId="0" borderId="2" xfId="0" applyBorder="1"/>',
    '<xf numFmtId="0" fontId="0" fillId="0" borderId="3" xfId="0" applyBorder="1"/>',
    '<xf numFmtId="0" fontId="0" fillId="0" borderId="4" xfId="0" applyBorder="1"/>',
    '<xf numFmtId="0" fontId="1" fillId="2" borderId="5" xfId="0" applyFont="1" applyFill="1" applyBorder="1"/>'
)
$parts['xl/styles.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet $main><fonts count="2"><font><sz val="11"/><name val="Calibri"/><family val="2"/></font><font><b/><sz val="11"/><color rgb="FFFF0000"/><name val="Calibri"/><family val="2"/></font></fonts><fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FFFFFF00"/><bgColor indexed="64"/></patternFill></fill></fills><borders count="$($borders.Count)">$($borders -join '')</borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="$($xfs.Count)">$($xfs -join '')</cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>
"@
$rows = (Row 1 @(Text 'A1' 'Plain' 0)) +
    (Row 3 @(Text 'A3' 'Boxed' 1)) +
    (Row 5 @(Text 'A5' 'Underlined' 2)) +
    (Row 7 @(Text 'A7' 'Double top' 3)) +
    (Row 9 @(Text 'A9' 'Dashed left' 4)) +
    (Row 11 @(Text 'A11' 'Mixed' 5))
$parts['xl/worksheets/sheet1.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet $main $rel><sheetViews><sheetView workbookViewId="0" tabSelected="1"><selection activeCell="A1" sqref="A1"/></sheetView></sheetViews><sheetFormatPr defaultRowHeight="15"/><cols><col min="1" max="1" width="14" customWidth="1"/></cols><sheetData>$rows</sheetData></worksheet>
"@

if (Test-Path $path) { Remove-Item -Force $path }
$zip = [System.IO.Compression.ZipFile]::Open($path, [System.IO.Compression.ZipArchiveMode]::Create)
$utf8 = New-Object System.Text.UTF8Encoding $false
foreach ($name in $parts.Keys) {
    $entry = $zip.CreateEntry($name)
    $stream = $entry.Open()
    $bytes = $utf8.GetBytes($parts[$name])
    $stream.Write($bytes, 0, $bytes.Length)
    $stream.Dispose()
}
$zip.Dispose()
(Get-Item $path).LastWriteTime = Get-Date -Year 2026 -Month 1 -Day 2 -Hour 3 -Minute 4 -Second 5 -Millisecond 0

Start-Process -FilePath $path

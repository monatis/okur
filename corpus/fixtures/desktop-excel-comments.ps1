<#
.SYNOPSIS
    Deterministic Excel fixture with a note: desktop-excel.ps1's workbook, its cell B2 with a note
    (Excel's legacy comment, "Includes Saturday mornings."), opened in Microsoft Excel.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block with
    "close_window": Excel's process can hold the user's own workbooks, so the recorder closes the
    fixture's window and never ends the process). Writes the workbook itself, a SpreadsheetML
    package, to %TEMP%\sr-excel-comments.xlsx on every run: the same cells each time, with no
    Excel automation, which hung on old-pc's unactivated Office as Word's did (2026-10-24). The
    total row's formulas carry their values, which Excel shows as they are: a calculation on
    opening would mark the workbook changed, and closing it would ask to save. Then opens the
    file as a user would, through Windows, so that it opens in the Excel already running (a new
    Excel process shows Office's Activation Wizard on old-pc, which took a trial's keys, round
    521e). ASCII only. Do not change it without re-recording
    every scenario that uses it (the manifest pins its SHA-256).
#>
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem

$path = Join-Path $env:TEMP 'sr-excel-comments.xlsx'
$main = 'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"'
$rel = 'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'

function Text([string]$ref, [string]$text) { "<c r=`"$ref`" t=`"inlineStr`"><is><t>$text</t></is></c>" }
function Number([string]$ref, [int]$value) { "<c r=`"$ref`"><v>$value</v></c>" }
function Formula([string]$ref, [string]$formula, [int]$value) { "<c r=`"$ref`"><f>$formula</f><v>$value</v></c>" }
function Row([int]$n, [string[]]$cells) { "<row r=`"$n`">" + ($cells -join '') + '</row>' }
function Sheet([string]$rows, [bool]$selected, [string]$tail = '') {
    $tab = if ($selected) { ' tabSelected="1"' } else { '' }
    @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet $main $rel><sheetViews><sheetView workbookViewId="0"$tab><selection activeCell="A1" sqref="A1"/></sheetView></sheetViews><sheetFormatPr defaultRowHeight="15"/><sheetData>$rows</sheetData>$tail</worksheet>
"@
}

$parts = [ordered]@{}
$parts['[Content_Types].xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="vml" ContentType="application/vnd.openxmlformats-officedocument.vmlDrawing"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
<Override PartName="/xl/worksheets/sheet2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
<Override PartName="/xl/comments1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml"/>
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
<workbook $main $rel><bookViews><workbookView activeTab="0"/></bookViews><sheets><sheet name="Hours" sheetId="1" r:id="rId1"/><sheet name="Notes" sheetId="2" r:id="rId2"/></sheets></workbook>
"@
$parts['xl/_rels/workbook.xml.rels'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>
"@
# Excel's own defaults: Calibri 11, General number format.
$parts['xl/styles.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet $main><fonts count="1"><font><sz val="11"/><name val="Calibri"/><family val="2"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>
"@
$hours = (Row 1 @((Text 'A1' 'Branch'), (Text 'B1' 'Weekdays'), (Text 'C1' 'Weekends'))) +
    (Row 2 @((Text 'A2' 'Central'), (Number 'B2' 12), (Number 'C2' 8))) +
    (Row 3 @((Text 'A3' 'Northside'), (Number 'B3' 10), (Number 'C3' 6))) +
    (Row 4 @((Text 'A4' 'Total'), (Formula 'B4' 'SUM(B2:B3)' 22), (Formula 'C4' 'SUM(C2:C3)' 14)))
$parts['xl/worksheets/sheet1.xml'] = Sheet $hours $true '<legacyDrawing r:id="rId1"/>'
# The note on B2: its text in comments1.xml, its hidden box in the sheet's VML drawing, as Excel
# writes a note.
$parts['xl/worksheets/_rels/sheet1.xml.rels'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing" Target="../drawings/vmlDrawing1.vml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="../comments1.xml"/>
</Relationships>
"@
$parts['xl/comments1.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<comments $main><authors><author>Lab</author></authors><commentList><comment ref="B2" authorId="0"><text><r><t>Includes Saturday mornings.</t></r></text></comment></commentList></comments>
"@
$parts['xl/drawings/vmlDrawing1.vml'] = @"
<xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:x="urn:schemas-microsoft-com:office:excel">
<o:shapelayout v:ext="edit"><o:idmap v:ext="edit" data="1"/></o:shapelayout>
<v:shapetype id="_x0000_t202" coordsize="21600,21600" o:spt="202" path="m,l,21600r21600,l21600,xe"><v:stroke joinstyle="miter"/><v:path gradientshapeok="t" o:connecttype="rect"/></v:shapetype>
<v:shape id="_x0000_s1025" type="#_x0000_t202" style="position:absolute;margin-left:107.25pt;margin-top:1.5pt;width:108pt;height:59.25pt;z-index:1;visibility:hidden" fillcolor="#ffffe1" o:insetmode="auto"><v:fill color2="#ffffe1"/><v:shadow on="t" color="black" obscured="t"/><v:path o:connecttype="none"/><v:textbox style="mso-direction-alt:auto"><div style="text-align:left"></div></v:textbox><x:ClientData ObjectType="Note"><x:MoveWithCells/><x:SizeWithCells/><x:Anchor>2, 15, 0, 2, 4, 15, 4, 4</x:Anchor><x:AutoFill>False</x:AutoFill><x:Row>1</x:Row><x:Column>1</x:Column></x:ClientData></v:shape>
</xml>
"@
$parts['xl/worksheets/sheet2.xml'] = Sheet (Row 1 @((Text 'A1' 'Hours change in April.'))) $false

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

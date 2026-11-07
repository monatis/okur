<#
.SYNOPSIS
    Deterministic Excel fixture: cells with hyperlinks and a sheet with manual page breaks,
    opened in Microsoft Excel.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block with
    "close_window", as desktop-excel.ps1). Writes the workbook itself, a SpreadsheetML package,
    to %TEMP%\sr-excel-links.xlsx on every run, with no Excel automation, then opens it through
    Windows so that it opens in the Excel already running. Sheet "Links": A1 "Home page" linked to
    https://example.com/, A2 "Docs" linked to https://example.org/docs, A3 "Plain", B30 "End";
    a manual page break above row 21 and one left of column D. ASCII only. Do not change it
    without re-recording every scenario that uses it (the manifest pins its SHA-256).
#>
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem

$path = Join-Path $env:TEMP 'sr-excel-links.xlsx'
$main = 'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"'
$rel = 'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'
$hyperlinkType = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink'

function Text([string]$ref, [string]$text) { "<c r=`"$ref`" t=`"inlineStr`"><is><t>$text</t></is></c>" }
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
<workbook $main $rel><bookViews><workbookView activeTab="0"/></bookViews><sheets><sheet name="Links" sheetId="1" r:id="rId1"/></sheets></workbook>
"@
$parts['xl/_rels/workbook.xml.rels'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>
"@
# Excel's own defaults: Calibri 11, General number format.
$parts['xl/styles.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet $main><fonts count="1"><font><sz val="11"/><name val="Calibri"/><family val="2"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>
"@
$rows = (Row 1 @(Text 'A1' 'Home page')) + (Row 2 @(Text 'A2' 'Docs')) + (Row 3 @(Text 'A3' 'Plain')) +
    (Row 30 @(Text 'B30' 'End'))
$parts['xl/worksheets/sheet1.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet $main $rel><sheetViews><sheetView workbookViewId="0" tabSelected="1"><selection activeCell="A1" sqref="A1"/></sheetView></sheetViews><sheetFormatPr defaultRowHeight="15"/><sheetData>$rows</sheetData><hyperlinks><hyperlink ref="A1" r:id="rId1"/><hyperlink ref="A2" r:id="rId2"/></hyperlinks><rowBreaks count="1" manualBreakCount="1"><brk id="20" max="16383" man="1"/></rowBreaks><colBreaks count="1" manualBreakCount="1"><brk id="3" max="1048575" man="1"/></colBreaks></worksheet>
"@
$parts['xl/worksheets/_rels/sheet1.xml.rels'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="$hyperlinkType" Target="https://example.com/" TargetMode="External"/>
<Relationship Id="rId2" Type="$hyperlinkType" Target="https://example.org/docs" TargetMode="External"/>
</Relationships>
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

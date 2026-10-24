<#
.SYNOPSIS
    Deterministic Word fixture: formatted words (bold, italic, underlined), a table whose first
    row is no header, and paragraphs at 1.5 and double line spacing, opened in Microsoft Word.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block with
    "close_window": Word's process can hold the user's own documents, so the recorder closes the
    fixture's window and never ends the process). Writes the document itself, a WordprocessingML
    package, to %TEMP%\sr-word-formats.docx on every run, as desktop-word.ps1 writes its own,
    with no Word automation. Then opens the file in Word as a user would. ASCII only. Do not
    change it without re-recording every scenario that uses it (the manifest pins its SHA-256).
#>
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem

$path = Join-Path $env:TEMP 'sr-word-formats.docx'
$main = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'

$parts = [ordered]@{}
$parts['[Content_Types].xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
<Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
</Types>
"@
$parts['_rels/.rels'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>
"@
$parts['word/_rels/document.xml.rels'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>
</Relationships>
"@
# Word's built-in styles, named as Word names them: Normal and Table Grid.
$parts['word/styles.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles $main>
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:cs="Calibri"/><w:sz w:val="22"/><w:lang w:val="en-US"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="160" w:line="259" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>
<w:style w:type="table" w:default="1" w:styleId="TableNormal"><w:name w:val="Normal Table"/><w:tblPr><w:tblInd w:w="0" w:type="dxa"/><w:tblCellMar><w:left w:w="108" w:type="dxa"/><w:right w:w="108" w:type="dxa"/></w:tblCellMar></w:tblPr></w:style>
<w:style w:type="table" w:styleId="TableGrid"><w:name w:val="Table Grid"/><w:basedOn w:val="TableNormal"/><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/></w:pPr><w:tblPr><w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/></w:tblBorders></w:tblPr></w:style>
</w:styles>
"@

# A document of Word 2013 and later: without it Word opens the file in Compatibility Mode.
$parts['word/settings.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings $main><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>
"@

# A run of text with its character properties.
function Run([string]$text, [string]$properties) {
    $rPr = if ($properties) { "<w:rPr>$properties</w:rPr>" } else { '' }
    "<w:r>$rPr<w:t xml:space=`"preserve`">$text</w:t></w:r>"
}
# A paragraph of runs, with its paragraph properties.
function Paragraph([string]$runs, [string]$properties) {
    $pPr = if ($properties) { "<w:pPr>$properties</w:pPr>" } else { '' }
    "<w:p>$pPr$runs</w:p>"
}
function Cell([string]$text) {
    "<w:tc><w:tcPr><w:tcW w:w=`"4675`" w:type=`"dxa`"/></w:tcPr>$(Paragraph (Run $text '') '')</w:tc>"
}
# A table whose first row is no header: its look leaves the first row's formatting off.
$table = '<w:tbl><w:tblPr><w:tblStyle w:val="TableGrid"/><w:tblW w:w="0" w:type="auto"/><w:tblLook w:val="0000" w:firstRow="0" w:lastRow="0" w:firstColumn="0" w:lastColumn="0" w:noHBand="1" w:noVBand="1"/></w:tblPr>' +
    '<w:tblGrid><w:gridCol w:w="4675"/><w:gridCol w:w="4675"/></w:tblGrid>' +
    "<w:tr>$(Cell 'Apples')$(Cell '3')</w:tr>" +
    "<w:tr>$(Cell 'Pears')$(Cell '5')</w:tr></w:tbl>"
$formatted = (Run 'Hello ' '') + (Run 'world' '<w:b/>') + (Run ' ' '') + (Run 'again' '<w:i/>') +
    (Run ' ' '') + (Run 'here' '<w:u w:val="single"/>') + (Run '.' '')
$body = @(
    (Paragraph $formatted ''),
    (Paragraph (Run 'Prices' '') ''),
    $table,
    (Paragraph (Run 'One and a half.' '') '<w:spacing w:line="360" w:lineRule="auto"/>'),
    (Paragraph (Run 'Double.' '') '<w:spacing w:line="480" w:lineRule="auto"/>'),
    (Paragraph (Run 'The end.' '') '')
) -join ''
$parts['word/document.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document $main><w:body>$body<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:body></w:document>
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
# A fixed date: File Explorer and Word's backstage say a file's date.
(Get-Item $path).LastWriteTime = Get-Date -Year 2026 -Month 1 -Day 2 -Hour 3 -Minute 4 -Second 5 -Millisecond 0

Start-Process 'C:\Program Files\Microsoft Office\Office16\WINWORD.EXE' -ArgumentList "`"$path`""

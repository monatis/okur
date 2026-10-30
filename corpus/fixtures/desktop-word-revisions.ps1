<#
.SYNOPSIS
    Deterministic Word fixture with tracked changes and two endnotes: a heading, a paragraph
    with a word inserted and a word deleted by "Lab", and two paragraphs with an endnote each,
    opened in Microsoft Word.

.DESCRIPTION
    For JAWS's Virtual Viewer of a document's revisions and of several endnotes
    (Windows+Semicolon), and its reading of a line with revisions. Written as desktop-word.ps1
    writes its document, a WordprocessingML package, to %TEMP%\sr-word-revisions.docx on every
    run, with no Word automation (it hung on
    old-pc's unactivated Office). Then opens the file in Word. ASCII only. Do not change it
    without re-recording every scenario that uses it (the manifest pins its SHA-256).
#>
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem

$path = Join-Path $env:TEMP 'sr-word-revisions.docx'
$main = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'

$parts = [ordered]@{}
$parts['[Content_Types].xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
<Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
<Override PartName="/word/endnotes.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.endnotes+xml"/>
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
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>
<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/endnotes" Target="endnotes.xml"/>
</Relationships>
"@
# Word's built-in styles, named as Word names them: Normal, Heading 1, the notes' own.
$parts['word/styles.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles $main>
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:cs="Calibri"/><w:sz w:val="22"/><w:lang w:val="en-US"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="160" w:line="259" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="0"/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:sz w:val="32"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="FootnoteText"><w:name w:val="footnote text"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/></w:pPr><w:rPr><w:sz w:val="20"/></w:rPr></w:style>
<w:style w:type="character" w:styleId="FootnoteReference"><w:name w:val="footnote reference"/><w:rPr><w:vertAlign w:val="superscript"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="EndnoteText"><w:name w:val="endnote text"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/></w:pPr><w:rPr><w:sz w:val="20"/></w:rPr></w:style>
<w:style w:type="character" w:styleId="EndnoteReference"><w:name w:val="endnote reference"/><w:rPr><w:vertAlign w:val="superscript"/></w:rPr></w:style>
</w:styles>
"@
# A document of Word 2013 and later: without it Word opens the file in Compatibility Mode.
$parts['word/settings.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings $main><w:endnotePr><w:endnote w:id="-1"/><w:endnote w:id="0"/></w:endnotePr><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>
"@
# Word's two separator endnotes, then the document's own.
$parts['word/endnotes.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:endnotes $main><w:endnote w:type="separator" w:id="-1"><w:p><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/></w:pPr><w:r><w:separator/></w:r></w:p></w:endnote><w:endnote w:type="continuationSeparator" w:id="0"><w:p><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/></w:pPr><w:r><w:continuationSeparator/></w:r></w:p></w:endnote><w:endnote w:id="1"><w:p><w:pPr><w:pStyle w:val="EndnoteText"/></w:pPr><w:r><w:rPr><w:rStyle w:val="EndnoteReference"/></w:rPr><w:endnoteRef/></w:r><w:r><w:t xml:space="preserve"> Renewals are free.</w:t></w:r></w:p></w:endnote><w:endnote w:id="2"><w:p><w:pPr><w:pStyle w:val="EndnoteText"/></w:pPr><w:r><w:rPr><w:rStyle w:val="EndnoteReference"/></w:rPr><w:endnoteRef/></w:r><w:r><w:t xml:space="preserve"> Bring a photo ID.</w:t></w:r></w:p></w:endnote></w:endnotes>
"@
function Run([string]$text) {
    "<w:r><w:t xml:space=`"preserve`">$text</w:t></w:r>"
}
# A run inserted and a run deleted, tracked as Lab's changes on a fixed date.
$changed = 'w:author="Lab" w:date="2026-01-04T03:04:05Z"'
function Inserted([string]$id, [string]$text) {
    "<w:ins w:id=`"$id`" $changed>" + (Run $text) + '</w:ins>'
}
function Deleted([string]$id, [string]$text) {
    "<w:del w:id=`"$id`" $changed><w:r><w:delText xml:space=`"preserve`">$text</w:delText></w:r></w:del>"
}
function Endnote([string]$id) {
    "<w:r><w:rPr><w:rStyle w:val=`"EndnoteReference`"/></w:rPr><w:endnoteReference w:id=`"$id`"/></w:r>"
}
$body = @(
    '<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr>' + (Run 'Library Changes') + '</w:p>',
    '<w:p>' + (Run 'The library opens at ') + (Inserted '10' 'eight') + (Deleted '11' 'nine') + (Run ' in the morning.') + '</w:p>',
    '<w:p>' + (Run 'Members borrow books for three weeks') + (Endnote '1') + (Run '.') + '</w:p>',
    '<w:p>' + (Run 'Bring your card') + (Endnote '2') + (Run '.') + '</w:p>'
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

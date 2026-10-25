<#
.SYNOPSIS
    Deterministic Word fixture with links and fields: a heading, a paragraph with a hyperlink,
    a legacy text form field and a check box form field, and a paragraph with a second link,
    opened in Microsoft Word.

.DESCRIPTION
    For JAWS's Links List (Insert+F7) and its list of Word's fields (Insert+F5) with items in
    them. Written as desktop-word.ps1 writes its document, a WordprocessingML package, to
    %TEMP%\sr-word-fields.docx on every run, with no Word automation (it hung on old-pc's
    unactivated Office). The links go to example.org, a reserved domain; the document is never
    protected, so the form fields are Word's fields, not filled in. Then opens the file in Word.
    ASCII only. Do not change it without re-recording every scenario that uses it (the manifest
    pins its SHA-256).
#>
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem

$path = Join-Path $env:TEMP 'sr-word-fields.docx'
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
<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.org/hours" TargetMode="External"/>
<Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.org/branches" TargetMode="External"/>
</Relationships>
"@
# Word's built-in styles, named as Word names them: Normal, Heading 1, Hyperlink.
$parts['word/styles.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles $main>
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:cs="Calibri"/><w:sz w:val="22"/><w:lang w:val="en-US"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="160" w:line="259" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="0"/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:sz w:val="32"/></w:rPr></w:style>
<w:style w:type="character" w:styleId="Hyperlink"><w:name w:val="Hyperlink"/><w:rPr><w:color w:val="0563C1"/><w:u w:val="single"/></w:rPr></w:style>
</w:styles>
"@
# A document of Word 2013 and later: without it Word opens the file in Compatibility Mode.
$parts['word/settings.xml'] = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings $main><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>
"@

function Run([string]$text) {
    "<w:r><w:t xml:space=`"preserve`">$text</w:t></w:r>"
}
function Link([string]$id, [string]$text) {
    "<w:hyperlink r:id=`"$id`" w:history=`"1`"><w:r><w:rPr><w:rStyle w:val=`"Hyperlink`"/></w:rPr><w:t xml:space=`"preserve`">$text</w:t></w:r></w:hyperlink>"
}
# A legacy form field: its field code between the field's begin, separator and end.
function FormField([string]$name, [string]$data, [string]$code, [string]$shown) {
    $begin = "<w:r><w:fldChar w:fldCharType=`"begin`"><w:ffData><w:name w:val=`"$name`"/><w:enabled/><w:calcOnExit w:val=`"0`"/>$data</w:ffData></w:fldChar></w:r>"
    $instruction = "<w:r><w:instrText xml:space=`"preserve`"> $code </w:instrText></w:r>"
    $result = if ($shown) { "<w:r><w:fldChar w:fldCharType=`"separate`"/></w:r>$(Run $shown)" } else { '' }
    "$begin$instruction$result<w:r><w:fldChar w:fldCharType=`"end`"/></w:r>"
}
$body = @(
    '<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr>' + (Run 'Library Card') + '</w:p>',
    '<w:p>' + (Run 'See the ') + (Link 'rId4' 'opening hours') + (Run ' before you come.') + '</w:p>',
    '<w:p>' + (Run 'Name: ') + (FormField 'Name' '<w:textInput/>' 'FORMTEXT' '     ') + '</w:p>',
    '<w:p>' + (Run 'Member: ') + (FormField 'Member' '<w:checkBox><w:sizeAuto/><w:default w:val="0"/></w:checkBox>' 'FORMCHECKBOX' '') + '</w:p>',
    '<w:p>' + (Run 'All ') + (Link 'rId5' 'branches') + (Run ' take the card.') + '</w:p>'
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

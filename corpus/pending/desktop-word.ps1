<#
.SYNOPSIS
    Deterministic Word fixture: a short document with a heading, paragraphs, a misspelled
    word, a bulleted list and a small table, opened in Microsoft Word.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "sr-word-fixture.docx - Word"). Builds the document with Word's own automation into
    %TEMP% on every run (the same text and styles each time; built-in styles by number, so a
    Word in another language builds the same document), closes that automation copy of Word,
    then opens the file in Word as a user would, so the application is the real Word. ASCII
    only. Do not change it without re-recording every scenario that uses it (the manifest pins
    its SHA-256).
#>
$path = Join-Path $env:TEMP 'sr-word-fixture.docx'
# Word's built-in styles (WdBuiltinStyle).
$normal = -1
$heading1 = -2
$heading2 = -3
$listBullet = -49

$word = New-Object -ComObject Word.Application
$word.Visible = $false
$word.DisplayAlerts = 0
$doc = $word.Documents.Add()
$selection = $word.Selection

function Write-Paragraph([string]$text, [int]$style) {
    $selection.Style = $doc.Styles.Item($style)
    $selection.TypeText($text)
    $selection.TypeParagraph()
}

Write-Paragraph 'Library Hours' $heading1
Write-Paragraph 'The library opens at nine in the morning. It closes at nine in the evening.' $normal
Write-Paragraph 'Members can borrow teh books for three weeks.' $normal
Write-Paragraph 'What changes' $heading2
Write-Paragraph 'Weekday hours are longer.' $listBullet
Write-Paragraph 'Weekend hours stay the same.' $listBullet
Write-Paragraph 'Branches' $heading2
$selection.Style = $doc.Styles.Item($normal)
$table = $doc.Tables.Add($selection.Range, 2, 2)
$table.Borders.Enable = 1
$table.Cell(1, 1).Range.Text = 'Branch'
$table.Cell(1, 2).Range.Text = 'Hours'
$table.Cell(2, 1).Range.Text = 'Central'
$table.Cell(2, 2).Range.Text = '9 to 9'
# After the table: the document's last paragraph.
[void]$selection.EndKey(6)
$selection.TypeText('The page ends here.')
$doc.SaveAs2($path, 16)
$doc.Close(0)
$word.Quit(0)
[void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($word)
Start-Process 'C:\Program Files\Microsoft Office\Office16\WINWORD.EXE' -ArgumentList "`"$path`""

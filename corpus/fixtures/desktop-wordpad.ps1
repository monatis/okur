<#
.SYNOPSIS
    Deterministic WordPad fixture: a short rich-text document (a bold heading, paragraphs, a
    misspelled word, bulleted lines) opened in Windows WordPad.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "sr-wordpad-fixture.rtf - WordPad"). Writes the same RTF to %TEMP% on every run, then opens
    it in WordPad, Windows' own rich-text editor (Office on old-pc is not activated). ASCII
    only. Do not change it without re-recording every scenario that uses it (the manifest pins
    its SHA-256).
#>
$path = Join-Path $env:TEMP 'sr-wordpad-fixture.rtf'
$rtf = @'
{\rtf1\ansi\deff0{\fonttbl{\f0 Calibri;}}\f0\fs22
{\b\fs32 Library Hours}\par
The library opens at nine in the morning. It closes at nine in the evening.\par
Members can borrow teh books for three weeks.\par
{\b What changes}\par
\bullet\tab Weekday hours are longer.\par
\bullet\tab Weekend hours stay the same.\par
The page ends here.\par
}
'@
[System.IO.File]::WriteAllText($path, $rtf, [System.Text.Encoding]::ASCII)
Start-Process 'C:\Program Files\Windows NT\Accessories\wordpad.exe' -ArgumentList "`"$path`""

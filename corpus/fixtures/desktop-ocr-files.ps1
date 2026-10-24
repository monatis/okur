<#
.SYNOPSIS
    Deterministic file recognition fixture: a picture and a PDF with text only in their pixels,
    selected in File Explorer.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, with
    "shell_window": the window is File Explorer's, closed and never killed). Writes three files
    to "SR OCR Files" in the user's temporary folder and opens the folder with the picture
    selected:
    - "SR OCR Sign.png": two lines of black text on white;
    - "SR OCR Letter.pdf": two pages, each one JPEG picture of two lines and no text layer, as a
      scanned letter is;
    - "SR OCR Notes.txt": a file that is neither a picture nor a PDF.
    In name order the letter is first and the sign last (Home and End select them). This
    process stays until ExitAfterSeconds. Do not change it without re-recording every scenario
    that uses it (the manifest pins its SHA-256). -FilesOnly writes the files and opens nothing,
    for trying Okur's recognition of them on another machine.
#>
param([int]$ExitAfterSeconds = 900, [switch]$FilesOnly)

Add-Type -AssemblyName System.Drawing

$folder = Join-Path $env:TEMP 'SR OCR Files'
if (Test-Path $folder) { Remove-Item -Recurse -Force $folder }
New-Item -ItemType Directory -Force -Path $folder | Out-Null

# A white picture of `width` by `height` with `lines` in black, 56 pixels high, one under the
# other from the top left.
function New-TextPicture([int]$width, [int]$height, [string[]]$lines) {
    $bitmap = New-Object System.Drawing.Bitmap $width, $height, ([System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $g = [System.Drawing.Graphics]::FromImage($bitmap)
    $g.Clear([System.Drawing.Color]::White)
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $font = New-Object System.Drawing.Font 'Arial', 56, ([System.Drawing.FontStyle]::Regular), ([System.Drawing.GraphicsUnit]::Pixel)
    $y = 120
    foreach ($line in $lines) {
        $g.DrawString($line, $font, [System.Drawing.Brushes]::Black, 120, $y)
        $y += 110
    }
    $font.Dispose()
    $g.Dispose()
    return $bitmap
}

function Get-JpegBytes($bitmap) {
    $codec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq 'image/jpeg' }
    $parameters = New-Object System.Drawing.Imaging.EncoderParameters 1
    $parameters.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter ([System.Drawing.Imaging.Encoder]::Quality), ([long]90)
    $stream = New-Object System.IO.MemoryStream
    $bitmap.Save($stream, $codec, $parameters)
    return $stream.ToArray()
}

$sign = New-TextPicture 1200 400 @('Opening hours', 'Monday to Friday 9 to 5')
$sign.Save((Join-Path $folder 'SR OCR Sign.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$sign.Dispose()

# An A4 PDF whose pages are JPEG pictures at 150 dots per inch, written object by object with
# the byte offsets its cross-reference table needs.
$pages = @(
    @('A letter drawn as pictures', 'Here is the first page'),
    @('Here is the second page', 'With kind regards')
)
$pdf = New-Object System.IO.MemoryStream
$offsets = New-Object System.Collections.Generic.List[long]
function Write-Ascii([string]$text) {
    $bytes = [System.Text.Encoding]::ASCII.GetBytes($text)
    $pdf.Write($bytes, 0, $bytes.Length)
}
function Start-Object([int]$number) {
    $offsets.Add($pdf.Position)
    Write-Ascii "$number 0 obj`n"
}
Write-Ascii "%PDF-1.4`n"
$kids = (0..($pages.Count - 1) | ForEach-Object { "$(3 + 3 * $_) 0 R" }) -join ' '
Start-Object 1
Write-Ascii "<< /Type /Catalog /Pages 2 0 R >>`nendobj`n"
Start-Object 2
Write-Ascii "<< /Type /Pages /Kids [$kids] /Count $($pages.Count) >>`nendobj`n"
for ($i = 0; $i -lt $pages.Count; $i++) {
    $page = 3 + 3 * $i
    $picture = New-TextPicture 1240 1754 $pages[$i]
    $jpeg = Get-JpegBytes $picture
    $picture.Dispose()
    Start-Object $page
    Write-Ascii "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /XObject << /Im0 $($page + 1) 0 R >> >> /Contents $($page + 2) 0 R >>`nendobj`n"
    Start-Object ($page + 1)
    Write-Ascii "<< /Type /XObject /Subtype /Image /Width 1240 /Height 1754 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length $($jpeg.Length) >>`nstream`n"
    $pdf.Write($jpeg, 0, $jpeg.Length)
    Write-Ascii "`nendstream`nendobj`n"
    $content = "q 595 0 0 842 0 0 cm /Im0 Do Q"
    Start-Object ($page + 2)
    Write-Ascii "<< /Length $($content.Length) >>`nstream`n$content`nendstream`nendobj`n"
}
$xref = $pdf.Position
Write-Ascii "xref`n0 $($offsets.Count + 1)`n0000000000 65535 f `n"
foreach ($offset in $offsets) { Write-Ascii ("{0:D10} 00000 n `n" -f $offset) }
Write-Ascii "trailer`n<< /Size $($offsets.Count + 1) /Root 1 0 R >>`nstartxref`n$xref`n%%EOF`n"
[System.IO.File]::WriteAllBytes((Join-Path $folder 'SR OCR Letter.pdf'), $pdf.ToArray())

Set-Content -Path (Join-Path $folder 'SR OCR Notes.txt') -Value 'A text file, not a picture.' -Encoding ascii

if ($FilesOnly) { return }
Start-Process explorer.exe -ArgumentList "/select,`"$(Join-Path $folder 'SR OCR Sign.png')`""
Start-Sleep -Seconds $ExitAfterSeconds

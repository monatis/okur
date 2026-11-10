<#
.SYNOPSIS
    Closes every File Explorer window on the reference machine before a batch.

.DESCRIPTION
    A File Explorer window left open (an OCR round's, or one opened on the Desktop folder) is
    where the focus lands when a fixture's window closes, and its trace then holds the
    machine's own files: round 1209b's trace of the Window List's Delete held the desktop's
    (2026-11-10). The tracer leaves the files beside an Explorer focus out
    (`privacy::without_files_beside`), and a batch starts with none open all the same. For
    queue-batch.ps1's -Before. Prints how many were closed.
#>
$closed = 0
$shell = New-Object -ComObject Shell.Application
foreach ($window in @($shell.Windows())) {
    try {
        $window.Quit()
        $closed++
    } catch {
    }
}
Start-Sleep -Seconds 2
"closed $closed File Explorer window(s); " + @((New-Object -ComObject Shell.Application).Windows()).Count + " left"

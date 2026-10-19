<#
.SYNOPSIS
    Deterministic Notepad fixture for file dialogs: a small text file, alone in a folder of its
    own, opened in Windows Notepad.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block, window title
    "sr-dialog-fixture.txt - Notepad"). Makes %TEMP%\sr-dialogs anew on every run with only
    this file in it, then opens it in Notepad. Notepad's Open and Save As dialogs open in the
    folder Notepad last used; skills/jaws-reference/scripts/set-dialog-folder.ps1 makes that this
    one, so a dialog shows this file and nothing of the machine's own folders. ASCII only. Do
    not change it without re-recording every scenario that uses it (the manifest pins its
    SHA-256).
#>
$folder = Join-Path $env:TEMP 'sr-dialogs'
Remove-Item -Recurse -Force $folder -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $folder | Out-Null
$path = Join-Path $folder 'sr-dialog-fixture.txt'
$lines = @(
    'Dialog fixture',
    'The file dialogs open in this folder.'
)
[System.IO.File]::WriteAllText($path, ($lines -join "`r`n"), (New-Object System.Text.UTF8Encoding $false))
Start-Process notepad.exe -ArgumentList "`"$path`""

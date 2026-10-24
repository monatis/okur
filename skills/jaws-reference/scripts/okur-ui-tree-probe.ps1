# Research, never evidence: okur-ui's settings tree on its own. okur-ui is fed a dialog with a
# search box, a tree (a closed group, a setting named with its value, a check box setting), a
# read-only description and OK/Cancel over its standard input; keys go to it (okur-lab keys,
# only while okur-ui is in front); after each key, what okur-ui reported on its standard output
# and what JAWS said of it are written. Run on old-pc with no batch recording.
param([string]$Out = (Join-Path $env:USERPROFILE 'sr-harness\probe\okur-ui-tree.txt'),
      [string[]]$Keys = @('Tab', 'Right', 'Down', 'Space', 'Shift+Space', 'Down', 'Space', 'Up', 'Left', 'Ctrl+E', 'Tab', 'Ctrl+Shift+D', 'Tab', 'Escape'),
      # Also an outline of the dialog's UI Automation tree before the keys (uia-outline.ps1).
      [switch]$Outline)
$bin = Join-Path $env:USERPROFILE 'sr-harness\bin'
$lab = Join-Path $bin 'okur-lab.exe'
$start = New-Object System.Diagnostics.ProcessStartInfo
$start.FileName = Join-Path $bin 'okur-ui.exe'
$start.UseShellExecute = $false
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$ui = [System.Diagnostics.Process]::Start($start)
$reports = New-Object System.Collections.Concurrent.ConcurrentQueue[string]
$reader = Register-ObjectEvent -InputObject $ui -EventName OutputDataReceived -MessageData $reports -Action {
    if ($EventArgs.Data) { $Event.MessageData.Enqueue($EventArgs.Data) }
}
$ui.BeginOutputReadLine()
$dialog = @'
{"type":"dialog_show","dialog":{"id":1,"title":"Notepad - Okur Settings Center","controls":[
 {"id":"search","label":"SearchBox","kind":{"kind":"edit","text":""}},
 {"id":"tree","label":"","kind":{"kind":"tree","selected":0,"items":[
   {"text":"Speech Options","level":0,"open":false},
   {"text":"Typing Echo Characters","level":1},
   {"text":"Caps Indicate","level":1,"checked":false},
   {"text":"Mouse Echo","level":0,"open":false},
   {"text":"Enable Mouse Echo","level":1,"checked":true}]}},
 {"id":"description","label":"","kind":{"kind":"note","text":"A group of settings: Right Arrow opens it, and Space on a setting in it changes its value."}},
 {"id":"ok","label":"OK","kind":{"kind":"button","default":true}},
 {"id":"cancel","label":"Cancel","kind":{"kind":"button","default":false}}],
 "keys":[{"key":"Ctrl+E","control":"search"},{"key":"Ctrl+Shift+D","control":"default_place"}]}}
'@ -replace "`r?`n", ''
$ui.StandardInput.WriteLine($dialog)
$ui.StandardInput.Flush()
Start-Sleep -Seconds 3
& $lab jaws-history --clear | Out-Null
$lines = @("--- shown: " + (& $lab front 'Okur Settings Center'))
if ($Outline) {
    $outlineFile = [System.IO.Path]::ChangeExtension($Out, '.outline.txt')
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'uia-outline.ps1') -Title 'Notepad - Okur Settings Center' -Out $outlineFile | Out-Null
    $lines += Get-Content $outlineFile
}
foreach ($key in $Keys) {
    & $lab keys --allow okur-ui $key 'wait:900' 2>&1 | Out-Null
    $said = (& $lab jaws-history --clear) -join ' | '
    $told = @()
    $line = $null
    while ($reports.TryDequeue([ref]$line)) { $told += $line }
    $lines += "== $key => $said"
    $lines += $told | ForEach-Object { "   okur-ui: $_" }
}
$ui.StandardInput.WriteLine('{"type":"dialog_close","id":1}')
$ui.StandardInput.Close()
[void]$ui.WaitForExit(5000)
if (-not $ui.HasExited) { $ui.Kill() }
Unregister-Event -SourceIdentifier $reader.Name
$lines | Set-Content $Out
$lines

# Research, never evidence: okur-ui's settings tree on its own. okur-ui is fed a dialog with a
# search box, a tree (a closed group, a setting named with its value, a check box setting), a
# read-only description and OK/Cancel over its standard input; keys go to it (okur-lab keys,
# only while okur-ui is in front); after each key, what okur-ui reported on its standard output
# and what JAWS said of it are written. Run on old-pc with no batch recording.
param([string]$Out = (Join-Path $env:USERPROFILE 'sr-harness\probe\okur-ui-tree.txt'),
      [string[]]$Keys = @('Tab', 'Right', 'Down', 'Space', 'Shift+Space', 'Down', 'Space', 'Up', 'Left', 'Ctrl+E', 'Down', 'Enter', 'Escape', 'Ctrl+Shift+D', 'Tab', 'Escape'),
      # Also an outline of the dialog's UI Automation tree before the keys (uia-outline.ps1).
      [switch]$Outline,
      # The lifetime menu instead: Tab to the tree, okur-ui asked for a context menu on it
      # ("menu"), Down and Enter to choose, then Escape.
      [switch]$Menu,
      # Okur's main window instead of the dialog: its menu bar walked with Alt and the arrows,
      # then Alt+F4.
      [switch]$Window,
      # Cancel's question instead: Escape and the Cancel button reported as pressed with the
      # dialog left open, the question shown over it ("question"), Tab, and Escape back.
      [switch]$Question,
      # Voice Adjustment's shape instead: Profile Name, Delete Profile, the Voice group's Adjust,
      # Person and Rate, Save As..., OK, Cancel; Tab around, then Profile Name's choices
      # replaced ("items", as after Save As) and read with Down and Up.
      [switch]$Voice,
      # The Startup Wizard's shape instead: "Okur Startup Wizard" on its Speech Settings page
      # (Rate, Punctuation, Typing Echo), the slider moved and Tab around; then turned to Common
      # Options ("page", as Okur answers Next), Tab around, and Escape (Cancel).
      [switch]$Wizard)
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
 "keys":[{"key":"Ctrl+E","control":"search"},{"key":"Ctrl+Shift+D","control":"default_place"},
  {"key":"Down","control":"tree","from":"search"},{"key":"Enter","control":"tree_enter","from":"tree"},
  {"key":"Escape","control":"tree_escape","from":"tree"},{"key":"Escape","control":"cancel"}]}}
'@ -replace "`r?`n", ''
if ($Window) {
    $dialog = '{"type":"main_window_show","window":{"title":"Okur","lines":["Okur version 0.1.0"],"menus":[{"label":"&Options","items":[{"id":"basics","label":"&Basics..."},{"id":"exit","label":"E&xit","key":"Alt+F4"}]},{"label":"&Utilities","items":[{"id":"notification_history","label":"Notification History","key":"INSERT+SPACE, N"},{"id":"settings_center","label":"Settings Center"}]},{"label":"&Help","items":[{"id":"about","label":"&About Okur..."}]}]}}'
    $Keys = @('Alt', 'Down', 'Down', 'Right', 'Down', 'Escape', 'Escape', 'Alt', 'Down', 'Enter', 'Alt+F4')
}
if ($Voice) {
    $dialog = '{"type":"dialog_show","dialog":{"id":1,"title":"Voice Adjustment","controls":[{"id":"profile","label":"Profile Name:","kind":{"kind":"choice","options":["Automatic - (factory)","OneCore - (factory)","SAPI 5 - (factory)"],"selected":0}},{"id":"delete_profile","label":"Delete Profile","kind":{"kind":"button","default":false},"enabled":false},{"id":"voice","label":"Voice","kind":{"kind":"group"}},{"id":"adjust","label":"Adjust:","kind":{"kind":"choice","options":["All Contexts","PC Cursor Voice","Okur Cursor Voice","Keyboard Voice","Tutor and Message Voice"],"selected":0}},{"id":"person","label":"Person:","kind":{"kind":"choice","options":["Default"],"selected":0}},{"id":"rate","label":"Rate:","kind":{"kind":"slider","min":-10,"max":10,"value":0}},{"id":"save_as","label":"Save As...","kind":{"kind":"button","default":false}},{"id":"ok","label":"OK","kind":{"kind":"button","default":true},"enabled":false},{"id":"cancel","label":"Cancel","kind":{"kind":"button","default":false}}]}}'
    $Keys = @('Tab', 'Tab', 'Tab', 'Tab', 'Tab', 'Tab', 'Shift+Tab', 'Shift+Tab', 'Shift+Tab', 'Shift+Tab', 'Shift+Tab', 'Shift+Tab', 'items', 'Down', 'Up', 'Escape')
}
if ($Wizard) {
    $dialog = '{"type":"wizard_show","wizard":{"id":1,"page":{"id":1,"title":"Speech Settings - Okur Startup Wizard","controls":[{"id":"rate","label":"Rate:","kind":{"kind":"slider","min":-10,"max":10,"value":0}},{"id":"punctuation","label":"Punctuation:","kind":{"kind":"choice","options":["None","Some","Most","All"],"selected":2}},{"id":"typing_echo","label":"Typing Echo","kind":{"kind":"radios","options":["Off","Characters","Words","Characters and Words"],"selected":1}}],"keys":[]},"buttons":{"back":false,"next":true,"finish":false}}}'
    $Keys = @('Right', 'Right', 'Tab', 'Tab', 'Down', 'Tab', 'Tab', 'Tab', 'page', 'Tab', 'Tab', 'Tab', 'Tab', 'Tab', 'Escape')
}
$ui.StandardInput.WriteLine($dialog)
$ui.StandardInput.Flush()
Start-Sleep -Seconds 3
& $lab jaws-history --clear | Out-Null
$lines = @("--- shown: " + (& $lab front ($(if ($Window) { 'Okur' } elseif ($Voice) { 'Voice Adjustment' } elseif ($Wizard) { 'Okur Startup Wizard' } else { 'Okur Settings Center' }))))
if ($Outline) {
    $outlineFile = [System.IO.Path]::ChangeExtension($Out, '.outline.txt')
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'uia-outline.ps1') -Title 'Notepad - Okur Settings Center' -Out $outlineFile | Out-Null
    $lines += Get-Content $outlineFile
}
if ($Menu) { $Keys = @('Tab', 'menu', 'Down', 'Enter', 'Tab', 'Escape') }
if ($Question) { $Keys = @('Escape', 'question', 'Tab', 'Escape', 'Tab', 'Tab', 'Tab', 'Tab', 'Space', 'question', 'Tab', 'Space') }
foreach ($key in $Keys) {
    if ($key -eq 'menu') {
        $ui.StandardInput.WriteLine('{"type":"dialog_menu","id":1,"control":"tree","items":[{"text":"Restore When Okur Exits"},{"text":"Restore When Focus Changes"},{"text":"Save","checked":true}]}')
        $ui.StandardInput.Flush()
        Start-Sleep -Milliseconds 1500
        $lines += "== menu => " + ((& $lab jaws-history --clear) -join ' | ')
        continue
    }
    if ($key -eq 'page') {
        $ui.StandardInput.WriteLine('{"type":"wizard_page","id":1,"page":{"id":1,"title":"Common Options - Okur Startup Wizard","controls":[{"id":"smart_navigation","label":"Smart Navigation","kind":{"kind":"radios","options":["Off","Controls Only","Tables","Controls and Tables"],"selected":0}},{"id":"forms_mode","label":"Forms Mode","kind":{"kind":"radios","options":["Auto","Semi Auto","Manual"],"selected":2}},{"id":"layout","label":"Use Keyboard Layout:","kind":{"kind":"choice","options":["Desktop","Laptop"],"selected":1}}],"keys":[]},"buttons":{"back":true,"next":true,"finish":false}}')
        $ui.StandardInput.Flush()
        Start-Sleep -Milliseconds 1500
        $lines += "== page => " + ((& $lab jaws-history --clear) -join ' | ')
        continue
    }
    if ($key -eq 'items') {
        $ui.StandardInput.WriteLine('{"type":"dialog_items","id":1,"control":"profile","items":["Automatic - (factory)","Fast","OneCore - (factory)","SAPI 5 - (factory)"],"selected":1}')
        $ui.StandardInput.Flush()
        Start-Sleep -Milliseconds 1500
        $lines += "== items => " + ((& $lab jaws-history --clear) -join ' | ')
        continue
    }
    if ($key -eq 'question') {
        $ui.StandardInput.WriteLine('{"type":"dialog_show","dialog":{"id":2,"title":"Okur Settings Center","controls":[{"id":"question","label":"","kind":{"kind":"message","text":"You have made changes to Notepad''s settings. Do you want to save them?"}},{"id":"yes","label":"&Yes","kind":{"kind":"button","default":true}},{"id":"no","label":"&No","kind":{"kind":"button","default":false}}]}}')
        $ui.StandardInput.Flush()
        Start-Sleep -Milliseconds 1500
        $lines += "== question => " + ((& $lab jaws-history --clear) -join ' | ')
        continue
    }
    & $lab keys --allow okur-ui $key 'wait:900' 2>&1 | Out-Null
    $said = (& $lab jaws-history --clear) -join ' | '
    $told = @()
    $line = $null
    while ($reports.TryDequeue([ref]$line)) { $told += $line }
    $lines += "== $key => $said"
    $lines += $told | ForEach-Object { "   okur-ui: $_" }
}
$ui.StandardInput.WriteLine('{"type":"dialog_close","id":2}')
$ui.StandardInput.WriteLine('{"type":"dialog_close","id":1}')
$ui.StandardInput.Close()
[void]$ui.WaitForExit(5000)
if (-not $ui.HasExited) { $ui.Kill() }
Unregister-Event -SourceIdentifier $reader.Name
$lines | Set-Content $Out
$lines

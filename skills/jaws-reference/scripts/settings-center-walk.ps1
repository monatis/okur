# Research, never evidence: JAWS's Settings Center (Insert+6) over Notepad, its tree walked
# from top to bottom with every group opened (jaws-ui-walk.ps1 -AutoExpand), then F6 to the
# selected item's control and back, for Okur's settings redesign
# (docs/references/okur-settings-redesign.md §5). Navigation keys, F6 and Escape only: Space or
# Enter would change JAWS's settings. Run on old-pc with no batch recording.
param([string]$OutDir = (Join-Path $env:USERPROFILE 'sr-harness\probe\settings-center'),
      [int]$Downs = 600, [switch]$TreeOnly)
$lab = Join-Path $env:USERPROFILE 'sr-harness\bin\okur-lab.exe'
$walk = Join-Path $PSScriptRoot 'jaws-ui-walk.ps1'
New-Item -ItemType Directory -Force $OutDir | Out-Null
Add-Type -Namespace SCW -Name W -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);'
function Front-Process {
    $p = 0
    [void][SCW.W]::GetWindowThreadProcessId([SCW.W]::GetForegroundWindow(), [ref]$p)
    (Get-Process -Id $p -ErrorAction SilentlyContinue).ProcessName
}
if (-not (Get-Process notepad -ErrorAction SilentlyContinue)) { Start-Process notepad; Start-Sleep -Seconds 2 }
& $lab front 'Notepad' | Out-Null
Start-Sleep -Milliseconds 500
& $lab jaws-history --clear | Out-Null
& $lab keys --allow notepad 'Insert+6' 'wait:4000' | Out-Null
$process = Front-Process
"--- Insert+6: process $process | " + ((& $lab jaws-history --clear) -join ' | ') | Set-Content (Join-Path $OutDir 'open.txt')
if (-not $process -or $process -eq 'notepad') { 'Settings Center did not come to the front'; exit 1 }
# The Tab order around the dialog, from the search box.
if (-not $TreeOnly) { & powershell -NoProfile -ExecutionPolicy Bypass -File $walk -Process $process -Chords 'Tab*9' -WaitMs 900 -Out (Join-Path $OutDir 'tab-order.txt') | Out-Null }
# Back to the search box (Ctrl+E), into the tree, and down it with every group opened; -TreeOnly
# skips the Tab order.
& powershell -NoProfile -ExecutionPolicy Bypass -File $walk -Process $process -Chords "Control+E,Down,Home,Down*$Downs" -WaitMs 700 -AutoExpand -Out (Join-Path $OutDir 'tree.txt') | Out-Null
# F6 from the item reached to its control in the pane and back; then Escape (Cancel).
& powershell -NoProfile -ExecutionPolicy Bypass -File $walk -Process $process -Chords 'F6,Tab,F6,Escape' -WaitMs 1000 -Out (Join-Path $OutDir 'f6.txt') | Out-Null
& $lab front 'Notepad' | Out-Null
'done'

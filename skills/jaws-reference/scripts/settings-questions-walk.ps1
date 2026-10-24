# Research, never evidence: the open questions of Okur's settings redesign
# (docs/references/okur-settings-redesign.md §5), walked in JAWS's Settings Center and Quick
# Settings over Notepad: Space and Shift+Space on a setting, the Tab order once something
# changed (Apply), F6 to the setting's control and back, Quick Settings' Tab order, and its
# Shift+F10 menu of lifetimes. Every change is left by Escape (Cancel): nothing is saved. Run on
# old-pc with no batch recording.
param([string]$OutDir = (Join-Path $env:USERPROFILE 'sr-harness\probe\settings-questions'))
$lab = Join-Path $env:USERPROFILE 'sr-harness\bin\okur-lab.exe'
$walk = Join-Path $PSScriptRoot 'jaws-ui-walk.ps1'
New-Item -ItemType Directory -Force $OutDir | Out-Null
Add-Type -Namespace SQW -Name W -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);'
function Front-Process {
    $p = 0
    [void][SQW.W]::GetWindowThreadProcessId([SQW.W]::GetForegroundWindow(), [ref]$p)
    (Get-Process -Id $p -ErrorAction SilentlyContinue).ProcessName
}
function Open([string]$chord, [string]$out) {
    & $lab front 'Notepad' | Out-Null
    Start-Sleep -Milliseconds 500
    & $lab jaws-history --clear | Out-Null
    & $lab keys --allow notepad $chord 'wait:4000' | Out-Null
    $process = Front-Process
    "--- ${chord}: process $process | " + ((& $lab jaws-history --clear) -join ' | ') | Set-Content (Join-Path $OutDir $out)
    $process
}
function Walk([string]$process, [string]$chords, [string]$out) {
    & powershell -NoProfile -ExecutionPolicy Bypass -File $walk -Process $process -Chords $chords -WaitMs 900 -Out (Join-Path $OutDir $out) | Out-Null
}
if (-not (Get-Process notepad -ErrorAction SilentlyContinue)) { Start-Process notepad; Start-Sleep -Seconds 2 }
# Settings Center: into the tree, User opened, down to Typing Echo; Space, Shift+Space; Tab
# around with something changed; back to the tree; F6 and F6; Escape (Cancel), and again in case
# it asks.
$center = Open 'Insert+6' 'center-open.txt'
if ($center -and $center -ne 'notepad') {
    Walk $center 'Down,Right,Down,Down,Space,Space,Shift+Space,Tab,Tab,Tab,Tab,Tab,Tab,Tab,Shift+Tab*6,F6,F6,Escape,Escape' 'center.txt'
}
# Quick Settings: its Tab order, then into the tree, a setting's Shift+F10 menu walked, and out.
$quick = Open 'Insert+V' 'quick-open.txt'
if ($quick -and $quick -ne 'notepad') {
    Walk $quick 'Tab*8,Down,Down,Shift+F10,Down,Down,Down,Escape,Escape,Escape' 'quick.txt'
}
& $lab front 'Notepad' | Out-Null
'done'

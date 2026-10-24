# Research, never evidence: the open questions of Okur's settings redesign
# (docs/references/okur-settings-redesign.md §5), walked in JAWS's Settings Center and Quick
# Settings over Notepad. Each walk opens its dialog from Notepad (Insert+6 or Insert+V), sends its
# keys while the dialog is in front, and writes what JAWS said. The defaults answer the second
# round's questions:
# - Settings Center: a change, Escape's question, Tab past No (a third button?), Escape back,
#   the question again, No (closes, nothing saved);
# - Settings Center again: Enter on a setting with nothing changed;
# - Quick Settings: its Tab order from where it opens;
# - Quick Settings again: a setting's Shift+F10 menu of lifetimes walked, and out.
# Nothing is saved: every change is left by No or Escape. A walk given as '' is skipped. Run on
# old-pc with no batch recording.
param([string]$OutDir = (Join-Path $env:USERPROFILE 'sr-harness\probe\settings-questions'),
      [string]$Center = 'Down,Right,Down,Down,Space,Escape,Tab,Tab,Tab,Escape,Escape,Tab,Space',
      [string]$CenterAgain = 'Down,Right,Down,Down,Enter',
      [string]$Quick = 'Tab,Tab,Tab,Tab,Tab,Tab,Tab,Tab',
      [string]$QuickAgain = 'Down,Down,Shift+F10,Down,Down,Down,Escape,Escape,Escape')
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
# One dialog's walk: opened from Notepad, keys sent while it is in front. A dialog still open
# afterwards (a walk that ends inside it) is left by Escape and, if it asks, No.
function Run([string]$chord, [string]$chords, [string]$name) {
    if (-not $chords) { return }
    $process = Open $chord "$name-open.txt"
    if ($process -and $process -ne 'notepad') {
        Walk $process $chords "$name.txt"
        if ((Front-Process) -eq $process) { Walk $process 'Escape,Tab,Space' "$name-left.txt" }
    }
}
if (-not (Get-Process notepad -ErrorAction SilentlyContinue)) { Start-Process notepad; Start-Sleep -Seconds 2 }
Run 'Insert+6' $Center 'center'
Run 'Insert+6' $CenterAgain 'center-again'
Run 'Insert+V' $Quick 'quick'
Run 'Insert+V' $QuickAgain 'quick-again'
& $lab front 'Notepad' | Out-Null
'done'

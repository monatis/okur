# Research, never evidence: JAWS's Manage Key Labels (Settings Center, Keyboard, General,
# "Manage Key Labels..."), each key's row read whole with Insert+Up (Key Name, Key Label, Mute
# Status, as JAWS's help says), from the first row down until a row comes again; then the
# dialog's Tab order, and Change Label's dialog opened and cancelled. Navigation keys, Tab,
# Enter on the item and on Change Label, and Escape only: Toggle Mute is never pressed, and
# every dialog is left by Escape, so JAWS's settings stay as they are. Run on old-pc with no
# batch recording and the Linux lab shut down.
param([string]$OutDir = (Join-Path $env:USERPROFILE 'sr-harness\probe\key-labels'),
      [int]$Rows = 400, [int]$WaitMs = 700)
$lab = Join-Path $env:USERPROFILE 'sr-harness\bin\okur-lab.exe'
$keys = 'C:\Program Files\Okur Lab\okur-lab-keys.exe'
New-Item -ItemType Directory -Force $OutDir | Out-Null
Add-Type -Namespace KLW -Name W -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);'
function Front-Process {
    $p = 0
    [void][KLW.W]::GetWindowThreadProcessId([KLW.W]::GetForegroundWindow(), [ref]$p)
    (Get-Process -Id $p -ErrorAction SilentlyContinue).ProcessName
}
# One chord into the dialog's process (elevated, so through okur-lab-keys with UI Access), and
# what JAWS said for it.
function Key([string]$Process, [string]$Chord, [int]$Wait = $WaitMs) {
    $p = Start-Process -FilePath $keys -ArgumentList @('--allow', $Process, "`"$Chord`"", "`"wait:$Wait`"") -WindowStyle Hidden -Wait -PassThru
    if ($p.ExitCode -ne 0) { return "(refused: $($p.ExitCode))" }
    (& $lab jaws-history --clear) -join ' | '
}
$log = Join-Path $OutDir 'walk.txt'
function Note([string]$Line) { $Line | Add-Content -Path $log -Encoding utf8; $Line }
Remove-Item $log -ErrorAction SilentlyContinue

if (-not (Get-Process notepad -ErrorAction SilentlyContinue)) { Start-Process notepad; Start-Sleep -Seconds 2 }
& $lab front 'Notepad' | Out-Null
Start-Sleep -Milliseconds 500
& $lab jaws-history --clear | Out-Null
& $lab keys --allow notepad 'Insert+6' 'wait:4000' | Out-Null
$sc = Front-Process
Note "--- Insert+6: process $sc | $((& $lab jaws-history --clear) -join ' | ')" | Out-Null
if (-not $sc -or $sc -eq 'notepad') { 'Settings Center did not come to the front'; exit 1 }

# The tree from its top: Keyboard by its first letter, opened, then General opened, and down to
# the item.
foreach ($chord in 'Control+E', 'Down', 'Home', 'K') { Note "== $chord => $(Key $sc $chord)" | Out-Null }
$said = Key $sc 'Right'; Note "== Right => $said" | Out-Null
$said = Key $sc 'Down'; Note "== Down => $said" | Out-Null
if ($said -match 'closed') { Note "== Right => $(Key $sc 'Right')" | Out-Null }
$found = $false
foreach ($i in 1..14) {
    $said = Key $sc 'Down'
    Note "== Down => $said" | Out-Null
    if ($said -match 'Manage Key Labels') { $found = $true; break }
}
if (-not $found) { Note 'Manage Key Labels not reached'; Key $sc 'Escape' | Out-Null; exit 1 }
# Enter in the tree is OK (Settings Center closes): F6 goes to the item's own button in the
# pane, and Space presses it.
$said = Key $sc 'F6'
Note "== F6 => $said" | Out-Null
if ($said -notmatch 'Manage Key Labels') { Note 'the button not reached'; Key $sc 'Escape' | Out-Null; exit 1 }
$said = Key $sc 'Space' 3000
$dialog = Front-Process
Note "== Space => process $dialog | $said" | Out-Null
if ($dialog -eq 'notepad') { Note 'the dialog did not open'; exit 1 }

# Each row, read whole, until the list ends (Down says the last row again or nothing new).
$said = Key $dialog 'Home'; Note "== Home => $said" | Out-Null
$last = $null
foreach ($row in 1..$Rows) {
    $whole = Key $dialog 'Insert+Up'
    Note "row $row => $whole" | Out-Null
    if ($whole -eq $last) { break }
    $last = $whole
    Key $dialog 'Down' | Out-Null
}
# The dialog's Tab order, from the list.
foreach ($i in 1..8) { Note "== Tab => $(Key $dialog 'Tab')" | Out-Null }
# Change Label's own dialog: Shift+Tab back to the list's first row, then Tab until Change Label.
foreach ($i in 1..8) {
    $said = Key $dialog 'Shift+Tab'
    if ($said -match 'Key List|list view|List view') { break }
}
Key $dialog 'Home' | Out-Null
foreach ($i in 1..6) {
    $said = Key $dialog 'Tab'
    Note "== Tab => $said" | Out-Null
    if ($said -match 'Change Label') {
        Note "== Enter => $(Key $dialog 'Enter' 1500)" | Out-Null
        foreach ($j in 1..4) { Note "== Tab => $(Key (Front-Process) 'Tab')" | Out-Null }
        Note "== Escape => $(Key (Front-Process) 'Escape' 1500)" | Out-Null
        break
    }
}
# Out of the dialog and Settings Center, nothing applied.
Note "== Escape => $(Key (Front-Process) 'Escape' 1500)" | Out-Null
$front = Front-Process
if ($front -ne 'notepad') { Note "== Escape => $(Key $front 'Escape' 1500)" | Out-Null }
Note "--- front: $(Front-Process)" | Out-Null
'done'

# Research, never evidence: JAWS's Web Verbosity Level and Configure Web Verbosity Levels
# (Settings Center, Web / HTML / PDFs, Reading). Over Chrome, the level Chrome's settings give
# (Chrome.JCF's [VirtualCursorVerbosity] VirtualCursorVerbosityLevel, 2 on old-pc: which level is
# that); over Notepad, each of High..., Medium... and Low... opened and its controls read by Tab,
# its tree's items read by Down. Navigation keys, F6, Tab, Space on the level's button only, and Escape:
# no check box is turned, and every dialog is left by Escape. Run on old-pc with no batch
# recording and the Linux lab shut down.
param([string]$OutDir = (Join-Path $env:USERPROFILE 'sr-harness\probe\web-verbosity'),
      [int]$WaitMs = 700, [int]$Tabs = 40,
      # Only the level Chrome's settings give.
      [switch]$ChromeOnly)
$lab = Join-Path $env:USERPROFILE 'sr-harness\bin\okur-lab.exe'
$keys = 'C:\Program Files\Okur Lab\okur-lab-keys.exe'
New-Item -ItemType Directory -Force $OutDir | Out-Null
$log = Join-Path $OutDir 'walk.txt'
Remove-Item $log -ErrorAction SilentlyContinue
function Note([string]$Line) { $Line | Add-Content -Path $log -Encoding utf8 }
Add-Type -Namespace WVW -Name W -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);'
function Front-Process {
    $p = 0
    [void][WVW.W]::GetWindowThreadProcessId([WVW.W]::GetForegroundWindow(), [ref]$p)
    (Get-Process -Id $p -ErrorAction SilentlyContinue).ProcessName
}
function Key([string]$Process, [string]$Chord, [int]$Wait = $WaitMs) {
    $p = Start-Process -FilePath $keys -ArgumentList @('--allow', $Process, "`"$Chord`"", "`"wait:$Wait`"") -WindowStyle Hidden -Wait -PassThru
    if ($p.ExitCode -ne 0) { return "(refused: $($p.ExitCode))" }
    (& $lab jaws-history --clear) -join ' | '
}
# Settings Center over the application `app` (its window titled `title`), the tree down to
# Web / HTML / PDFs, Reading, opened: the process name, or nothing.
function Open-Reading([string]$app, [string]$title) {
    & $lab front $title | Out-Null
    Start-Sleep -Milliseconds 500
    & $lab jaws-history --clear | Out-Null
    & $lab keys --allow $app 'Insert+6' 'wait:4000' | Out-Null
    $sc = Front-Process
    if ($sc -ne 'SettingsCenter') { Note "Settings Center did not open over $app ($sc)"; return $null }
    foreach ($chord in 'Control+E', 'Down', 'Home', 'W') { Note "== $chord => $(Key $sc $chord)" }
    Note "== Right => $(Key $sc 'Right')"
    $said = Key $sc 'Down'; Note "== Down => $said"
    if ($said -match 'closed') { Note "== Right => $(Key $sc 'Right')" }
    Note "== Down => $(Key $sc 'Down')"
    $sc
}

# Over Chrome: the level its settings give.
if (Get-Process chrome -ErrorAction SilentlyContinue) {
    $sc = Open-Reading 'chrome' 'Chrome'
    if ($sc) { Note "== Escape => $(Key $sc 'Escape' 1500)" }
} else { Note 'Chrome is not running: its level not read' }

# Over Notepad: each level's Configure dialog.
if (-not (Get-Process notepad -ErrorAction SilentlyContinue)) { Start-Process notepad; Start-Sleep -Seconds 2 }
foreach ($level in @(1..3 | Where-Object { -not $ChromeOnly })) {
    $sc = Open-Reading 'notepad' 'Notepad'
    if (-not $sc) { break }
    # Down to Configure Web Verbosity Levels, opened, then to the level's button item.
    $said = Key $sc 'Down'; Note "== Down => $said"
    if ($said -match 'closed') { Note "== Right => $(Key $sc 'Right')" }
    foreach ($i in 1..$level) { Note "== Down => $(Key $sc 'Down')" }
    Note "== F6 => $(Key $sc 'F6')"
    $said = Key $sc 'Space' 3000
    $dialog = Front-Process
    Note "== Space => process $dialog | $said"
    # Its tree of what is announced, "Announce Start and End For:": each item and its check box.
    $last = $null
    foreach ($i in 1..$Tabs) {
        $said = Key $dialog 'Down'
        Note "== Down => $said"
        if ($said -eq $last -or -not $said) { break }
        $last = $said
    }
    # The read-only text under it, the item's description.
    Note "== Tab => $(Key $dialog 'Tab')"
    Note "== Escape => $(Key $dialog 'Escape' 1500)"
    Note "== Escape => $(Key (Front-Process) 'Escape' 1500)"
    if ((Front-Process) -eq 'SettingsCenter') { Note "== Escape => $(Key 'SettingsCenter' 'Escape' 1500)" }
}
Note "--- front: $(Front-Process)"
'done'

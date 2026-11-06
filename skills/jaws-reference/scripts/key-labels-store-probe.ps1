# Research, never evidence: where JAWS keeps a key label's change. Space's mute status is
# toggled in Manage Key Labels (Settings Center, Keyboard, General) and OK'd, the user settings
# folder compared before and after, then toggled back the same way and compared again: it must
# equal the first. A changed settings file (.jcf, .jkm, .vpf, the files AGENTS.md §1.4 lets us
# read) is shown line by line; any other changed file by its name alone. Run on old-pc with no
# batch recording and the Linux lab shut down.
param([string]$OutDir = (Join-Path $env:USERPROFILE 'sr-harness\probe\key-labels'),
      [int]$Row = 95, [int]$WaitMs = 700)
$lab = Join-Path $env:USERPROFILE 'sr-harness\bin\okur-lab.exe'
$keys = 'C:\Program Files\Okur Lab\okur-lab-keys.exe'
$folders = @((Join-Path $env:APPDATA 'Freedom Scientific\JAWS\2025\Settings'),
             (Join-Path $env:APPDATA 'Freedom Scientific\JAWS\2025\Scripts'))
New-Item -ItemType Directory -Force $OutDir | Out-Null
$log = Join-Path $OutDir 'store.txt'
Remove-Item $log -ErrorAction SilentlyContinue
function Note([string]$Line) { $Line | Add-Content -Path $log -Encoding utf8 }
Add-Type -Namespace KLS -Name W -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);'
function Front-Process {
    $p = 0
    [void][KLS.W]::GetWindowThreadProcessId([KLS.W]::GetForegroundWindow(), [ref]$p)
    (Get-Process -Id $p -ErrorAction SilentlyContinue).ProcessName
}
function Key([string]$Process, [string]$Chord, [int]$Wait = $WaitMs) {
    $p = Start-Process -FilePath $keys -ArgumentList @('--allow', $Process, "`"$Chord`"", "`"wait:$Wait`"") -WindowStyle Hidden -Wait -PassThru
    if ($p.ExitCode -ne 0) { return "(refused: $($p.ExitCode))" }
    (& $lab jaws-history --clear) -join ' | '
}
# Every file under the folders: its path, hash and text where it may be read.
function Snapshot {
    $map = @{}
    foreach ($folder in $folders) {
        Get-ChildItem $folder -Recurse -File -ErrorAction SilentlyContinue | ForEach-Object {
            $readable = $_.Extension -in '.jcf', '.jkm', '.vpf', '.JCF', '.JKM', '.VPF'
            $map[$_.FullName] = [pscustomobject]@{
                Hash = (Get-FileHash $_.FullName).Hash
                Lines = if ($readable) { @(Get-Content $_.FullName -ErrorAction SilentlyContinue) } else { $null }
            }
        }
    }
    $map
}
function Compare-Snapshots($Before, $After, [string]$Title) {
    Note "=== $Title"
    foreach ($path in ($Before.Keys + $After.Keys | Sort-Object -Unique)) {
        $b = $Before[$path]; $a = $After[$path]
        if ($b -and $a -and $b.Hash -eq $a.Hash) { continue }
        Note "changed: $path$(if (-not $b) { ' (new)' } elseif (-not $a) { ' (gone)' })"
        if (($b -and $b.Lines) -or ($a -and $a.Lines)) {
            Compare-Object @($b.Lines) @($a.Lines) | ForEach-Object { Note "  $($_.SideIndicator) $($_.InputObject)" }
        }
    }
}
# Settings Center over Notepad, to Manage Key Labels' list at row $Row, Toggle Mute, OK, OK.
function Toggle-Row {
    if (-not (Get-Process notepad -ErrorAction SilentlyContinue)) { Start-Process notepad; Start-Sleep -Seconds 2 }
    & $lab front 'Notepad' | Out-Null
    Start-Sleep -Milliseconds 500
    & $lab jaws-history --clear | Out-Null
    & $lab keys --allow notepad 'Insert+6' 'wait:4000' | Out-Null
    $sc = Front-Process
    if ($sc -ne 'SettingsCenter') { Note "Settings Center did not open ($sc)"; return $false }
    foreach ($chord in 'Control+E', 'Down', 'Home', 'K', 'Right', 'Down') { Key $sc $chord | Out-Null }
    $said = Key $sc 'Right'
    $found = $false
    foreach ($i in 1..14) { if ((Key $sc 'Down') -match 'Manage Key Labels') { $found = $true; break } }
    if (-not $found) { Note 'Manage Key Labels not reached'; Key $sc 'Escape' | Out-Null; return $false }
    if ((Key $sc 'F6') -notmatch 'Manage Key Labels') { Note 'the button not reached'; Key $sc 'Escape' | Out-Null; return $false }
    Key $sc 'Space' 3000 | Out-Null
    Key $sc 'Home' | Out-Null
    foreach ($i in 2..$Row) { Key $sc 'Down' 250 | Out-Null }
    $row = Key $sc 'Insert+Up'
    Note "row: $row"
    $said = Key $sc 'Tab'
    if ($said -notmatch 'Toggle Mute') { Note "not Toggle Mute: $said"; Key $sc 'Escape' | Out-Null; Key $sc 'Escape' | Out-Null; return $false }
    Note "Space on Toggle Mute => $(Key $sc 'Space' 1000)"
    Note "Shift+Tab => $(Key $sc 'Shift+Tab')"
    Note "Insert+Up => $(Key $sc 'Insert+Up')"
    # OK: Tab past Toggle Mute and Change Label.
    foreach ($i in 1..3) { $said = Key $sc 'Tab' }
    Note "Tab => $said"
    if ($said -notmatch 'OK') { Note 'OK not reached'; Key $sc 'Escape' | Out-Null; Key $sc 'Escape' | Out-Null; return $false }
    Note "Space on OK => $(Key $sc 'Space' 1500)"
    # Settings Center's own OK: Enter in its pane's button would press the button again; back to
    # the tree (F6), then Enter.
    Note "F6 => $(Key $sc 'F6')"
    Note "Enter => $(Key $sc 'Enter' 3000)"
    Note "front: $(Front-Process)"
    $true
}

$first = Snapshot
if (-not (Toggle-Row)) { 'failed to toggle'; exit 1 }
Start-Sleep -Seconds 2
$toggled = Snapshot
Compare-Snapshots $first $toggled 'toggled'
if (-not (Toggle-Row)) { 'failed to toggle back'; exit 1 }
Start-Sleep -Seconds 2
$back = Snapshot
Compare-Snapshots $first $back 'toggled back (should list nothing)'
Note '--- done'
'done'

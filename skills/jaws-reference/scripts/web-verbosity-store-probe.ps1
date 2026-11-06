# Research, never evidence: where JAWS keeps a Configure Web Verbosity Levels change. The
# Medium level's item $Row (15: Banner) is turned in its dialog and OK'd, the user settings folder
# compared before and after, then turned back the same way and compared again: it must equal the
# first. A changed settings file (.jcf, .jkm, .vpf) is shown line by line; any other by its name.
# Run on old-pc with no batch recording and the Linux lab shut down.
param([string]$OutDir = (Join-Path $env:USERPROFILE 'sr-harness\probe\web-verbosity'),
      [int]$Row = 15, [int]$WaitMs = 700)
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
# Settings Center over Notepad, to Configure Web Verbosity Levels' Medium..., its tree's item
# $Row turned by Space, OK, and Settings Center's OK.
function Toggle-Row {
    if (-not (Get-Process notepad -ErrorAction SilentlyContinue)) { Start-Process notepad; Start-Sleep -Seconds 2 }
    & $lab front 'Notepad' | Out-Null
    Start-Sleep -Milliseconds 500
    & $lab jaws-history --clear | Out-Null
    & $lab keys --allow notepad 'Insert+6' 'wait:4000' | Out-Null
    $sc = Front-Process
    if ($sc -ne 'SettingsCenter') { Note "Settings Center did not open ($sc)"; return $false }
    foreach ($chord in 'Control+E', 'Down', 'Home', 'W', 'Right') { Key $sc $chord | Out-Null }
    if ((Key $sc 'Down') -match 'closed') { Key $sc 'Right' | Out-Null }
    Key $sc 'Down' | Out-Null
    if ((Key $sc 'Down') -match 'closed') { Key $sc 'Right' | Out-Null }
    Key $sc 'Down' | Out-Null
    $said = Key $sc 'Down'
    if ($said -notmatch 'Medium') { Note "not Medium...: $said"; Key $sc 'Escape' | Out-Null; return $false }
    Key $sc 'F6' | Out-Null
    Note "Space => $(Key $sc 'Space' 3000)"
    Key $sc 'Home' | Out-Null
    foreach ($i in 2..$Row) { Key $sc 'Down' 300 | Out-Null }
    Note "row: $(Key $sc 'Insert+Up')"
    Note "Space => $(Key $sc 'Space' 1000)"
    foreach ($i in 1..2) { $said = Key $sc 'Tab' }
    if ($said -notmatch '^OK') { Note "OK not reached: $said"; Key $sc 'Escape' | Out-Null; Key $sc 'Escape' | Out-Null; return $false }
    Note "Space on OK => $(Key $sc 'Space' 1500)"
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

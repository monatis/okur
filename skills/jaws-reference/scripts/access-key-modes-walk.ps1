# Research, never evidence: how JAWS's settings file writes the Startup Wizard's Access Key and
# Tutor Messages choices (`[OutputModes]` in DEFAULT.JCF), for Okur's settings redesign
# (docs/references/okur-settings-redesign.md). For each choice asked for, the Startup Wizard is
# opened from JAWS's window (Alt+H, Z), its Verbosity page reached (Alt+N four times), the choice
# made by arrows from the choice it has (radio groups wrap around, so the walk keeps count from
# -Tutor and -Access, the choices old-pc has), and Finish pressed (Enter on the last page);
# then the file's [OutputModes] lines are written down. JAWS's own choices are put back the same
# way at the end, and the file is compared with a copy taken before (written to -OutDir). Run on
# old-pc with no batch recording. -Choices: "access:N" or "tutor:N", N from 0 (the group's first
# radio button), comma separated; -Restore the same for what old-pc had.
param([string]$OutDir = (Join-Path $env:USERPROFILE 'sr-harness\probe\access-key-modes'),
      [string]$Choices = 'access:2,access:3,tutor:2',
      [string]$Restore = 'access:0,tutor:1',
      # The choices the wizard shows when the walk begins, from 0: Tutor Messages' "Announce
      # Menu and Control Help", Access Key's "Off" [the walk of the Startup Wizard].
      [int]$Tutor = 1, [int]$Access = 0)
$script:at = @{ tutor = $Tutor; access = $Access }
$lab = Join-Path $env:USERPROFILE 'sr-harness\bin\okur-lab.exe'
$walk = Join-Path $PSScriptRoot 'jaws-ui-walk.ps1'
$jcf = Join-Path $env:APPDATA 'Freedom Scientific\JAWS\2025\Settings\enu\DEFAULT.JCF'
New-Item -ItemType Directory -Force $OutDir | Out-Null
Copy-Item $jcf (Join-Path $OutDir 'DEFAULT.JCF.before') -Force
$log = Join-Path $OutDir 'walk.txt'
"started $(Get-Date -Format o)" | Set-Content $log
Add-Type -Namespace AKM -Name W -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p); [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);'
function Front-Process {
    $id = 0
    [void][AKM.W]::GetWindowThreadProcessId([AKM.W]::GetForegroundWindow(), [ref]$id)
    (Get-Process -Id $id -ErrorAction SilentlyContinue).ProcessName
}
function Front-Title {
    $text = New-Object System.Text.StringBuilder 512
    [void][AKM.W]::GetWindowText([AKM.W]::GetForegroundWindow(), $text, 512)
    $text.ToString()
}
function Output-Modes {
    $in = $false
    foreach ($line in Get-Content $jcf) {
        if ($line -match '^\[(.+)\]') { $in = ($Matches[1] -eq 'OutputModes'); continue }
        if ($in -and $line.Trim()) { "    $line" }
    }
}
function Open-JawsWindow {
    & $lab front 'Notepad' | Out-Null
    Start-Sleep -Milliseconds 500
    & $lab keys --allow notepad 'Insert+J' 'wait:2500' | Out-Null
    if ((Front-Process) -ne 'jfw') {
        Start-Process -FilePath 'C:\Program Files\Okur Lab\okur-lab-keys.exe' -ArgumentList @('--allow', 'notepad', '"Alt+Tab"', '"wait:1500"') -WindowStyle Hidden -Wait
    }
}
# One choice: the wizard to its Verbosity page, its group (Tutor Messages first, Access Key a
# Tab later) moved from the choice it has to choice N, then Next to the last page and Finish.
function Choose([string]$choice) {
    $group, $n = $choice -split ':'
    Open-JawsWindow
    & powershell -NoProfile -ExecutionPolicy Bypass -File $walk -Process jfw -Chords 'Alt+H,Z' -WaitMs 1000 -Out (Join-Path $OutDir "open-$($choice -replace ':','-').txt") | Out-Null
    Start-Sleep -Seconds 3
    $process = Front-Process
    $title = Front-Title
    if (-not ($process -and $title -like '*Startup Wizard*')) {
        "${choice}: no wizard in front ($process, '$title'); nothing changed" | Add-Content $log
        return
    }
    $tab = if ($group -eq 'access') { 'Tab,' } else { '' }
    $delta = [int]$n - $script:at[$group]
    $move = if ($delta -gt 0) { "Down*$delta," } elseif ($delta -lt 0) { "Up*$(-$delta)," } else { '' }
    $chords = "Alt+N,Alt+N,Alt+N,Alt+N,$($tab)$($move)Alt+N,Alt+N,Enter"
    $script:at[$group] = [int]$n
    & powershell -NoProfile -ExecutionPolicy Bypass -File $walk -Process $process -Chords $chords -WaitMs 1000 -Out (Join-Path $OutDir "walk-$($choice -replace ':','-').txt") | Out-Null
    Start-Sleep -Seconds 3
    "${choice}: front after Finish $(Front-Process) '$(Front-Title)'" | Add-Content $log
    Output-Modes | Add-Content $log
}
if (-not (Get-Process notepad -ErrorAction SilentlyContinue)) { Start-Process notepad; Start-Sleep -Seconds 2 }
"before:" | Add-Content $log
Output-Modes | Add-Content $log
foreach ($choice in ($Choices -split ',' | Where-Object { $_ })) { Choose $choice }
foreach ($choice in ($Restore -split ',' | Where-Object { $_ })) { Choose $choice }
Copy-Item $jcf (Join-Path $OutDir 'DEFAULT.JCF.after') -Force
$same = (Get-FileHash $jcf).Hash -eq (Get-FileHash (Join-Path $OutDir 'DEFAULT.JCF.before')).Hash
"file as before: $same" | Add-Content $log
& $lab front 'Notepad' | Out-Null
"done"

# Research, never evidence: keys into one of JAWS's own dialogs (elevated, so through
# okur-lab-keys with UI Access) and JAWS's speech history after each, one line per key. It learned
# Quick Settings' tree (2026-10-24): "level 1 | Automatic Activation not checked | 1 of 4 | Touch
# Cursor Options". Use navigation keys and Escape only: Space or Enter would change JAWS's
# settings. Example: -Process QuickSettings -Chords 'Tab,Down*40' -AutoExpand
#   -Process  the dialog's process, which the keys may reach (QuickSettings, jfw, ...)
#   -Chords   the keys, in order; "Down*40" repeats a chord
#   -Out      the file written
param([string]$Process = 'QuickSettings', [string[]]$Chords = @('Tab', 'Down*5'),
      [int]$WaitMs = 1200, [string]$Out = "$env:TEMP\tree-walk.txt",
      # After a key whose speech says a group is closed, Right opens it, so a walk with Down
      # goes through every group (Settings Center's may start closed).
      [switch]$AutoExpand)
$lab = Join-Path $env:USERPROFILE 'sr-harness\bin\okur-lab.exe'
$keys = 'C:\Program Files\Okur Lab\okur-lab-keys.exe'
$Chords = @($Chords | ForEach-Object { $_ -split "," } | Where-Object { $_ })
$expanded = foreach ($c in $Chords) {
    if ($c -match '^(.+)\*(\d+)$') { 1..[int]$Matches[2] | ForEach-Object { $Matches[1] } } else { $c }
}
& $lab jaws-history --clear | Out-Null
$lines = foreach ($chord in $expanded) {
    $p = Start-Process -FilePath $keys -ArgumentList @('--allow', $Process, "`"$chord`"", "`"wait:$WaitMs`"") -WindowStyle Hidden -Wait -PassThru
    if ($p.ExitCode -ne 0) { "== $chord => (refused: $($p.ExitCode))"; break }
    $said = (& $lab jaws-history --clear) -join ' | '
    "== $chord => $said"
    if ($AutoExpand -and $said -match '(^| )closed( |$)') {
        $p = Start-Process -FilePath $keys -ArgumentList @('--allow', $Process, '"Right"', "`"wait:$WaitMs`"") -WindowStyle Hidden -Wait -PassThru
        "== Right => " + ((& $lab jaws-history --clear) -join ' | ')
    }
}
$lines | Out-File $Out -Encoding utf8
$lines.Count

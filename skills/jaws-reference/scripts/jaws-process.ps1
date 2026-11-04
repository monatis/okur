<#
.SYNOPSIS
    Stops or starts JAWS on the reference machine, around a live run of Okur.

.DESCRIPTION
    run-scenario.ps1 -Reader okur runs this script elevated (JAWS runs elevated; the lab's UAC
    elevates without asking): -Stop before the run, so that JAWS takes no keys and speaks
    nothing while Okur reads, and -Start after it, so that the next JAWS recording finds JAWS
    as before. Old-pc is the lab: stopping and starting JAWS there is allowed. Never run this
    on the dev box, where JAWS is the user's.

    -StopOkur stops Okur's own programs (okur, okur-sr, okur-ui) before a JAWS recording: the
    Store build starts at sign-in (old-pc, 2026-11-04, after a power outage), and beside JAWS
    its hook takes the Okur key's chords and its speech confuses the silence detection. With
    UI Access it runs above an ordinary process, hence elevated.
#>
param(
    [switch]$Stop,
    [switch]$Start,
    [switch]$StopOkur,
    [string]$Exe = 'C:\Program Files\Freedom Scientific\JAWS\2025\jfw.exe'
)
if ($Stop) {
    # JAWS and the processes it runs beside it, all under Freedom Scientific's folder.
    $folder = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $Exe))
    Get-Process | Where-Object { $_.Path -and $_.Path.StartsWith($folder, [StringComparison]::OrdinalIgnoreCase) } |
        Stop-Process -Force -ErrorAction SilentlyContinue
}
if ($StopOkur) {
    Get-Process okur, okur-sr, okur-ui -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
}
if ($Start -and -not (Get-Process jfw -ErrorAction SilentlyContinue)) {
    Start-Process -FilePath $Exe
}

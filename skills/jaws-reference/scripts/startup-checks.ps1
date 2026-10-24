<#
.SYNOPSIS
    The secure-screens proposal's checks that need no certificate (M0's E1 and M1), run on
    old-pc between batches, each result written to a log.

.DESCRIPTION
    docs/references/startup-and-secure-screens-proposal.md, "Live checks on old-pc". Waits for
    -After's batch.done (a check must not run during a capture), then, with the release
    programs in -Dir:
      E1   okur doctor; okur-sr.exe --doctor-exit (does an unsigned UI Access program start,
           and with which token?)
      M1a  okur run twice: the first quits for the second; okur quit ends it
      M1b  okur-sr-no-uiaccess --ease-of-access twice: one process
      M1c  --sign-in with both Shift keys held: no process
    Every Okur process started from -Dir is ended afterwards, and <OutputDir>\batch.done is
    written last, so a queued batch can follow (queue-batch.ps1 -After). It changes nothing
    on the machine (no registry, certificate or install) but Okur's own settings folder: the
    first Okur start for a user without %APPDATA%\Okur makes it and opens the Startup Wizard,
    which ending the processes closes, saving nothing.
#>
param(
    [Parameter(Mandatory = $true)][string]$After,
    [Parameter(Mandatory = $true)][string]$OutputDir,
    [string]$Dir = 'C:\Users\ailabs\sr-harness\okur-release',
    [int]$MaxWaitMinutes = 360
)
$ErrorActionPreference = 'Continue'
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$log = Join-Path $OutputDir 'checks.log'
$done = Join-Path $OutputDir 'batch.done'
Remove-Item $done -ErrorAction SilentlyContinue
function Log([string]$line) { "$(Get-Date -Format HH:mm:ss) $line" | Out-File $log -Append -Encoding utf8 }

$deadline = (Get-Date).AddMinutes($MaxWaitMinutes)
while (-not (Test-Path (Join-Path $After 'batch.done'))) {
    if ((Get-Date) -gt $deadline) { Log "gave up waiting for $After"; exit 1 }
    Start-Sleep -Seconds 10
}

Add-Type @'
using System.Runtime.InteropServices;
public static class StartupChecksKeys {
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, System.UIntPtr extra);
}
'@

# The Okur processes started from $Dir, by their path.
function Get-OurProcesses {
    Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -and $_.ExecutablePath.StartsWith($Dir, [StringComparison]::OrdinalIgnoreCase) }
}
function Stop-OurProcesses {
    foreach ($p in Get-OurProcesses) { Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue }
}
function Count([string]$name) { @(Get-OurProcesses | Where-Object { $_.Name -eq $name }).Count }

$okur = Join-Path $Dir 'okur.exe'
$sr = Join-Path $Dir 'okur-sr.exe'
$srPlain = Join-Path $Dir 'okur-sr-no-uiaccess.exe'
try {
    Log "=== startup checks, programs in $Dir"
    foreach ($exe in $okur, $sr, $srPlain) {
        Log "$((Split-Path -Leaf $exe)) sha256 $((Get-FileHash -LiteralPath $exe).Hash.ToLower())"
    }

    Log '--- E1: okur doctor'
    & $okur doctor 2>&1 | ForEach-Object { Log "    $_" }

    Log '--- E1: okur-sr.exe --doctor-exit (unsigned, uiAccess=true)'
    $report = Join-Path $env:TEMP 'okur-e1a.txt'
    Remove-Item $report -ErrorAction SilentlyContinue
    try {
        $p = Start-Process $sr -ArgumentList '--doctor-exit', $report -PassThru -Wait -ErrorAction Stop
        Log "    started; exit code $($p.ExitCode)"
        if (Test-Path $report) { Get-Content $report | ForEach-Object { Log "    $_" } } else { Log '    no report written' }
    } catch {
        Log "    did not start: $($_.Exception.Message)"
    }

    Log '--- M1a: okur run twice'
    $first = Start-Process $okur -ArgumentList 'run' -PassThru -WindowStyle Minimized
    Start-Sleep -Seconds 10
    & $okur doctor 2>&1 | Select-String 'Okur on this desktop' | ForEach-Object { Log "    with one: $_" }
    $second = Start-Process $okur -ArgumentList 'run' -PassThru -WindowStyle Minimized
    Start-Sleep -Seconds 10
    Log "    first ended: $($first.HasExited) (exit code $(if ($first.HasExited) { $first.ExitCode } else { '-' })); second running: $(-not $second.HasExited); okur.exe processes: $(Count 'okur.exe')"
    & $okur quit 2>&1 | ForEach-Object { Log "    okur quit: $_" }
    Start-Sleep -Seconds 6
    Log "    second ended: $($second.HasExited); okur.exe processes: $(Count 'okur.exe')"
    & $okur doctor 2>&1 | Select-String 'Okur on this desktop' | ForEach-Object { Log "    after quit: $_" }
    Stop-OurProcesses

    Log '--- M1b: --ease-of-access twice'
    Start-Process $srPlain -ArgumentList '--ease-of-access' | Out-Null
    Start-Sleep -Seconds 8
    Start-Process $srPlain -ArgumentList '--ease-of-access' | Out-Null
    Start-Sleep -Seconds 8
    Log "    okur-sr-no-uiaccess.exe processes: $(Count 'okur-sr-no-uiaccess.exe')"
    & $okur quit 2>&1 | ForEach-Object { Log "    okur quit: $_" }
    Start-Sleep -Seconds 6
    Log "    after quit: $(Count 'okur-sr-no-uiaccess.exe')"
    Stop-OurProcesses

    Log '--- M1c: --sign-in with both Shift keys held'
    $down = 0; $up = 2
    try {
        [StartupChecksKeys]::keybd_event(0xA0, 0x2A, $down, [UIntPtr]::Zero)
        [StartupChecksKeys]::keybd_event(0xA1, 0x36, $down, [UIntPtr]::Zero)
        Start-Process $srPlain -ArgumentList '--sign-in' | Out-Null
        Start-Sleep -Seconds 4
    } finally {
        [StartupChecksKeys]::keybd_event(0xA1, 0x36, $up, [UIntPtr]::Zero)
        [StartupChecksKeys]::keybd_event(0xA0, 0x2A, $up, [UIntPtr]::Zero)
    }
    Start-Sleep -Seconds 4
    Log "    okur-sr-no-uiaccess.exe processes: $(Count 'okur-sr-no-uiaccess.exe')"
} finally {
    Stop-OurProcesses
    Log '=== done'
    "done $(Get-Date -Format o)" | Out-File $done -Encoding utf8
}

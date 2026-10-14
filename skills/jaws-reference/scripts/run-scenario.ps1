[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [string]$ScenarioFile,
    [int]$Repetitions = 5,
    [string]$OutputPath,
    [string]$TargetProcess,
    [string]$SrCli = (Join-Path $env:USERPROFILE "sr-harness\bin\sr-cli.exe"),
    # When set, each trial records an accessibility event trace (sr-cli trace) to
    # <TraceDir>\<scenario-id>-trial<N>.jsonl, aligned with steps by unix-ms timestamps.
    [string]$TraceDir,
    # The recording SAPI voice's log (install-speech-recorder.ps1). While JAWS speaks through that
    # voice, each step also records what JAWS sent the synthesizer: the text after JAWS's symbol
    # and punctuation processing, which the speech history does not show.
    [string]$SpokenLog = (Join-Path $env:USERPROFILE "sr-harness\recorder\spoken.jsonl"),
    [switch]$Quiet
)

<#
Evidence capture (format_version 2). Runs on the reference machine only.

Step types:
  key          Readable key chord, sent as scan-coded SendInput with the extended-key flag
               where the physical key has one: "Down", "H", "Shift+H", "Ctrl+Alt+Right",
               "Insert+Down", "Insert+F7". Modifiers: Ctrl, Alt, Shift, Insert, Win.
               An optional "repeat": N presses the chord N times quickly ("twice quickly").
               "capture_clipboard": true stores the clipboard after the step (observation.clipboard).
  jaws_script  JAWS script name dispatched through FSAPI RunScript.
  keyboard     Legacy .NET SendKeys string (kept so scenarios 01-30 stay reproducible).

Settling: after each action the default audio endpoint is metered (sr-cli wait-silence; JAWS is
the only audio source on the reference machine) until it has been silent for quiet_ms (default
500) after speech, or no speech started within 800 ms, or settle_max_ms (default 10000) elapses.
The speech history is then copied exactly once; polling it is impossible because JAWS speaks and
records a "Copy speech history to clipboard" confirmation on every copy. Every trial reloads the
page, waits for the page-load speech to finish, applies the reset keys, waits again, then stops
speech and clears the history, so no trial inherits speech from setup.
#>

$ErrorActionPreference = "Stop"
$sw = [System.Diagnostics.Stopwatch]::StartNew()

if (-not (Test-Path $ScenarioFile)) {
    Write-Error "Scenario file not found: $ScenarioFile"
    exit 1
}

$scenario = Get-Content $ScenarioFile -Raw | ConvertFrom-Json
$scenarioId = if ($scenario.id) { $scenario.id } else { [System.IO.Path]::GetFileNameWithoutExtension($ScenarioFile) }
$targetProc = if ($TargetProcess) { $TargetProcess } elseif ($scenario.target_app) { $scenario.target_app } else { "chrome" }

if (-not ([System.Management.Automation.PSTypeName]"SrKeyInput").Type) {
    Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class SrKeyInput {
    [StructLayout(LayoutKind.Sequential)] struct MOUSEINPUT { public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Sequential)] struct KEYBDINPUT { public ushort wVk, wScan; public uint dwFlags, time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Explicit)] struct UNION { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
    [StructLayout(LayoutKind.Sequential)] struct INPUT { public uint type; public UNION u; }
    [DllImport("user32.dll", SetLastError = true)] static extern uint SendInput(uint n, INPUT[] inputs, int size);
    [DllImport("user32.dll")] static extern uint MapVirtualKey(uint code, uint mapType);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder s, int n);
    const uint KEYEVENTF_EXTENDEDKEY = 0x1, KEYEVENTF_KEYUP = 0x2;

    public static bool Key(ushort vk, bool extended, bool up) {
        INPUT[] input = new INPUT[1];
        input[0].type = 1;
        input[0].u.ki.wVk = vk;
        input[0].u.ki.wScan = (ushort)MapVirtualKey(vk, 0);
        input[0].u.ki.dwFlags = (extended ? KEYEVENTF_EXTENDEDKEY : 0) | (up ? KEYEVENTF_KEYUP : 0);
        return SendInput(1, input, Marshal.SizeOf(typeof(INPUT))) == 1;
    }
}
"@
}
Add-Type -AssemblyName System.Windows.Forms

# name -> @(virtual key, extended)
$KeyTable = @{
    'down'=@(0x28,$true); 'up'=@(0x26,$true); 'left'=@(0x25,$true); 'right'=@(0x27,$true);
    'home'=@(0x24,$true); 'end'=@(0x23,$true); 'pageup'=@(0x21,$true); 'pagedown'=@(0x22,$true);
    'insert'=@(0x2D,$true); 'delete'=@(0x2E,$true); 'tab'=@(0x09,$false); 'enter'=@(0x0D,$false);
    'escape'=@(0x1B,$false); 'esc'=@(0x1B,$false); 'space'=@(0x20,$false); 'backspace'=@(0x08,$false);
    'semicolon'=@(0xBA,$false); 'apostrophe'=@(0xDE,$false);
    'ctrl'=@(0xA2,$false); 'alt'=@(0xA4,$false); 'shift'=@(0xA0,$false); 'win'=@(0x5B,$true);
    'capslock'=@(0x14,$false); 'scrolllock'=@(0x91,$false)
}

function Resolve-Key([string]$name) {
    $n = $name.Trim().ToLowerInvariant()
    if ($KeyTable.ContainsKey($n)) { return $KeyTable[$n] }
    if ($n -match '^f([1-9]|1[0-2])$') { return @((0x6F + [int]$Matches[1]), $false) }
    if ($n -match '^[a-z0-9]$') { return @([int][char]$n.ToUpperInvariant(), $false) }
    throw "Unknown key name '$name'"
}

function Send-Chord([string]$chord) {
    $parts = $chord.Split('+') | Where-Object { $_ -ne '' }
    $keys = @($parts | ForEach-Object { ,(Resolve-Key $_) })
    $ok = $true
    foreach ($k in $keys) { $ok = $ok -and [SrKeyInput]::Key([uint16]$k[0], $k[1], $false); Start-Sleep -Milliseconds 30 }
    [array]::Reverse($keys)
    foreach ($k in $keys) { $ok = $ok -and [SrKeyInput]::Key([uint16]$k[0], $k[1], $true); Start-Sleep -Milliseconds 30 }
    return $ok
}

function Get-ForegroundTitle {
    $sb = New-Object System.Text.StringBuilder 512
    [void][SrKeyInput]::GetWindowText([SrKeyInput]::GetForegroundWindow(), $sb, 512)
    return $sb.ToString()
}

function Focus-Target([string]$procName) {
    $proc = Get-Process $procName -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero } | Select-Object -First 1
    if (-not $proc) { throw "No '$procName' window found." }
    [void][SrKeyInput]::ShowWindow($proc.MainWindowHandle, 9)
    [void][SrKeyInput]::SetForegroundWindow($proc.MainWindowHandle)
    Start-Sleep -Milliseconds 250
}

$jaws = New-Object -ComObject "FreedomSci.JawsApi"

$CopyConfirmation = 'Copy speech history to clipboard'

# Blocks until JAWS's audio output is silent (sr-cli wait-silence on the default endpoint;
# JAWS is the only audio source on the reference machine). Returns the parsed SilenceReport.
function Wait-Silence([int]$OnsetMs = 800, [int]$QuietMs = 500, [int]$MaxMs = 4000) {
    $out = & $SrCli wait-silence --onset-ms $OnsetMs --quiet-ms $QuietMs --max-ms $MaxMs
    if ($LASTEXITCODE -ne 0) { throw "sr-cli wait-silence failed: $out" }
    return ($out | Select-Object -Last 1 | ConvertFrom-Json)
}

# Copies the speech history exactly once, then silences and clears JAWS's spoken copy
# confirmation (it is queued as soon as the clipboard changes) so it cannot reach the next step.
function Read-SpeechHistoryOnce {
    $sentinel = "__SR_SENTINEL_$([guid]::NewGuid().ToString('N').Substring(0,8))__"
    Set-Clipboard -Value $sentinel
    [void]$jaws.RunScript("CopySpeechHistoryToClipboard")
    $text = ""
    for ($i = 0; $i -lt 40; $i++) {
        Start-Sleep -Milliseconds 25
        $clip = Get-Clipboard -Raw -ErrorAction SilentlyContinue
        if ($null -ne $clip -and $clip -ne $sentinel) { $text = $clip; break }
    }
    Start-Sleep -Milliseconds 150
    $jaws.StopSpeech()
    [void]$jaws.RunScript("ClearSpeechHistory")
    Start-Sleep -Milliseconds 80
    return $text
}

# The spoken log grows by one JSON line per synthesizer call; a step reads what was appended
# while it ran.
function Get-SpokenOffset {
    if (Test-Path $SpokenLog) { return (Get-Item $SpokenLog).Length }
    return [long]0
}

function Read-SpokenSince([long]$offset) {
    if (-not (Test-Path $SpokenLog)) { return @() }
    $fs = [System.IO.File]::Open($SpokenLog, "Open", "Read", "ReadWrite")
    try {
        if ($fs.Length -lt $offset) { $offset = 0 }
        [void]$fs.Seek($offset, "Begin")
        $text = (New-Object System.IO.StreamReader($fs, [System.Text.Encoding]::UTF8)).ReadToEnd()
    } finally { $fs.Dispose() }
    return @($text -split "`n" | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json })
}

# A synthesizer call's text: its spoken, spelled and pronounced fragments.
function Get-SpokenText($record) {
    return ((@($record.frags | Where-Object { $_.action -in @("speak", "spell", "pronounce") } | ForEach-Object { $_.text })) -join " ")
}

function Invoke-Step($type, [string]$command, [int]$repeat = 1) {
    switch ($type) {
        'key' {
            # "repeat": the chord pressed again 120 ms after each release, for JAWS's
            # "twice quickly" commands.
            $ok = $true
            for ($i = 0; $i -lt [Math]::Max(1, $repeat); $i++) {
                if ($i -gt 0) { Start-Sleep -Milliseconds 120 }
                $ok = (Send-Chord $command) -and $ok
            }
            return $ok
        }
        'jaws_script' { return [bool]$jaws.RunScript($command) }
        'keyboard'    { [System.Windows.Forms.SendKeys]::SendWait($command); return $true }
        default       { throw "Unsupported step type '$type'" }
    }
}

# --- Provenance -------------------------------------------------------------------------
function Get-Sha256([string]$path) { (Get-FileHash -Algorithm SHA256 -Path $path).Hash.ToLowerInvariant() }
$fixturePath = $null
if ($scenario.fixture -match '^file:///(.+)$') { $fixturePath = [uri]::UnescapeDataString($Matches[1]) -replace '/', '\' }
$chromeExe = @("C:\Program Files\Google\Chrome\Application\chrome.exe", "C:\Program Files (x86)\Google\Chrome\Application\chrome.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
$jfwExe = "C:\Program Files\Freedom Scientific\JAWS\2025\jfw.exe"

# JAWS's user settings files, as sections of key=value pairs. A JCF holds only the options
# changed from JAWS's defaults, so the evidence says which non-default settings were active
# (e.g. KeyboardType=Laptop, TypingEcho=0, AutoFormsMode=0 on old-pc).
function Read-JawsSettings([string]$name) {
    $path = Join-Path $env:APPDATA "Freedom Scientific\JAWS\2025\Settings\enu\$name"
    if (-not (Test-Path $path)) { return $null }
    $ini = [ordered]@{}
    $section = $null
    foreach ($line in Get-Content $path -Encoding UTF8) {
        $t = $line.Trim()
        if ($t -eq '' -or $t.StartsWith(';')) { continue }
        if ($t -match '^\[(.+)\]$') { $section = [ordered]@{}; $ini[$Matches[1]] = $section; continue }
        if ($null -ne $section -and $t -match '^([^=]+)=(.*)$') { $section[$Matches[1]] = $Matches[2] }
    }
    return $ini
}
$jawsSettings = [ordered]@{ 'DEFAULT.JCF' = Read-JawsSettings 'DEFAULT.JCF' }
if ($targetProc -eq 'chrome') { $jawsSettings['Chrome.JCF'] = Read-JawsSettings 'Chrome.JCF' }
# Voice profiles hold the synthesizer, voice and punctuation level (e.g. Punctuation=0, None).
Get-ChildItem (Join-Path $env:APPDATA 'Freedom Scientific\JAWS\2025\Settings\voiceprofiles') -Filter *.VPF -ErrorAction SilentlyContinue |
    ForEach-Object { $jawsSettings["voiceprofiles/$($_.Name)"] = Read-JawsSettings "..\voiceprofiles\$($_.Name)" }

$expectedTitle = $null
if ($fixturePath -and (Test-Path $fixturePath)) {
    $html = Get-Content -Raw $fixturePath
    if ($html -match '(?is)<title>\s*(.*?)\s*</title>') { $expectedTitle = $Matches[1] }
}

$evidence = [ordered]@{
    format_version = 2
    scenario_id = $scenarioId
    scenario_version = if ($scenario.version) { $scenario.version } else { "1.0.0" }
    timestamp = (Get-Date -Format "o")
    provenance = [ordered]@{
        capture_script_sha256 = Get-Sha256 $PSCommandPath
        scenario_sha256 = Get-Sha256 $ScenarioFile
        fixture_sha256 = if ($fixturePath -and (Test-Path $fixturePath)) { Get-Sha256 $fixturePath } else { $null }
        host = $env:COMPUTERNAME
        os_build = (cmd /c ver | Out-String).Trim()
        keyboard_layout = [System.Windows.Forms.InputLanguage]::CurrentInputLanguage.LayoutName
        # Time and date announcements follow the user locale and time zone.
        culture = (Get-Culture).Name
        utc_offset_minutes = [int][TimeZoneInfo]::Local.GetUtcOffset((Get-Date)).TotalMinutes
        chrome_version = if ($chromeExe) { (Get-Item $chromeExe).VersionInfo.FileVersion } else { $null }
        jaws_version = if (Test-Path $jfwExe) { (Get-Item $jfwExe).VersionInfo.ProductVersion } else { $null }
        jaws_settings = $jawsSettings
    }
    environment = [ordered]@{
        os_version = [System.Environment]::OSVersion.VersionString
        jaws_version = if (Test-Path $jfwExe) { $vi = (Get-Item $jfwExe).VersionInfo; "$($vi.ProductVersion) (File: $($vi.FileVersion))" } else { "Unknown" }
        target_process = $targetProc
        interactive_session = ((Get-Process -Id $PID).SessionId -ne 0)
    }
    speech_recorder_log = if (Test-Path $SpokenLog) { $SpokenLog } else { $null }
    fixture = $scenario.fixture
    repetitions_requested = $Repetitions
    trials = @()
}

# Desktop scenarios carry a "launch" block: a fixture app started fresh for every trial and
# found by its window title. Browser scenarios open the fixture page once and reload it.
$launch = $scenario.launch
if ($launch) { $expectedTitle = $launch.window_title }

function Find-WindowByTitle([string]$title) {
    Get-Process -ErrorAction SilentlyContinue |
        Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero -and $_.MainWindowTitle -eq $title } |
        Select-Object -First 1
}

function Start-DesktopFixture {
    $old = Find-WindowByTitle $launch.window_title
    if ($old) { Stop-Process -Id $old.Id -Force; Start-Sleep -Milliseconds 500 }
    Start-Process powershell -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $fixturePath) | Out-Null
    for ($i = 0; $i -lt 100; $i++) {
        $p = Find-WindowByTitle $launch.window_title
        if ($p) {
            [void][SrKeyInput]::ShowWindow($p.MainWindowHandle, 9)
            [void][SrKeyInput]::SetForegroundWindow($p.MainWindowHandle)
            return $true
        }
        Start-Sleep -Milliseconds 100
    }
    return $false
}

if ($scenario.fixture -and -not $launch) {
    [System.Diagnostics.Process]::Start($targetProc, $scenario.fixture) | Out-Null
    # A cold browser start takes longer than a new tab: wait for the fixture's window.
    for ($i = 0; $i -lt 100; $i++) {
        $w = Get-Process $targetProc -ErrorAction SilentlyContinue |
            Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero -and (-not $expectedTitle -or $_.MainWindowTitle.StartsWith($expectedTitle)) }
        if ($w) { break }
        Start-Sleep -Milliseconds 200
    }
    Start-Sleep -Milliseconds 2000
}

$traceProc = $null
try {
for ($t = 1; $t -le $Repetitions; $t++) {
    $trialResult = [ordered]@{
        trial_index = $t
        timestamp = (Get-Date -Format "o")
        foreground_window_start = $null
        setup_settled = $true
        step_results = @()
        trial_success = $true
    }

    # The trace starts before setup, so replay knows the window and focus that launch and
    # reset produced, as JAWS does when step 1 begins.
    $traceProc = $null
    $traceStop = $null
    if ($TraceDir) {
        New-Item -ItemType Directory -Force -Path $TraceDir | Out-Null
        $tracePath = Join-Path $TraceDir "$scenarioId-trial$t.jsonl"
        $traceStop = Join-Path $env:TEMP "sr-trace-stop-$([guid]::NewGuid().ToString('N'))"
        $traceProc = Start-Process -FilePath $SrCli -ArgumentList @('trace', '--out', $tracePath, '--stop-file', $traceStop) -WindowStyle Hidden -PassThru
        Start-Sleep -Milliseconds 400
        $trialResult.trace_file = Split-Path -Leaf $tracePath
    }
    # Identical setup for every trial: reload (or relaunch), let load speech finish, reset,
    # settle, clear.
    if ($launch) {
        if (-not (Start-DesktopFixture)) { $trialResult.trial_success = $false }
        # A launch reads the window and the focused control, which takes long for a full edit.
        $loadQuiet = Wait-Silence -OnsetMs 1500 -QuietMs 700 -MaxMs 20000
    } else {
        Focus-Target $targetProc
        [void](Send-Chord "Escape"); Start-Sleep -Milliseconds 100
        [void](Send-Chord "Ctrl+R")
        # JAWS starts Say All on page load by default; the reset keys interrupt it, so the
        # load wait is bounded and only the post-reset silence decides whether setup settled.
        $loadQuiet = Wait-Silence -OnsetMs 1500 -QuietMs 700 -MaxMs 3000
    }
    # An explicit (possibly empty) reset.keys list wins; browser scenarios default to Ctrl+Home.
    $hasResetKeys = $scenario.reset -and ($scenario.reset.PSObject.Properties.Name -contains 'keys')
    $resetKeys = if ($hasResetKeys) { @($scenario.reset.keys) } else { @("Ctrl+Home") }
    $resetQuiet = $loadQuiet
    foreach ($rk in $resetKeys) {
        [void](Send-Chord $rk)
        $resetQuiet = Wait-Silence -OnsetMs 800 -QuietMs 500 -MaxMs 10000
    }
    $jaws.StopSpeech()
    [void]$jaws.RunScript("ClearSpeechHistory")
    Start-Sleep -Milliseconds 80
    $trialResult.page_load_settled = $loadQuiet.settled
    $trialResult.setup_settled = $resetQuiet.settled
    if (-not $resetQuiet.settled) { $trialResult.trial_success = $false }
    $trialResult.foreground_window_start = Get-ForegroundTitle
    if ($expectedTitle -and -not $trialResult.foreground_window_start.StartsWith($expectedTitle)) {
        $trialResult.trial_success = $false
    }


    foreach ($step in $scenario.steps) {
        $maxMs = if ($step.settle_max_ms) { [int]$step.settle_max_ms } else { 10000 }
        $quietMs = if ($step.quiet_ms) { [int]$step.quiet_ms } else { 500 }

        # Start every step from a silent baseline, so residual audio from the previous step
        # (e.g. the tail of the copy confirmation) cannot be mistaken for this step's speech.
        $baseline = Wait-Silence -OnsetMs 150 -QuietMs 300 -MaxMs 3000
        [void]$jaws.RunScript("ClearSpeechHistory")
        $spokenStart = Get-SpokenOffset

        $tStart = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
        $dispatchOk = Invoke-Step $step.type $step.command ([int]$step.repeat)
        $observed = Wait-Silence -OnsetMs 800 -QuietMs $quietMs -MaxMs $maxMs
        # Read before the history copy, whose confirmation JAWS also speaks.
        $spokenRecords = Read-SpokenSince $spokenStart
        # "capture_clipboard": what the step copied (e.g. Ctrl+C in the Virtual Viewer), read
        # before the speech history copy replaces it.
        $clipboard = if ($step.capture_clipboard) { Get-Clipboard -Raw -ErrorAction SilentlyContinue } else { $null }
        $transcript = Read-SpeechHistoryOnce
        $tEnd = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
        if ($transcript -match "unknown script call" -or $transcript -match "unhandled script call") { $dispatchOk = $false }
        # A leaked copy confirmation, or a Windows notification JAWS announced mid-step (seen
        # in 105 v2 trial 1), means the history was contaminated: fail, never trim.
        $contaminated = $transcript.Contains($CopyConfirmation) -or $transcript.Contains('New notification from')
        # Audio with an empty history entry is recorded, not failed: JAWS speaks a space
        # character aloud but logs an empty history entry (verified 2026-10-14, 107). The
        # silent baseline above is what prevents the copy from racing ahead of speech.
        $audioWithoutText = $observed.heard_audio -and [string]::IsNullOrEmpty($transcript)

        $stepSuccess = $dispatchOk -and $baseline.settled -and $observed.settled -and -not $contaminated
        if (-not $stepSuccess) { $trialResult.trial_success = $false }

        $trialResult.step_results += [ordered]@{
            step_id = $step.id
            t_start_ms = $tStart
            t_end_ms = $tEnd
            action = [ordered]@{ type = $step.type; command = $step.command; repeat = [Math]::Max(1, [int]$step.repeat) }
            observation = [ordered]@{
                speech_raw = $transcript
                spoken = @($spokenRecords | ForEach-Object { Get-SpokenText $_ })
                spoken_records = $spokenRecords
                clipboard = $clipboard
                dispatch_ok = $dispatchOk
                settled = $observed.settled
                heard_audio = $observed.heard_audio
                settle_ms = $observed.elapsed_ms
                contaminated = $contaminated
                audio_without_text = $audioWithoutText
                baseline_settled = $baseline.settled
            }
            step_success = $stepSuccess
        }
    }
    if ($traceProc) {
        New-Item -ItemType File -Force -Path $traceStop | Out-Null
        if (-not $traceProc.WaitForExit(15000)) { $traceProc.Kill(); $trialResult.trial_success = $false }
        Remove-Item $traceStop -ErrorAction SilentlyContinue
        if (Test-Path $tracePath) { $trialResult.trace_sha256 = Get-Sha256 $tracePath } else { $trialResult.trial_success = $false }
    }
    $evidence.trials += $trialResult
}
} finally {
    # An exception mid-trial (e.g. no browser window) must not leave the tracer running: it
    # holds the trace file and the harness binary.
    if ($traceProc -and -not $traceProc.HasExited) { $traceProc.Kill() }
}

# Close what this run opened: the desktop fixture app, or the browser tab.
if ($launch) {
    $p = Find-WindowByTitle $launch.window_title
    if ($p) { Stop-Process -Id $p.Id -Force }
} else {
    Focus-Target $targetProc
    [void](Send-Chord "Ctrl+W")
}

$sw.Stop()
$evidence.total_elapsed_ms = $sw.ElapsedMilliseconds
$jsonEvidence = $evidence | ConvertTo-Json -Depth 12

if ($OutputPath) {
    $outDir = [System.IO.Path]::GetDirectoryName($OutputPath)
    if ($outDir -and -not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir -Force | Out-Null }
    [System.IO.File]::WriteAllText($OutputPath, $jsonEvidence, (New-Object System.Text.UTF8Encoding $false))
}
if (-not $Quiet) { Write-Output $jsonEvidence }

$failed = @($evidence.trials | Where-Object { -not $_.trial_success }).Count
if ($failed -eq 0) { exit 0 } else { exit 1 }

[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [string]$ScenarioFile,
    [int]$Repetitions = 5,
    [string]$OutputPath,
    [string]$TargetProcess,
    [string]$Okur = (Join-Path $env:USERPROFILE "sr-harness\bin\okur.exe"),
    # When set, each trial records an accessibility event trace (okur trace) to
    # <TraceDir>\<scenario-id>-trial<N>.jsonl, aligned with steps by unix-ms timestamps.
    [string]$TraceDir,
    # With TraceDir: the tracer also logs each event's read, its stages timed, to
    # <TraceDir><scenario-id>-trial<N>.log (okur trace --debug-log), for the performance
    # review's measurements. Off by default: logging could shift a trace's timing.
    [switch]$TraceTimings,
    # The recording SAPI voice's log (install-speech-recorder.ps1). While JAWS speaks through that
    # voice, each step also records what JAWS sent the synthesizer: the text after JAWS's symbol
    # and punctuation processing, which the speech history does not show.
    [string]$SpokenLog = (Join-Path $env:USERPROFILE "sr-harness\recorder\spoken.jsonl"),
    # Who reads: JAWS (evidence), or Okur (a live run, never evidence: JAWS is stopped for it,
    # Okur runs with the reference machine's settings and speaks through OkurVoice, and each
    # step reads Okur's speech log as it reads JAWS's speech history). Compare a run with
    # `okur live-parity`.
    [ValidateSet('jaws', 'okur')]
    [string]$Reader = 'jaws',
    [string]$OkurVoice = 'SR Speech Recorder',
    # With -Reader okur: Okur's debug log (keys, engine decisions, OS events, speech), for
    # finding why a live step differs.
    [string]$OkurDebugLog,
    # The lab's key sender with UI Access (crates/okur/src/bin/okur-lab-keys.rs), signed with the
    # development certificate and copied here once by the owner: key_ui_access steps use it.
    [string]$LabKeys = 'C:Program FilesOkur Labokur-lab-keys.exe',
    [switch]$Quiet
)

<#
Evidence capture (format_version 2). Runs on the reference machine only.

Step types:
  key          Readable key chord, sent as scan-coded SendInput with the extended-key flag
               where the physical key has one: "Down", "H", "Shift+H", "Ctrl+Alt+Right",
               "Insert+Down", "Insert+F7". Modifiers: Ctrl, Alt, Shift, Insert, Win.
               The keypad as with Num Lock off: "Numpad5", "Numpad4", "Insert+Numpad5",
               "NumpadPlus"; "Comma", "Period", "Slash" by their US places ($ScanTable).
               An optional "repeat": N presses the chord N times quickly ("twice quickly").
               "capture_clipboard": true stores the clipboard after the step (observation.clipboard).
               "copies_speech_history": true for a key that copies JAWS's speech history itself
               (Ctrl+H in the Speech History): its own "Copy speech history to clipboard" is
               its speech, not a leaked copy confirmation.
               "during": [{"after_ms": 3000, "command": "Ctrl"}] presses chords while the
               step's speech goes on, each after_ms after the one before (Say All stopped or
               moved by Left and Right); the step settles after the last one.
  text         Characters typed as Unicode input, whatever the keyboard layout:
               "%TEMP%\sr-dialogs" into a file dialog's name field.
  jaws_script  JAWS script name dispatched through FSAPI RunScript.
  jaws_function  JAWS function call through FSAPI RunFunction: TypeKey("Down") for keys inside
               JAWS's own dialogs (Links List, Find), which run elevated and refuse our keys.
  Any step may say "history": false: JAWS's speech history is neither cleared nor read for it,
               and the step keeps only the speech recorder's text (steps in Keyboard Help, which
               describes the history scripts instead of running them).
  key_ui_access  A key sent with UI Access by okur-lab-keys.exe (-LabKeys): it reaches JAWS's
               modal dialogs, which TypeKey closes (559/s2). Needs the signed program under
               Program Files.
  key_down     A key pressed and held across the following steps ("Alt" through several
               Tabs); key_up releases it. Held keys are released when a trial ends.
  keyboard     Legacy .NET SendKeys string (kept so scenarios 01-30 stay reproducible).
Notifications: a Windows notification JAWS announces in a step fails it, as a stray toast
spoils the transcript; a scenario about notifications sets "expects_notifications": true.
Reset: "reset": {"keys": [...]} runs before each trial; "window_title" names the window the
reset leaves in front when it is not the launched one (a trial fails if another is in front).

Settling: after each action the default audio endpoint is metered (okur wait-silence; JAWS is
the only audio source on the reference machine) until it has been silent for quiet_ms (default
500) after speech, or no speech started within 800 ms, or settle_max_ms (default 10000) elapses.
The speech history is then copied exactly once; polling it is impossible because JAWS speaks and
records a "Copy speech history to clipboard" confirmation on every copy. JAWS queues that
confirmation behind the speech it still holds, so the step's recorder records end where it
begins (observation.spoken_end, Read-SpeechHistoryOnce). Every trial reloads the
page, waits for the page-load speech to finish, applies the reset keys, waits again, then stops
speech and clears the history, so no trial inherits speech from setup.
#>

$ErrorActionPreference = "Stop"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
. (Join-Path $PSScriptRoot 'lab-chrome.ps1')

if (-not (Test-Path $ScenarioFile)) {
    Write-Error "Scenario file not found: $ScenarioFile"
    exit 1
}

$scenario = Get-Content $ScenarioFile -Raw -Encoding UTF8 | ConvertFrom-Json
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
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string name);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vk);
    [DllImport("user32.dll")] static extern IntPtr GetTopWindow(IntPtr hWnd);
    [DllImport("user32.dll")] static extern IntPtr GetWindow(IntPtr hWnd, uint cmd);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr hWnd, StringBuilder s, int n);

    /// The class of the window in front.
    public static string ForegroundClass() {
        var name = new StringBuilder(64);
        GetClassName(GetForegroundWindow(), name, name.Capacity);
        return name.ToString();
    }

    /// The process of the window in front.
    public static uint ForegroundProcess() {
        uint pid;
        GetWindowThreadProcessId(GetForegroundWindow(), out pid);
        return pid;
    }

    /// JAWS's dialog boxes (`#32770`) visible on the screen, of the process `jaws`.
    /// Minimizes the visible top-level windows of class `cls`; returns how many.
    public static int MinimizeClass(string cls) {
        int count = 0;
        var name = new StringBuilder(64);
        for (IntPtr h = GetTopWindow(IntPtr.Zero); h != IntPtr.Zero; h = GetWindow(h, 2)) {
            if (!IsWindowVisible(h)) continue;
            name.Clear();
            GetClassName(h, name, name.Capacity);
            if (name.ToString() == cls) { ShowWindow(h, 6); count++; }
        }
        return count;
    }

    public static System.Collections.Generic.List<IntPtr> JawsDialogs(uint jaws) {
        var found = new System.Collections.Generic.List<IntPtr>();
        var cls = new StringBuilder(64);
        for (IntPtr h = GetTopWindow(IntPtr.Zero); h != IntPtr.Zero; h = GetWindow(h, 2)) {
            if (!IsWindowVisible(h)) continue;
            uint pid;
            GetWindowThreadProcessId(h, out pid);
            if (pid != jaws) continue;
            cls.Clear();
            GetClassName(h, cls, cls.Capacity);
            if (cls.ToString() == "#32770") found.Add(h);
        }
        return found;
    }
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder s, int n);
    const uint KEYEVENTF_EXTENDEDKEY = 0x1, KEYEVENTF_KEYUP = 0x2;

    public static bool Key(ushort vk, bool extended, bool up) {
        return Key(vk, extended, up, 0);
    }

    /// A key with its scan code given (0: the active layout's for vk).
    public static bool Key(ushort vk, bool extended, bool up, ushort scan) {
        INPUT[] input = new INPUT[1];
        input[0].type = 1;
        input[0].u.ki.wVk = vk;
        input[0].u.ki.wScan = scan != 0 ? scan : (ushort)MapVirtualKey(vk, 0);
        input[0].u.ki.dwFlags = (extended ? KEYEVENTF_EXTENDEDKEY : 0) | (up ? KEYEVENTF_KEYUP : 0);
        return SendInput(1, input, Marshal.SizeOf(typeof(INPUT))) == 1;
    }

    /// One character typed as itself (KEYEVENTF_UNICODE), whatever the keyboard layout.
    public static bool Char(char c) {
        INPUT[] input = new INPUT[2];
        for (int i = 0; i < 2; i++) {
            input[i].type = 1;
            input[i].u.ki.wVk = 0;
            input[i].u.ki.wScan = c;
            input[i].u.ki.dwFlags = 0x4 | (i == 1 ? KEYEVENTF_KEYUP : 0);
        }
        return SendInput(2, input, Marshal.SizeOf(typeof(INPUT))) == 2;
    }

    /// The virtual key the active layout gives the key with this scan code.
    public static ushort VkForScan(ushort scan) {
        return (ushort)MapVirtualKey(scan, 1);
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
    'downarrow'=@(0x28,$true); 'uparrow'=@(0x26,$true); 'leftarrow'=@(0x25,$true); 'rightarrow'=@(0x27,$true);
    'escape'=@(0x1B,$false); 'esc'=@(0x1B,$false); 'space'=@(0x20,$false); 'backspace'=@(0x08,$false);
    'semicolon'=@(0xBA,$false); 'apostrophe'=@(0xDE,$false);
    'ctrl'=@(0xA2,$false); 'alt'=@(0xA4,$false); 'shift'=@(0xA0,$false); 'win'=@(0x5B,$true);
    'capslock'=@(0x14,$false); 'scrolllock'=@(0x91,$false);
    # The numeric keypad as it sends with Num Lock off, as JAWS's desktop layout uses it: the
    # same virtual keys as the gray keys, without the extended flag (numpad 5 is VK_CLEAR).
    'numpad0'=@(0x2D,$false); 'numpad1'=@(0x23,$false); 'numpad2'=@(0x28,$false);
    'numpad3'=@(0x22,$false); 'numpad4'=@(0x25,$false); 'numpad5'=@(0x0C,$false);
    'numpad6'=@(0x27,$false); 'numpad7'=@(0x24,$false); 'numpad8'=@(0x26,$false);
    'numpad9'=@(0x21,$false); 'numpaddelete'=@(0x2E,$false);
    'numpadplus'=@(0x6B,$false); 'numpadminus'=@(0x6D,$false); 'numpadstar'=@(0x6A,$false);
    'numpadslash'=@(0x6F,$true); 'numpadenter'=@(0x0D,$true)
}

# Keys named by their place on a US keyboard, sent by scan code: JAWS takes keys by scan code
# (Use Virtual Key Codes is off by default), and old-pc's Turkish Q layout gives these virtual
# keys to other keys. The virtual key sent is the one the active layout gives that place, so
# on Turkish Q "comma" types what that key types there, and JAWS hears its Comma.
$ScanTable = @{
    'comma' = 0x33; 'period' = 0x34; 'slash' = 0x35; 'leftbracket' = 0x1A; 'rightbracket' = 0x1B;
    'backslash' = 0x2B; 'grave' = 0x29; 'minus' = 0x0C; 'equals' = 0x0D
}

# name -> @(virtual key, extended, scan code or 0 for the layout's)
function Resolve-Key([string]$name) {
    $n = $name.Trim().ToLowerInvariant()
    if ($KeyTable.ContainsKey($n)) { return @($KeyTable[$n][0], $KeyTable[$n][1], 0) }
    if ($ScanTable.ContainsKey($n)) {
        $scan = [uint16]$ScanTable[$n]
        return @([int][SrKeyInput]::VkForScan($scan), $false, [int]$scan)
    }
    if ($n -match '^f([1-9]|1[0-2])$') { return @((0x6F + [int]$Matches[1]), $false, 0) }
    if ($n -match '^[a-z0-9]$') { return @([int][char]$n.ToUpperInvariant(), $false, 0) }
    throw "Unknown key name '$name'"
}

function Send-Chord([string]$chord) {
    $parts = $chord.Split('+') | Where-Object { $_ -ne '' }
    $keys = @($parts | ForEach-Object { ,(Resolve-Key $_) })
    # Every key is sent and every key released, whatever failed before it: a failed press
    # (refused while a JAWS window came forward) once skipped the releases after it, and left
    # Ctrl down for the scenarios that followed (old-pc, 2026-10-18).
    $ok = $true
    foreach ($k in $keys) {
        if (-not [SrKeyInput]::Key([uint16]$k[0], $k[1], $false, [uint16]$k[2])) { $ok = $false }
        Start-Sleep -Milliseconds 30
    }
    [array]::Reverse($keys)
    foreach ($k in $keys) {
        if (-not [SrKeyInput]::Key([uint16]$k[0], $k[1], $true, [uint16]$k[2])) { $ok = $false }
        Start-Sleep -Milliseconds 30
    }
    # A release Windows refused leaves the key down: an elevated helper releases it. Windows
    # hides the key state while one of JAWS's windows is in front, and JAWS keeps its own idea
    # of a held JAWS key: a chord with a JAWS key that brought a JAWS window forward (Insert+F7)
    # is released elevated too.
    $stuck = @($keys | Where-Object { [SrKeyInput]::GetAsyncKeyState([int]$_[0]) -band 0x8000 })
    $jawsKey = @($keys | Where-Object { $_[0] -eq 0x2D -or $_[0] -eq 0x14 }).Count -gt 0
    # Kept cheap (JAWS's process id is looked up once per run): a process lookup here, between
    # the presses of a double press, made them two single presses, and JAWS's CapsLock pressed
    # twice said nothing (309, old-pc, 2026-10-18).
    $jawsInFront = ($jawsKey -or -not $ok) -and $script:JawsPid -ne 0 -and
        [SrKeyInput]::ForegroundProcess() -eq $script:JawsPid
    if ($stuck.Count -gt 0 -or $jawsInFront) { Invoke-ClearLabState -KeysOnly }
    return $ok
}

# A bare release of the JAWS keys (Insert, CapsLock), harmless when nothing is held: it resets
# JAWS's own idea of a held JAWS key (see clear-lab-state.ps1).
function Release-JawsKeys {
    [void][SrKeyInput]::Key(0x2D, $true, $true)
    [void][SrKeyInput]::Key(0x14, $false, $true)
}

function Get-ForegroundTitle {
    $sb = New-Object System.Text.StringBuilder 512
    [void][SrKeyInput]::GetWindowText([SrKeyInput]::GetForegroundWindow(), $sb, 512)
    return $sb.ToString()
}

function Focus-Target([string]$procName) {
    $proc = if ($procName -eq 'chrome') { Get-LabChromeWindow } else {
        Get-Process $procName -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero } | Select-Object -First 1
    }
    if (-not $proc) { throw "No '$procName' window found." }
    [void][SrKeyInput]::ShowWindow($proc.MainWindowHandle, 9)
    [void][SrKeyInput]::SetForegroundWindow($proc.MainWindowHandle)
    Start-Sleep -Milliseconds 250
}

# Runs jaws-process.ps1 elevated: JAWS runs elevated, and so must what stops it.
function Invoke-JawsProcess([switch]$Stop, [switch]$Start) {
    $helper = Join-Path $PSScriptRoot 'jaws-process.ps1'
    $helperArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$helper`"")
    if ($Stop) { $helperArgs += '-Stop' }
    if ($Start) { $helperArgs += '-Start' }
    $p = Start-Process powershell -Verb RunAs -WindowStyle Hidden -PassThru -ArgumentList $helperArgs
    if ($p) { [void]$p.WaitForExit(60000) }
}

$okurRun = $null
if ($Reader -eq 'okur') {
    $jaws = $null
    $OkurSpeechLog = Join-Path $env:TEMP "okur-live-$([guid]::NewGuid().ToString('N')).jsonl"
    # A settings folder of its own, with the reference machine's settings for all applications,
    # made anew for every run.
    $okurSettings = Join-Path $env:TEMP 'okur-live-settings'
    Remove-Item -Recurse -Force $okurSettings -ErrorAction SilentlyContinue
    & $Okur reference-settings $okurSettings | Out-Null
    Invoke-JawsProcess -Stop
    $okurArgs = @(
        'run', '--settings', "`"$okurSettings`"", '--speech-log', "`"$OkurSpeechLog`"",
        '--engine', 'sapi5', '--voice', "`"$OkurVoice`"")
    if ($OkurDebugLog) { $okurArgs += @('--debug-log', "`"$OkurDebugLog`"") }
    $okurRun = Start-Process -FilePath $Okur -WindowStyle Hidden -PassThru -ArgumentList $okurArgs
    Start-Sleep -Seconds 3
} else {
    $jaws = New-Object -ComObject "FreedomSci.JawsApi"
}
$script:OkurLogOffset = [long]0
# JAWS's process, for Send-Chord's check of a JAWS window in front (0 while JAWS is stopped).
$script:JawsPid = [uint32]0
$jfwProcess = Get-Process jfw -ErrorAction SilentlyContinue | Select-Object -First 1
if ($jfwProcess) { $script:JawsPid = [uint32]$jfwProcess.Id }

# What the reader said since the last clear: JAWS's speech history, or the utterances Okur
# logged; and the recorder's records since $SpokenStart. Returns text, records and spoken_end
# (how the records were cut: see Read-SpeechHistoryOnce; "silence" for Okur, whose records are
# read once its audio is quiet).
function Read-ReaderHistory([long]$SpokenStart) {
    if ($Reader -ne 'okur') { return Read-SpeechHistoryOnce $SpokenStart }
    $records = @(Read-SpokenSince $SpokenStart)
    if (-not (Test-Path $OkurSpeechLog)) { return [pscustomobject]@{ text = ''; records = $records; spoken_end = 'silence' } }
    $fs = [System.IO.File]::Open($OkurSpeechLog, 'Open', 'Read', 'ReadWrite')
    try {
        [void]$fs.Seek($script:OkurLogOffset, 'Begin')
        $text = (New-Object System.IO.StreamReader($fs, [System.Text.Encoding]::UTF8)).ReadToEnd()
        $script:OkurLogOffset = $fs.Length
    } finally { $fs.Dispose() }
    $said = @($text -split "`n" | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json } |
        Where-Object { $_.kind -eq 'speak' } | ForEach-Object { $_.text })
    return [pscustomobject]@{ text = ($said -join "`n"); records = $records; spoken_end = 'silence' }
}

function Clear-ReaderHistory {
    if ($Reader -ne 'okur') { [void]$jaws.RunScript("ClearSpeechHistory"); return }
    $script:OkurLogOffset = if (Test-Path $OkurSpeechLog) { (Get-Item $OkurSpeechLog).Length } else { [long]0 }
}

function Stop-ReaderSpeech { if ($jaws) { $jaws.StopSpeech() } }

$CopyConfirmation = 'Copy speech history to clipboard'

# Blocks until JAWS's audio output is silent (okur wait-silence on the default endpoint;
# JAWS is the only audio source on the reference machine). Returns the parsed SilenceReport.
function Wait-Silence([int]$OnsetMs = 800, [int]$QuietMs = 500, [int]$MaxMs = 4000) {
    $out = & $Okur wait-silence --onset-ms $OnsetMs --quiet-ms $QuietMs --max-ms $MaxMs
    if ($LASTEXITCODE -ne 0) { throw "okur wait-silence failed: $out" }
    return ($out | Select-Object -Last 1 | ConvertFrom-Json)
}

# How long a step waits for JAWS to reach its copy confirmation, speaking what it still had
# queued when its history was copied: counted from JAWS's last utterance, so a long read (Say
# Window in a dialog speaks for more than 15 s, 323/s2) is waited for while it goes on; and at
# most $ConfirmationMaxMs in all.
$ConfirmationWaitMs = 15000
$ConfirmationMaxMs = 180000

# Copies the speech history exactly once, and cuts the recorder's records where JAWS's copy
# confirmation begins. JAWS queues that confirmation behind whatever it has not spoken yet
# (old-pc, 2026-10-19: three utterances queued with SayString, the copy sent 400 ms in; the
# confirmation was the fourth record, at 2.9 s). So the records before it are what JAWS had
# queued when its history was copied, however long JAWS paused between its utterances: a step
# that settled in such a pause is no longer cut short (parity gap audit §3.3). Then speech is
# stopped and the history cleared, so the confirmation cannot reach the next step.
# spoken_end: "confirmation", "timeout" (JAWS did not reach it in time; the step fails) or
# "no-recorder" (no log to cut).
function Read-SpeechHistoryOnce([long]$SpokenStart) {
    $sentinel = "__SR_SENTINEL_$([guid]::NewGuid().ToString('N').Substring(0,8))__"
    Set-Clipboard -Value $sentinel
    [void]$jaws.RunScript("CopySpeechHistoryToClipboard")
    $text = ""
    for ($i = 0; $i -lt 40; $i++) {
        Start-Sleep -Milliseconds 25
        $clip = Get-Clipboard -Raw -ErrorAction SilentlyContinue
        if ($null -ne $clip -and $clip -ne $sentinel) { $text = $clip; break }
    }
    $records = @()
    $spokenEnd = 'no-recorder'
    if (Test-Path $SpokenLog) {
        $spokenEnd = 'timeout'
        $deadline = [DateTime]::UtcNow.AddMilliseconds($ConfirmationWaitMs)
        $limit = [DateTime]::UtcNow.AddMilliseconds($ConfirmationMaxMs)
        $seen = 0
        do {
            # @(): Windows PowerShell returns one record as itself, and a lone object has no Count.
            $records = @(Read-SpokenSince $SpokenStart)
            if ($records.Count -gt $seen) {
                # JAWS is still speaking what it had queued.
                $seen = $records.Count
                $deadline = [DateTime]::UtcNow.AddMilliseconds($ConfirmationWaitMs)
                if ($deadline -gt $limit) { $deadline = $limit }
            }
            $at = -1
            for ($i = 0; $i -lt $records.Count; $i++) {
                if ((Get-SpokenText $records[$i]).Trim() -eq $CopyConfirmation) { $at = $i; break }
            }
            if ($at -ge 0) {
                # Assigned in each branch: an if's output would turn an empty array into $null.
                if ($at -gt 0) { $records = @($records[0..($at - 1)]) } else { $records = @() }
                $spokenEnd = 'confirmation'
                break
            }
            Start-Sleep -Milliseconds 25
        } while ([DateTime]::UtcNow -lt $deadline)
    } else {
        Start-Sleep -Milliseconds 150
    }
    $jaws.StopSpeech()
    [void]$jaws.RunScript("ClearSpeechHistory")
    Start-Sleep -Milliseconds 80
    return [pscustomobject]@{ text = $text; records = $records; spoken_end = $spokenEnd }
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
        # A key held down across the following steps (Alt held through several Tabs), and
        # its release. Held keys are released at the end of a trial whatever happens.
        'key_down' {
            $k = Resolve-Key $command
            $script:HeldKeys += ,$k
            return [SrKeyInput]::Key([uint16]$k[0], $k[1], $false, [uint16]$k[2])
        }
        'key_up' {
            $k = Resolve-Key $command
            $script:HeldKeys = @($script:HeldKeys | Where-Object { $_[0] -ne $k[0] })
            return [SrKeyInput]::Key([uint16]$k[0], $k[1], $true, [uint16]$k[2])
        }
        # Started through ShellExecute (Start-Process), as Windows requires to grant UI Access;
        # JAWS's dialogs are jfw.exe's, Okur's okur-ui.exe's.
        'key_ui_access' {
            if (-not (Test-Path -LiteralPath $LabKeys)) { return $false }
            $sent = Start-Process -FilePath $LabKeys -ArgumentList @('--allow', 'jfw,okur-ui', "`"$command`"") -WindowStyle Hidden -Wait -PassThru
            return $sent.ExitCode -eq 0
        }
        'jaws_script' { if (-not $jaws) { return $false }; return [bool]$jaws.RunScript($command) }
        # A JAWS function call through FSAPI RunFunction, e.g. TypeKey("Down"): keys JAWS types
        # itself reach its own elevated dialogs, which refuse ours. With Okur reading, the key
        # is the user's key in Okur's dialog.
        'jaws_function' {
            if ($jaws) { return [bool]$jaws.RunFunction($command) }
            if ($command -match '^TypeKey\("(.+)"\)$') { return Send-Chord $Matches[1] }
            return $false
        }
        # Text typed character by character as Unicode input, so old-pc's Turkish layout
        # types it as written ("%TEMP%\sr-dialogs" into a file dialog). Environment variables
        # are left for the application to expand.
        'text' {
            $ok = $true
            foreach ($c in $command.ToCharArray()) {
                $ok = [SrKeyInput]::Char($c) -and $ok
                Start-Sleep -Milliseconds 40
            }
            return $ok
        }
        'keyboard'    { [System.Windows.Forms.SendKeys]::SendWait($command); return $true }
        default       { throw "Unsupported step type '$type'" }
    }
}

$script:HeldKeys = @()
function Release-HeldKeys {
    foreach ($k in $script:HeldKeys) { [void][SrKeyInput]::Key([uint16]$k[0], $k[1], $true, [uint16]$k[2]) }
    $script:HeldKeys = @()
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

# A scenario's "jaws_settings" (jaws-settings.ps1): JAWS must be running with them, and with
# nothing else changed, or the evidence would hold another setting's speech. run-batch.ps1
# applies them and restarts JAWS. Every file they name is recorded with the others.
if ($Reader -eq 'jaws') {
    $settingsTool = Join-Path $PSScriptRoot 'jaws-settings.ps1'
    $asks = $scenario.PSObject.Properties.Name -contains 'jaws_settings' -and $null -ne $scenario.jaws_settings
    $applied = Test-Path (Join-Path $env:USERPROFILE 'sr-harness\settings-backup\manifest.json')
    if ($asks) {
        $out = & powershell -NoProfile -ExecutionPolicy Bypass -File $settingsTool -Check $ScenarioFile
        if ($LASTEXITCODE -ne 0) { throw "JAWS's settings are not the scenario's (run it through run-batch.ps1): $out" }
        foreach ($name in $scenario.jaws_settings.PSObject.Properties.Name) {
            if ($jawsSettings.Contains($name)) { continue }
            $jawsSettings[$name] = if ($name -match '^voiceprofiles/(.+)$') { Read-JawsSettings "..\voiceprofiles\$($Matches[1])" } else { Read-JawsSettings $name }
        }
    } elseif ($applied) {
        throw "JAWS runs with settings another scenario applied: put them back first (jaws-settings.ps1 -Restore, then restart JAWS)."
    }
}

$expectedTitle = $null
if ($fixturePath -and (Test-Path $fixturePath)) {
    # Fixtures are UTF-8 without a byte order mark, which Windows PowerShell 5.1 would read in
    # the system's code page: "Türkçe Metin" came out garbled and every 613 trial failed its
    # window title check (rounds 86, 92, perf105b, 110).
    $html = Get-Content -Raw -Encoding UTF8 $fixturePath
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
        # The numpad keys are sent as with Num Lock off; JAWS may also look at the lock itself.
        num_lock = [System.Windows.Forms.Control]::IsKeyLocked([System.Windows.Forms.Keys]::NumLock)
        # Time and date announcements follow the user locale and time zone.
        culture = (Get-Culture).Name
        utc_offset_minutes = [int][TimeZoneInfo]::Local.GetUtcOffset((Get-Date)).TotalMinutes
        chrome_version = if ($chromeExe) { (Get-Item $chromeExe).VersionInfo.FileVersion } else { $null }
        # Web fixtures open in the lab's own signed-out Chrome profile (lab-chrome.ps1).
        chrome_profile = if ($targetProc -eq 'chrome') { 'sr-harness' } else { $null }
        jaws_version = if (Test-Path $jfwExe) { (Get-Item $jfwExe).VersionInfo.ProductVersion } else { $null }
        jaws_settings = $jawsSettings
        reader = $Reader
        okur_sha256 = if ($Reader -eq 'okur') { Get-Sha256 $Okur } else { $null }
        # The tracer runs elevated, so it sees JAWS's own windows whole.
        trace_elevated = $true
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
    $p = Get-Process -ErrorAction SilentlyContinue |
        Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero -and $_.MainWindowTitle -eq $title } |
        Select-Object -First 1
    if ($p) { return $p }
    # An owned window is no process's main window (a property sheet the shell opens for the
    # fixture): look among the desktop's top-level windows, for its process and handle.
    Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
    $named = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::NameProperty, $title)
    $window = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
        [System.Windows.Automation.TreeScope]::Children, $named)
    if (-not $window) { return $null }
    [pscustomobject]@{
        Id               = $window.Current.ProcessId
        MainWindowHandle = [IntPtr]$window.Current.NativeWindowHandle
    }
}

# Modifier keys left down and JAWS's own window in front spoil every step after them, and only
# an elevated process can clear them while a JAWS window is in front: clear-lab-state.ps1
# (which explains both) runs elevated when either is found. The lab's UAC elevates without
# asking.
$ModifierKeys = @(0xA2, 0xA3, 0xA4, 0xA5, 0xA0, 0xA1, 0x5B, 0x5C, 0x2D)

function Get-HeldModifiers {
    @($ModifierKeys | Where-Object { [SrKeyInput]::GetAsyncKeyState($_) -band 0x8000 })
}

function Invoke-ClearLabState([switch]$KeysOnly) {
    $helper = Join-Path $PSScriptRoot 'clear-lab-state.ps1'
    $helperArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$helper`"")
    if ($KeysOnly) { $helperArgs += '-KeysOnly' }
    $p = Start-Process powershell -Verb RunAs -WindowStyle Hidden -PassThru -ArgumentList $helperArgs
    if ($p) { [void]$p.WaitForExit(15000) }
}

# Before a trial: what had to be cleared, for the trial's record (null when nothing).
function Clear-LabState {
    Release-JawsKeys
    # Windows' own menus left open by an earlier step (the Start menu after a Windows key
    # chord, the Win+X menu; 2026-10-19 round 25) take every key of the trials after them:
    # Escape closes them.
    $shellClosed = @()
    for ($i = 0; $i -lt 3; $i++) {
        $cls = [SrKeyInput]::ForegroundClass()
        if ($cls -notin @('Windows.UI.Core.CoreWindow', '#32768')) { break }
        $shellClosed += $cls
        [void](Send-Chord "Escape"); Start-Sleep -Milliseconds 300
    }
    $held = Get-HeldModifiers
    $window = [SrKeyInput]::FindWindow('JFWUI2', [NullString]::Value)
    $jawsInWay = $window -ne [IntPtr]::Zero -and (
        [SrKeyInput]::GetForegroundWindow() -eq $window -or [SrKeyInput]::IsWindowVisible($window))
    $jawsProcess = Get-Process jfw -ErrorAction SilentlyContinue | Select-Object -First 1
    $dialogs = if ($jawsProcess) { [SrKeyInput]::JawsDialogs([uint32]$jawsProcess.Id).Count } else { 0 }
    if ($held.Count -eq 0 -and -not $jawsInWay -and $dialogs -eq 0) {
        if ($shellClosed.Count -eq 0) { return $null }
        return [ordered]@{ shell_windows_closed = $shellClosed }
    }
    Invoke-ClearLabState
    [ordered]@{
        shell_windows_closed = $shellClosed
        modifiers_held = @($held | ForEach-Object { '0x{0:X2}' -f $_ })
        jaws_window_in_front = $jawsInWay
        jaws_dialogs_closed = $dialogs
        modifiers_still_held = @(Get-HeldModifiers | ForEach-Object { '0x{0:X2}' -f $_ })
    }
}

function Start-DesktopFixture {
    $old = Find-WindowByTitle $launch.window_title
    if ($old) { Stop-Process -Id $old.Id -Force; Start-Sleep -Milliseconds 500 }
    # Browser windows left by web scenarios are minimized: one that came forward in the middle
    # of a desktop trial took its keys, and Insert+Shift+Down twice spelled a whole web page
    # for minutes, so every step after it timed out (504 and 505 on old-pc, 2026-10-19).
    # A browser of another profile is closed: its titles carry the profile's name.
    [void](Stop-OtherChrome)
    [void][SrKeyInput]::MinimizeClass('Chrome_WidgetWin_1')
    Start-Process powershell -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $fixturePath) | Out-Null
    for ($i = 0; $i -lt 100; $i++) {
        $p = Find-WindowByTitle $launch.window_title
        if ($p) {
            # Windows can refuse to bring a window forward: then a browser window or JAWS's viewer
            # left by an earlier scenario keeps the keys, and the trial reads it (504 and 505 on
            # old-pc, 2026-10-19). The fixture is asked until it is in front, minimized and
            # restored if asking is not enough.
            $handle = $p.MainWindowHandle
            for ($k = 0; $k -lt 30; $k++) {
                if ($k -eq 10 -or $k -eq 20) { [void][SrKeyInput]::ShowWindow($handle, 6) }
                [void][SrKeyInput]::ShowWindow($handle, 9)
                [void][SrKeyInput]::SetForegroundWindow($handle)
                Start-Sleep -Milliseconds 100
                if ([SrKeyInput]::GetForegroundWindow() -eq $handle) { return $true }
            }
            return $false
        }
        Start-Sleep -Milliseconds 100
    }
    return $false
}

if ($scenario.fixture -and -not $launch) {
    if ($targetProc -eq 'chrome') {
        $evidence.provenance.other_browsers_closed = Start-LabChrome $scenario.fixture
    } else {
        [System.Diagnostics.Process]::Start($targetProc, $scenario.fixture) | Out-Null
    }
    # A cold browser start takes longer than a new tab: wait for the fixture's window.
    for ($i = 0; $i -lt 100; $i++) {
        $w = if ($targetProc -eq 'chrome') { Get-LabChromeWindow } else { Get-Process $targetProc -ErrorAction SilentlyContinue }
        $w = $w | Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero -and (-not $expectedTitle -or $_.MainWindowTitle.StartsWith($expectedTitle)) }
        if ($w) { break }
        Start-Sleep -Milliseconds 200
    }
    Start-Sleep -Milliseconds 2000
}

# The tracer runs elevated: stopping it may need elevation too.
function Stop-Tracer($process) {
    try { $process.Kill() } catch {
        Start-Process powershell -Verb RunAs -WindowStyle Hidden -Wait -ArgumentList @('-NoProfile', '-Command', "Stop-Process -Id $($process.Id) -Force")
    }
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

    # A desktop trial puts the browser's windows away before its trace starts: one left in front
    # by a web scenario reached the trace with its title, which carries the browser profile's
    # name (526, round 24). A second lets the minimizing's events go by first (300 ms was not
    # enough: 537, round 32).
    if ($launch) {
        [void][SrKeyInput]::MinimizeClass('Chrome_WidgetWin_1')
        Start-Sleep -Milliseconds 1000
    }
    # The trace starts before setup, so replay knows the window and focus that launch and
    # reset produced, as JAWS does when step 1 begins.
    $traceProc = $null
    $traceStop = $null
    if ($TraceDir) {
        New-Item -ItemType Directory -Force -Path $TraceDir | Out-Null
        $tracePath = Join-Path $TraceDir "$scenarioId-trial$t.jsonl"
        $traceStop = Join-Path $env:TEMP "sr-trace-stop-$([guid]::NewGuid().ToString('N'))"
        # Elevated, as JAWS is: from normal integrity UI Automation sees JAWS's own dialogs
        # thinly (a list view as a pane, buttons as panes named "&Move To Heading"; 224,
        # round 23). old-pc elevates without asking.
        $traceArgs = @('trace', '--out', $tracePath, '--stop-file', $traceStop)
        if ($TraceTimings) { $traceArgs += @('--debug-log', (Join-Path $TraceDir "$scenarioId-trial$t.log")) }
        $traceProc = Start-Process -FilePath $Okur -ArgumentList $traceArgs -WindowStyle Hidden -PassThru -Verb RunAs
        Start-Sleep -Milliseconds 400
        $trialResult.trace_file = Split-Path -Leaf $tracePath
    }
    # Identical setup for every trial: reload (or relaunch), let load speech finish, reset,
    # settle, clear.
    $cleared = Clear-LabState
    if ($cleared) { $trialResult.lab_state_cleared = $cleared }
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
    Stop-ReaderSpeech
    Clear-ReaderHistory
    Start-Sleep -Milliseconds 80
    $trialResult.page_load_settled = $loadQuiet.settled
    $trialResult.setup_settled = $resetQuiet.settled
    if (-not $resetQuiet.settled) { $trialResult.trial_success = $false }
    $trialResult.foreground_window_start = Get-ForegroundTitle
    # A reset can open another window on purpose ("reset": {"window_title": ...}): 317's Tab,
    # Enter opens the fixture's second window, so that Alt+Tab has the first to go back to.
    $startTitle = if ($scenario.reset -and $scenario.reset.window_title) { $scenario.reset.window_title } else { $expectedTitle }
    if ($startTitle -and -not $trialResult.foreground_window_start.StartsWith($startTitle)) {
        $trialResult.trial_success = $false
    }


    foreach ($step in $scenario.steps) {
        $maxMs = if ($step.settle_max_ms) { [int]$step.settle_max_ms } else { 10000 }
        $quietMs = if ($step.quiet_ms) { [int]$step.quiet_ms } else { 500 }

        # Start every step from a silent baseline, so residual audio from the previous step
        # (e.g. the tail of the copy confirmation) cannot be mistaken for this step's speech.
        # Up to 10 s: a step can settle in a pause of JAWS's speech (between "Title is" and the
        # title, between the symbols of a code line read with punctuation Most) and JAWS then
        # speaks on for seconds; 3 s failed steps of 209, 314 and 315 on 2026-10-18.
        $baseline = Wait-Silence -OnsetMs 150 -QuietMs 300 -MaxMs 10000
        # "history": false: JAWS's speech history is neither cleared nor read for the step,
        # only the speech recorder's text is kept. Clearing and copying it are JAWS scripts, which
        # Keyboard Help describes instead of running ("Insert+1 | Copy Speech History to
        # Clipboard.", 509 in round 91).
        $readsHistory = -not ($step.PSObject.Properties.Name -contains 'history' -and $step.history -eq $false)
        if ($readsHistory) { Clear-ReaderHistory }
        $spokenStart = Get-SpokenOffset

        $tStart = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
        $dispatchOk = Invoke-Step $step.type $step.command ([int]$step.repeat)
        # "during": chords pressed while the step's speech goes on (Ctrl or Right during Say
        # All), each after_ms after the one before; the step then settles and reads the history
        # once, so its records end where the last of them left JAWS.
        foreach ($d in @($step.during)) {
            if ($null -eq $d) { continue }
            Start-Sleep -Milliseconds ([int]$d.after_ms)
            $dispatchOk = (Send-Chord $d.command) -and $dispatchOk
        }
        $observed = Wait-Silence -OnsetMs 800 -QuietMs $quietMs -MaxMs $maxMs
        # "capture_clipboard": what the step copied (e.g. Ctrl+C in the Virtual Viewer), read
        # before the speech history copy replaces it.
        $clipboard = if ($step.capture_clipboard) { Get-Clipboard -Raw -ErrorAction SilentlyContinue } else { $null }
        $history = if ($readsHistory) { Read-ReaderHistory $spokenStart } else {
            [pscustomobject]@{ text = ''; records = @(Read-SpokenSince $spokenStart); spoken_end = 'silence' }
        }
        $transcript = $history.text
        $spokenRecords = @($history.records)
        $tEnd = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
        if ($transcript -match "unknown script call" -or $transcript -match "unhandled script call") { $dispatchOk = $false }
        # A leaked copy confirmation, or a Windows notification JAWS announced mid-step (seen
        # in 105 v2 trial 1), means the history was contaminated: fail, never trim. A scenario
        # about notifications says so ("expects_notifications": true, 533).
        $strayNotification = $transcript.Contains('New notification from') -and -not $scenario.expects_notifications
        # A step that copies the history itself (Ctrl+H in the Speech History) says the
        # confirmation once, as its own speech; a second is a leak.
        $confirmations = [regex]::Matches($transcript, [regex]::Escape($CopyConfirmation)).Count
        $ownCopies = if ($step.copies_speech_history) { 1 } else { 0 }
        $contaminated = $confirmations -gt $ownCopies -or $strayNotification
        # Audio with an empty history entry is recorded, not failed: JAWS speaks a space
        # character aloud but logs an empty history entry (verified 2026-10-14, 107). The
        # silent baseline above is what prevents the copy from racing ahead of speech.
        $audioWithoutText = $observed.heard_audio -and [string]::IsNullOrEmpty($transcript)

        $stepSuccess = $dispatchOk -and $baseline.settled -and $observed.settled -and -not $contaminated -and
            $history.spoken_end -ne 'timeout'
        if (-not $stepSuccess) { $trialResult.trial_success = $false }

        $trialResult.step_results += [ordered]@{
            step_id = $step.id
            t_start_ms = $tStart
            t_end_ms = $tEnd
            action = if ($step.during) {
                [ordered]@{ type = $step.type; command = $step.command; repeat = [Math]::Max(1, [int]$step.repeat); during = @($step.during) }
            } else {
                [ordered]@{ type = $step.type; command = $step.command; repeat = [Math]::Max(1, [int]$step.repeat) }
            }
            observation = [ordered]@{
                speech_raw = $transcript
                spoken = @($spokenRecords | ForEach-Object { Get-SpokenText $_ })
                spoken_records = $spokenRecords
                spoken_end = $history.spoken_end
                clipboard = $clipboard
                dispatch_ok = $dispatchOk
                settled = $observed.settled
                heard_audio = $observed.heard_audio
                settle_ms = $observed.elapsed_ms
                contaminated = $contaminated
                audio_without_text = $audioWithoutText
                baseline_settled = $baseline.settled
                history_read = $readsHistory
            }
            step_success = $stepSuccess
        }
    }
    if ($traceProc) {
        New-Item -ItemType File -Force -Path $traceStop | Out-Null
        if (-not $traceProc.WaitForExit(15000)) { Stop-Tracer $traceProc; $trialResult.trial_success = $false }
        Remove-Item $traceStop -ErrorAction SilentlyContinue
        if (Test-Path $tracePath) { $trialResult.trace_sha256 = Get-Sha256 $tracePath } else { $trialResult.trial_success = $false }
    }
    Release-HeldKeys
    $evidence.trials += $trialResult
}
} finally {
    # An exception mid-trial (e.g. no browser window) must not leave the tracer running: it
    # holds the trace file and the harness binary.
    Release-HeldKeys
    if ($traceProc -and -not $traceProc.HasExited) { Stop-Tracer $traceProc }
    # A live run of Okur ends with Okur stopped and JAWS back.
    if ($okurRun -and -not $okurRun.HasExited) { $okurRun.Kill() }
    if ($Reader -eq 'okur') { Invoke-JawsProcess -Start }
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

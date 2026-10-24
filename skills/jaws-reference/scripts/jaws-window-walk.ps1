# Research, never evidence: JAWS's window (Insert+J), its menus, and the Basics and Voice
# Adjustment dialogs, walked with keys (jaws-ui-walk.ps1) and read from JAWS's speech history,
# for Okur's settings redesign (docs/references/okur-settings-redesign.md §5). Only navigation
# keys and Escape: no setting changes. Each part is written to its own file in -OutDir. -Parts
# chooses the parts, by name, comma separated: options, menus, basics, voice (Tab around Voice
# Adjustment), contexts (Voice Adjustment's Adjust box walked, each context's controls heard),
# profile (Select a Voice Profile, Insert+Ctrl+S from Notepad), profiles (Voice Adjustment's
# Profile Name box walked), saveas (its Save As... dialog), manage (Manage Application
# Settings...), wizard (the Startup Wizard's first page), wizardpages (with wizard: its pages 2
# to 5, by Next), wizardvalues (with wizard: its radio buttons' names, each put back, and the
# last page's Finish). Run on old-pc with no batch recording.
param([string]$OutDir = (Join-Path $env:USERPROFILE 'sr-harness\probe'),
      [string]$Parts = 'options,menus,basics,voice')
$chosen = $Parts -split ',' | ForEach-Object { $_.Trim() }
$lab = Join-Path $env:USERPROFILE 'sr-harness\bin\okur-lab.exe'
$walk = Join-Path $PSScriptRoot 'jaws-ui-walk.ps1'
New-Item -ItemType Directory -Force $OutDir | Out-Null
function Walk([string]$chords, [string]$out) {
    & powershell -NoProfile -ExecutionPolicy Bypass -File $walk -Process jfw -Chords $chords -WaitMs 1000 -Out (Join-Path $OutDir $out) | Out-Null
}
Add-Type -Namespace JWW -Name W -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow(); [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p); [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);'
function Front-Process {
    $id = 0
    [void][JWW.W]::GetWindowThreadProcessId([JWW.W]::GetForegroundWindow(), [ref]$id)
    (Get-Process -Id $id -ErrorAction SilentlyContinue).ProcessName
}
function Front-Title {
    $text = New-Object System.Text.StringBuilder 512
    [void][JWW.W]::GetWindowText([JWW.W]::GetForegroundWindow(), $text, 512)
    $text.ToString()
}
# JAWS's window in front. With it already open behind Notepad, Insert+J leaves Notepad in front
# (2026-10-24), and Alt+Tab, through the UI Access keys, brings it.
function Open-JawsWindow {
    & $lab front 'Notepad' | Out-Null
    Start-Sleep -Milliseconds 500
    & $lab keys --allow notepad 'Insert+J' 'wait:2500' | Out-Null
    if ((Front-Process) -ne 'jfw') {
        Start-Process -FilePath 'C:\Program Files\Okur Lab\okur-lab-keys.exe' -ArgumentList @('--allow', 'notepad', '"Alt+Tab"', '"wait:1500"') -WindowStyle Hidden -Wait
    }
    "--- Insert+J: " + ((& $lab jaws-history --clear) -join ' | ')
}
if (-not (Get-Process notepad -ErrorAction SilentlyContinue)) { Start-Process notepad; Start-Sleep -Seconds 2 }
& $lab jaws-history --clear | Out-Null
if ($chosen -contains 'options') {
    Open-JawsWindow
    # Options, with the Voices submenu opened on the way.
    Walk 'Alt,Down,Down,Right,Down*3,Left,Down*9,Escape,Escape' 'jw-options.txt'
}
if ($chosen -contains 'menus') {
    Open-JawsWindow
    # Utilities, Language and Help, each opened and walked.
    Walk 'Alt,Right,Down,Down*18,Escape,Right,Down,Down*4,Escape,Right,Down,Down*14,Escape,Escape' 'jw-menus.txt'
}
if ($chosen -contains 'basics') {
    Open-JawsWindow
    # Options, Basics...: Tab around the dialog, then Escape (Cancel).
    Walk 'Alt,Down,Enter,Tab*14,Escape' 'jw-basics.txt'
}
if ($chosen -contains 'voice') {
    Open-JawsWindow
    # Options, Voices, Voice Adjustment...: Tab around, then Escape.
    Walk 'Alt,Down,Down,Right,Enter,Tab*22,Escape' 'jw-voice-adjustment.txt'
}
if ($chosen -contains 'contexts') {
    Open-JawsWindow
    # Voice Adjustment's Adjust box (fifth Tab): each context chosen with Down, its Person, Rate,
    # Punctuation and Volume heard by Tab and back, then Escape (Cancel).
    $context = 'Tab*4,Shift+Tab*4'
    Walk "Alt,Down,Down,Right,Enter,Tab*5,$context,Down,$context,Down,$context,Down,$context,Down,$context,Down,Escape" 'jw-voice-contexts.txt'
}
if ($chosen -contains 'profiles') {
    Open-JawsWindow
    # Voice Adjustment's Profile Name box, every profile by Up and back by Down, then Escape
    # (Cancel), and Escape again in case it asks.
    Walk 'Alt,Down,Down,Right,Enter,Up,Up,Up,Up,Up,Down,Down,Down,Down,Down,Escape,Escape' 'jw-voice-profiles.txt'
}
if ($chosen -contains 'saveas') {
    Open-JawsWindow
    # Voice Adjustment's Save As... (twelfth Tab): its dialog toured by Tab, then Escape twice.
    Walk 'Alt,Down,Down,Right,Enter,Tab*12,Space,Tab,Tab,Tab,Tab,Escape,Escape' 'jw-voice-save-as.txt'
}
if ($chosen -contains 'manage') {
    Open-JawsWindow
    # Options, Manage Application Settings... (fifth): toured by Tab, then Escape.
    Walk 'Alt,Down,Down*4,Enter,Tab*8,Escape,Escape' 'jw-manage-applications.txt'
}
# Restore to Factory Settings... is not walked: it is a Yes/No question with Yes focused, which
# ignores Escape, so a walk left it open with Yes focused (2026-10-24; answered No by
# okur-lab-keys after UI Automation showed the focus on No). Its words are in the redesign's
# §1.1.
if ($chosen -contains 'wizard') {
    Open-JawsWindow
    # Help, Startup Wizard... by their access keys (Alt+H, Z: JAWS's help), as counting Downs
    # once walked JAWS's window with its menu bar closed; then its first page toured in the
    # wizard's own process, and Escape (Cancel). Never Finish. Keys go to the wizard only when
    # the window in front is titled as it.
    Walk 'Alt+H,Z' 'jw-startup-wizard-open.txt'
    Start-Sleep -Seconds 3
    $process = Front-Process
    $title = Front-Title
    "wizard process: $process, title: $title" | Add-Content (Join-Path $OutDir 'jw-startup-wizard-open.txt')
    if ($process -and $title -like '*Startup Wizard*') {
        # With -Parts wizardpages, pages 2 to 5 too: Next (Alt+N) and each page toured by Tab;
        # nothing is changed, and Escape saves nothing (JAWS's help: only Finish saves).
        # With -Parts wizardvalues, the radio buttons of Common Options and Verbosity Settings
        # walked each way and put back as found (a change takes effect at once in the wizard),
        # then Next to the last page for its Finish; Escape saves nothing.
        $chords = if ($chosen -contains 'wizardvalues') {
            'Alt+N,Alt+N,Alt+N,Down*3,Up*3,Tab,Up*2,Down*2,Alt+N,Down,Up,Tab,Down*3,Up*3,Tab,Down*2,Up*2,Alt+N,Alt+N,Tab*12,Escape,Escape'
        } elseif ($chosen -contains 'wizardpages') {
            'Alt+N,Tab*9,Alt+N,Tab*7,Alt+N,Tab*11,Alt+N,Tab*7,Escape,Escape'
        } else { 'Tab*10,Escape,Escape' }
        & powershell -NoProfile -ExecutionPolicy Bypass -File $walk -Process $process -Chords $chords -WaitMs 1000 -Out (Join-Path $OutDir 'jw-startup-wizard.txt') | Out-Null
    }
}
if ($chosen -contains 'profile') {
    # Select a Voice Profile over Notepad: Tab around, then Escape.
    & $lab front 'Notepad' | Out-Null
    Start-Sleep -Milliseconds 500
    & $lab jaws-history --clear | Out-Null
    & $lab keys --allow notepad 'Insert+Ctrl+S' 'wait:2500' | Out-Null
    "--- Insert+Ctrl+S: " + ((& $lab jaws-history --clear) -join ' | ') | Set-Content (Join-Path $OutDir 'jw-voice-profile-open.txt')
    Walk 'Tab,Tab,Tab,Tab,Tab,Escape' 'jw-voice-profile.txt'
}
# Leave JAWS's window behind Notepad, as it was found.
& $lab front 'Notepad' | Out-Null
"done"

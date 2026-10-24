# Research, never evidence: JAWS's window (Insert+J), its menus, and the Basics and Voice
# Adjustment dialogs, walked with keys (jaws-ui-walk.ps1) and read from JAWS's speech history,
# for Okur's settings redesign (docs/references/okur-settings-redesign.md §5). Only navigation
# keys and Escape: no setting changes. Each part is written to its own file in -OutDir. -Parts
# chooses the parts, by name, comma separated: options, menus, basics, voice (Tab around Voice
# Adjustment), contexts (Voice Adjustment's Adjust box walked, each context's controls heard),
# profile (Select a Voice Profile, Insert+Ctrl+S from Notepad). Run on old-pc with no batch
# recording.
param([string]$OutDir = (Join-Path $env:USERPROFILE 'sr-harness\probe'),
      [string]$Parts = 'options,menus,basics,voice')
$chosen = $Parts -split ',' | ForEach-Object { $_.Trim() }
$lab = Join-Path $env:USERPROFILE 'sr-harness\bin\okur-lab.exe'
$walk = Join-Path $PSScriptRoot 'jaws-ui-walk.ps1'
New-Item -ItemType Directory -Force $OutDir | Out-Null
function Walk([string]$chords, [string]$out) {
    & powershell -NoProfile -ExecutionPolicy Bypass -File $walk -Process jfw -Chords $chords -WaitMs 1000 -Out (Join-Path $OutDir $out) | Out-Null
}
function Open-JawsWindow {
    & $lab front 'Notepad' | Out-Null
    Start-Sleep -Milliseconds 500
    & $lab keys --allow notepad 'Insert+J' 'wait:2500' | Out-Null
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

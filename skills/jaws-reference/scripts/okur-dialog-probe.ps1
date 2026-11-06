# Research, never evidence: one of Okur's own dialogs as Windows holds it, for finding why Okur
# reads it as it does. JAWS is stopped, Okur started with old-pc's settings and no speech, the
# dialog opened over Notepad by its keys (Settings Center by default), then every child window of
# the dialog is listed (class, id, visible, text, rectangle) and UI Automation's control view of
# the window in front saved (okur-lab window-tree); Escape closes the dialog, Okur stops, JAWS
# starts again. Run on old-pc with no batch recording and the Linux lab shut down.
param([string[]]$Keys = @('Insert+6'),
      [string]$OutDir = (Join-Path $env:USERPROFILE 'sr-harness\probe\okur-dialog'))
$bin = Join-Path $env:USERPROFILE 'sr-harness\bin'
$lab = Join-Path $bin 'okur-lab.exe'
$okur = Join-Path $bin 'okur.exe'
$keysExe = 'C:\Program Files\Okur Lab\okur-lab-keys.exe'
New-Item -ItemType Directory -Force $OutDir | Out-Null
$log = Join-Path $OutDir 'probe.txt'
Remove-Item $log -ErrorAction SilentlyContinue
function Note([string]$Line) { $Line | Add-Content -Path $log -Encoding utf8 }
function Jaws([string]$How) {
    $helper = Join-Path $PSScriptRoot 'jaws-process.ps1'
    $p = Start-Process powershell -Verb RunAs -WindowStyle Hidden -PassThru -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$helper`"", "-$How")
    if ($p) { [void]$p.WaitForExit(60000) }
}
Add-Type -Namespace ODP -Name W -MemberDefinition @'
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
[DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int i);
[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
public struct RECT { public int Left, Top, Right, Bottom; }
public delegate bool EnumProc(IntPtr h, IntPtr p);
[DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, EnumProc f, IntPtr p);
'@

$settings = Join-Path $env:TEMP 'okur-probe-settings'
Remove-Item -Recurse -Force $settings -ErrorAction SilentlyContinue
& $lab reference-settings $settings | Out-Null
if (-not (Get-Process notepad -ErrorAction SilentlyContinue)) { Start-Process notepad; Start-Sleep -Seconds 2 }
Jaws 'Stop'
$run = Start-Process -FilePath $okur -WindowStyle Hidden -PassThru -ArgumentList @('run', '--settings', "`"$settings`"", '--engine', 'sapi5', '--voice', '"SR Speech Recorder"')
Start-Sleep -Seconds 4
try {
    & $lab front 'Notepad' | Out-Null
    Start-Sleep -Milliseconds 500
    foreach ($k in $Keys) { & $lab keys --allow notepad $k 'wait:3000' | Out-Null }
    $front = [ODP.W]::GetForegroundWindow()
    $title = New-Object System.Text.StringBuilder 512
    [void][ODP.W]::GetWindowText($front, $title, 512)
    Note "front: $($title.ToString())"
    $rows = New-Object System.Collections.ArrayList
    $proc = [ODP.W+EnumProc]{
        param($h, $p)
        $c = New-Object System.Text.StringBuilder 256; [void][ODP.W]::GetClassName($h, $c, 256)
        $t = New-Object System.Text.StringBuilder 512; [void][ODP.W]::GetWindowText($h, $t, 512)
        $r = New-Object ODP.W+RECT; [void][ODP.W]::GetWindowRect($h, [ref]$r)
        $style = [ODP.W]::GetWindowLong($h, -16)
        [void]$rows.Add(('{0} hwnd={1} id={2} visible={3} style=0x{4:X8} rect={5},{6},{7},{8} text={9}' -f $c, $h, [ODP.W]::GetDlgCtrlID($h), [ODP.W]::IsWindowVisible($h), $style, $r.Left, $r.Top, $r.Right, $r.Bottom, $t.ToString()))
        $true
    }
    [void][ODP.W]::EnumChildWindows($front, $proc, [IntPtr]::Zero)
    $rows | ForEach-Object { Note $_ }
    & $lab window-tree > (Join-Path $OutDir 'window-tree.json') 2>&1
    Start-Process -FilePath $keysExe -ArgumentList @('--allow', 'okur-ui', '"Escape"', '"wait:1500"') -WindowStyle Hidden -Wait | Out-Null
} finally {
    if ($run -and -not $run.HasExited) { Stop-Process -Id $run.Id -Force }
    Get-Process okur-ui -ErrorAction SilentlyContinue | Stop-Process -Force
    Jaws 'Start'
}
Note '--- done'
'done'

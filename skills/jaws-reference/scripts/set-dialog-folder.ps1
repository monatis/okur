<#
.SYNOPSIS
    Makes Notepad's file dialogs open in the dialog fixture's folder on the reference machine.

.DESCRIPTION
    Windows opens an application's Open and Save As dialogs in the folder it last used there.
    A dialog scenario recorded in the lab account's home folder lists that folder's contents
    in its traces, which do not belong in a public corpus. This opens the dialog fixture
    (corpus\fixtures\desktop-notepad-dialogs.ps1, or -Fixture), opens its file again through
    Notepad's Open dialog by its full path, and closes Notepad, so Notepad's last folder is
    the fixture's. Run it before a batch of dialog scenarios, never during one (it types into
    the window in front). Prints the folder Notepad was left on.
#>
param(
    [string]$Fixture = (Join-Path $env:USERPROFILE 'corpus\fixtures\desktop-notepad-dialogs.ps1')
)
$ErrorActionPreference = 'Stop'
if (-not ([System.Management.Automation.PSTypeName]'SrDialogInput').Type) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class SrDialogInput {
    [StructLayout(LayoutKind.Sequential)] struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Sequential)] struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Explicit)] struct UNION { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
    [StructLayout(LayoutKind.Sequential)] struct INPUT { public uint type; public UNION u; }
    [DllImport("user32.dll", SetLastError = true)] static extern uint SendInput(uint n, INPUT[] inputs, int size);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    static bool Send(ushort vk, ushort scan, uint flags) {
        INPUT[] input = new INPUT[2];
        for (int i = 0; i < 2; i++) {
            input[i].type = 1;
            input[i].u.ki.wVk = vk;
            input[i].u.ki.wScan = scan;
            input[i].u.ki.dwFlags = flags | (i == 1 ? 0x2u : 0u);
        }
        return SendInput(2, input, Marshal.SizeOf(typeof(INPUT))) == 2;
    }
    public static bool Char(char c) { return Send(0, c, 0x4); }
    public static bool Enter() { return Send(0x0D, 0, 0); }
    public static bool CtrlO() {
        INPUT[] input = new INPUT[4];
        ushort[] vks = { 0x11, 0x4F, 0x4F, 0x11 };
        for (int i = 0; i < 4; i++) {
            input[i].type = 1;
            input[i].u.ki.wVk = vks[i];
            input[i].u.ki.dwFlags = i >= 2 ? 0x2u : 0u;
        }
        return SendInput(4, input, Marshal.SizeOf(typeof(INPUT))) == 4;
    }
}
"@
}

function Wait-Window([string]$title, [int]$seconds = 20) {
    for ($i = 0; $i -lt $seconds * 4; $i++) {
        $p = Get-Process notepad -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowTitle -eq $title } | Select-Object -First 1
        if ($p) { return $p }
        Start-Sleep -Milliseconds 250
    }
    throw "no window '$title'"
}

Get-Process notepad -ErrorAction SilentlyContinue | Stop-Process -Force
& powershell -NoProfile -ExecutionPolicy Bypass -File $Fixture
$title = 'sr-dialog-fixture.txt - Notepad'
$notepad = Wait-Window $title
[void][SrDialogInput]::SetForegroundWindow($notepad.MainWindowHandle)
Start-Sleep -Milliseconds 800
[void][SrDialogInput]::CtrlO()
Start-Sleep -Seconds 3
$path = Join-Path (Join-Path $env:TEMP 'sr-dialogs') 'sr-dialog-fixture.txt'
foreach ($c in $path.ToCharArray()) { [void][SrDialogInput]::Char($c); Start-Sleep -Milliseconds 30 }
Start-Sleep -Milliseconds 500
[void][SrDialogInput]::Enter()
Start-Sleep -Seconds 3
$notepad = Wait-Window $title
Stop-Process -Id $notepad.Id -Force
"Notepad's dialogs now open in $(Split-Path $path)"

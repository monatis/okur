<#
.SYNOPSIS
    Puts the reference machine's keyboard and foreground back as a trial expects them:
    releases modifier keys left down, and hides JAWS's own main window (class JFWUI2), giving
    the foreground to the window below it.

.DESCRIPTION
    run-scenario.ps1 runs this script elevated, because JAWS runs elevated and Windows refuses
    input and foreground changes from ordinary processes while one of JAWS's windows is in
    front (old-pc, 2026-10-18):
    - A chord's key release sent while a JAWS window came forward never arrives: left Ctrl
      stayed down, and Ctrl+R became Insert+Ctrl+R (JAWS's regions list), Win+R JAWS's
      Research It prompt. An elevated release always arrives.
    - JAWS's main window comes forward after its dialogs close (the Links List) and keeps the
      foreground from the next fixture (223-226 heard nothing). Hidden, as when JAWS starts,
      it still has the foreground, which this script passes on, as Alt+Tab would, to the next
      titled window; run-scenario.ps1 then brings the fixture forward as usual.
    JAWS keeps running; only its window is hidden.
#>
param(
    # Only release keys: a scenario step may have one of JAWS's dialogs up, whose main window
    # stays as it is then.
    [switch]$KeysOnly
)
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class SrLabState {
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string name);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern IntPtr GetTopWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr hWnd, uint cmd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, IntPtr pid);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr hWnd, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string cls, string name);
    [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hWnd, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr w, IntPtr l);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint from, uint to, bool attach);
    [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vk);
    [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint mapType);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);

    const uint GW_HWNDNEXT = 2, GW_OWNER = 4;
    const uint KEYEVENTF_EXTENDEDKEY = 1, KEYEVENTF_KEYUP = 2;

    /// Releases the modifier keys Windows has down: Ctrl, Alt, Shift and Windows on either
    /// side, and Insert (the JAWS key). Returns how many were down.
    public static int ReleaseModifiers() {
        int[] keys = { 0xA2, 0xA3, 0xA4, 0xA5, 0xA0, 0xA1, 0x5B, 0x5C, 0x2D };
        int released = 0;
        foreach (int vk in keys) {
            if ((GetAsyncKeyState(vk) & 0x8000) == 0) continue;
            bool extended = vk == 0xA3 || vk == 0xA5 || vk == 0x5B || vk == 0x5C || vk == 0x2D;
            byte scan = (byte)MapVirtualKey((uint)vk, 0);
            keybd_event((byte)vk, scan, KEYEVENTF_KEYUP | (extended ? KEYEVENTF_EXTENDEDKEY : 0), UIntPtr.Zero);
            released++;
        }
        // JAWS keeps its own idea of a held JAWS key, which Windows' key state does not show:
        // after a lost release, every Ctrl+R was Insert+Ctrl+R to it (its regions list). A bare
        // release of Insert and CapsLock, harmless when nothing is held, resets it.
        keybd_event(0x2D, (byte)MapVirtualKey(0x2D, 0), KEYEVENTF_KEYUP | KEYEVENTF_EXTENDEDKEY, UIntPtr.Zero);
        keybd_event(0x14, (byte)MapVirtualKey(0x14, 0), KEYEVENTF_KEYUP, UIntPtr.Zero);
        return released;
    }

/// JAWS's dialog boxes (`#32770`) visible on the screen, of the process `jaws`.
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

    /// Closes JAWS's dialog boxes left open, by their Cancel, No or Close button as a user
    /// would (a button press reaches them from an elevated process), or with WM_CLOSE.
    public static int CloseJawsDialogs(uint jaws) {
        string[] buttons = { "&Cancel", "Cancel", "&No", "No", "&Close", "Close" };
        var dialogs = JawsDialogs(jaws);
        foreach (IntPtr dialog in dialogs) {
            IntPtr button = IntPtr.Zero;
            foreach (string name in buttons) {
                button = FindWindowEx(dialog, IntPtr.Zero, "Button", name);
                if (button != IntPtr.Zero) break;
            }
            if (button != IntPtr.Zero) {
                SendMessage(button, 0x00F5, IntPtr.Zero, IntPtr.Zero);
            } else {
                PostMessage(dialog, 0x0010, IntPtr.Zero, IntPtr.Zero);
            }
        }
        return dialogs.Count;
    }

    /// The first visible, titled, unowned top-level window in z-order other than `skip`.
    public static IntPtr NextWindow(IntPtr skip) {
        for (IntPtr h = GetTopWindow(IntPtr.Zero); h != IntPtr.Zero; h = GetWindow(h, GW_HWNDNEXT)) {
            if (h == skip || !IsWindowVisible(h) || IsIconic(h)) continue;
            if (GetWindow(h, GW_OWNER) != IntPtr.Zero || GetWindowTextLength(h) == 0) continue;
            return h;
        }
        return IntPtr.Zero;
    }

    /// Gives `target` the foreground from the foreground window's own input thread.
    public static bool Activate(IntPtr target) {
        uint foreground = GetWindowThreadProcessId(GetForegroundWindow(), IntPtr.Zero);
        uint me = GetCurrentThreadId();
        bool attached = foreground != me && AttachThreadInput(me, foreground, true);
        bool ok = SetForegroundWindow(target);
        if (attached) AttachThreadInput(me, foreground, false);
        return ok;
    }
}
"@
[void][SrLabState]::ReleaseModifiers()
if ($KeysOnly) { return }
# JAWS's dialogs left open by a scenario before (its regions list, its Research It prompt)
# take the foreground at every trial's start (old-pc, 2026-10-18).
$jaws = Get-Process jfw -ErrorAction SilentlyContinue | Select-Object -First 1
if ($jaws) { [void][SrLabState]::CloseJawsDialogs([uint32]$jaws.Id) }
$window = [SrLabState]::FindWindow('JFWUI2', [NullString]::Value)
if ($window -ne [IntPtr]::Zero) {
    # SW_HIDE
    [void][SrLabState]::ShowWindow($window, 0)
    if ([SrLabState]::GetForegroundWindow() -eq $window) {
        $next = [SrLabState]::NextWindow($window)
        if ($next -ne [IntPtr]::Zero) { [void][SrLabState]::Activate($next) }
    }
}

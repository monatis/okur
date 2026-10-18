<#
.SYNOPSIS
    Hides JAWS's own main window (class JFWUI2), as it is when JAWS starts, and gives the
    foreground to the window below it.

.DESCRIPTION
    JAWS's main window comes forward after its dialogs close (the Links List) and, JAWS running
    elevated, no ordinary process can take the foreground back from it: the next scenario's
    fixture never gets the keys (223-226, old-pc, 2026-10-18). run-scenario.ps1 runs this script
    elevated when the window is visible. Hidden, the window keeps the foreground, so this script
    passes it on, as Alt+Tab would, to the next titled window; run-scenario.ps1 then brings the
    fixture forward as usual. JAWS keeps running; only its window is hidden.
#>
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class SrJawsWindow {
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
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint from, uint to, bool attach);

    const uint GW_HWNDNEXT = 2, GW_OWNER = 4;

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
$window = [SrJawsWindow]::FindWindow('JFWUI2', [NullString]::Value)
if ($window -ne [IntPtr]::Zero) {
    # SW_HIDE
    [void][SrJawsWindow]::ShowWindow($window, 0)
    if ([SrJawsWindow]::GetForegroundWindow() -eq $window) {
        $next = [SrJawsWindow]::NextWindow($window)
        if ($next -ne [IntPtr]::Zero) { [void][SrJawsWindow]::Activate($next) }
    }
}

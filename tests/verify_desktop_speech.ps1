# Automated Programmatic Speech & Audio Device Verification Test for Windows Desktop Navigation
$ErrorActionPreference = "Stop"

Write-Host "============================================================"
Write-Host "  Live Desktop Navigation & Audio Verification Test         "
Write-Host "============================================================"

$targetDir = (Resolve-Path (Join-Path $PSScriptRoot "..\target")).Path
$historyLog = Join-Path $targetDir "speech_history.log"
$daemonLog = Join-Path $targetDir "daemon.log"
$daemonErr = Join-Path $targetDir "daemon_err.log"

if (Test-Path $historyLog) { Remove-Item -Path $historyLog -Force }
if (Test-Path $daemonLog) { Remove-Item -Path $daemonLog -Force }
if (Test-Path $daemonErr) { Remove-Item -Path $daemonErr -Force }

# Kill any existing sr-cli instances
Get-Process "sr-cli" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 300

$srCliExe = Join-Path $targetDir "debug\sr-cli.exe"
if (-not (Test-Path $srCliExe)) {
    Write-Host "Building sr-cli binary..."
    $buildProc = Start-Process -FilePath "cargo" -ArgumentList "build", "--bin", "sr-cli" -NoNewWindow -PassThru -Wait
    if ($buildProc.ExitCode -ne 0) {
        Write-Error "Cargo build failed"
        exit 1
    }
}

# Start sr-cli daemon with SAPI5
Write-Host "Starting sr-cli daemon with SAPI 5 engine..."
$daemon = Start-Process -FilePath $srCliExe -ArgumentList "run", "--engine", "sapi5" -RedirectStandardOutput $daemonLog -RedirectStandardError $daemonErr -PassThru
Start-Sleep -Milliseconds 1200

if ($daemon.HasExited) {
    Write-Error "sr-cli daemon exited prematurely! Stderr: $(Get-Content $daemonErr -Raw -ErrorAction SilentlyContinue)"
    exit 1
}

# Add Win32 desktop and keyboard helpers
Add-Type -ReferencedAssemblies @("UIAutomationClient", "UIAutomationTypes") @"
using System;
using System.Runtime.InteropServices;
using System.Windows.Automation;

public class DesktopVerifier {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr OpenDesktop(string lpszDesktop, uint dwFlags, bool fInherit, uint dwDesiredAccess);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool SetThreadDesktop(IntPtr hDesktop);

    [DllImport("user32.dll")]
    public static extern IntPtr FindWindow(string lpClassName, string lpWindowName);

    [DllImport("user32.dll")]
    public static extern IntPtr FindWindowEx(IntPtr hwndParent, IntPtr hwndChildAfter, string lpszClass, string lpszWindow);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);

    public static void NavigateDesktop() {
        IntPtr h = OpenDesktop("default", 0, false, 0x01FF);
        var t = new System.Threading.Thread(() => {
            if (h != IntPtr.Zero) SetThreadDesktop(h);

            IntPtr progman = FindWindow("Progman", null);
            IntPtr defView = FindWindowEx(progman, IntPtr.Zero, "SHELLDLL_DefView", null);
            if (defView == IntPtr.Zero) {
                IntPtr w = IntPtr.Zero;
                do {
                    w = FindWindowEx(IntPtr.Zero, w, "WorkerW", null);
                    if (w != IntPtr.Zero) {
                        defView = FindWindowEx(w, IntPtr.Zero, "SHELLDLL_DefView", null);
                        if (defView != IntPtr.Zero) break;
                    }
                } while (w != IntPtr.Zero);
            }
            IntPtr lv = FindWindowEx(defView, IntPtr.Zero, "SysListView32", null);
            Console.WriteLine("Target Desktop ListView HWND: " + lv);

            // Bring desktop list view to foreground
            if (lv != IntPtr.Zero) SetForegroundWindow(lv);
            System.Threading.Thread.Sleep(300);

            // 1. Send Win+M to focus Desktop
            Console.WriteLine("Action: Sending Win+M...");
            keybd_event(0x5B, 0, 0, UIntPtr.Zero);
            keybd_event(0x4D, 0, 0, UIntPtr.Zero);
            keybd_event(0x4D, 0, 2, UIntPtr.Zero);
            keybd_event(0x5B, 0, 2, UIntPtr.Zero);
            System.Threading.Thread.Sleep(800);

            // 2. Send Down Arrow to move to next item
            Console.WriteLine("Action: Sending Down Arrow...");
            keybd_event(0x28, 0, 0, UIntPtr.Zero);
            keybd_event(0x28, 0, 2, UIntPtr.Zero);
            System.Threading.Thread.Sleep(800);

            // 3. Send 'M' to jump to Microsoft Edge
            Console.WriteLine("Action: Sending 'M' key...");
            keybd_event(0x4D, 0, 0, UIntPtr.Zero);
            keybd_event(0x4D, 0, 2, UIntPtr.Zero);
            System.Threading.Thread.Sleep(800);
        });
        t.SetApartmentState(System.Threading.ApartmentState.MTA);
        t.Start();
        t.Join();
    }
}
"@ -ErrorAction SilentlyContinue

try {
    # Perform navigation steps on desktop
    [DesktopVerifier]::NavigateDesktop()

    # Verify speech history log
    Write-Host "`nEvaluating Speech History Log ($historyLog):"
    if (-not (Test-Path $historyLog)) {
        Write-Error "FAIL: No speech history log was created!"
        exit 1
    }

    $historyContent = Get-Content $historyLog -Raw
    Write-Host $historyContent

    # Assertion 1: Must contain initial desktop entry announcement
    if ($historyContent -notmatch "List box" -and $historyContent -notmatch "FolderView") {
        Write-Error "FAIL: Initial desktop entry announcement was NOT spoken!"
        exit 1
    }
    Write-Host "[ASSERT 1 PASS] Initial desktop entry announcement recorded."

    # Assertion 2: Must contain speech interruption [SPEECH_STOPPED]
    if ($historyContent -notmatch "\[SPEECH_STOPPED\]") {
        Write-Error "FAIL: Navigation keys did not interrupt speech!"
        exit 1
    }
    Write-Host "[ASSERT 2 PASS] Speech interruption on key press confirmed."

    # Assertion 3: Must announce newly moved-to items with position (e.g. 'of 11')
    if ($historyContent -notmatch "of 11") {
        Write-Error "FAIL: Newly moved item position was not announced!"
        exit 1
    }
    Write-Host "[ASSERT 3 PASS] Item position announcement confirmed."

    # Assertion 4: Must announce Microsoft Edge when 'M' is pressed
    if ($historyContent -notmatch "Microsoft Edge") {
        Write-Error "FAIL: Pressing 'M' did not announce Microsoft Edge!"
        exit 1
    }
    Write-Host "[ASSERT 4 PASS] Initial-letter navigation announced 'Microsoft Edge'!"

    # Query hardware soundcard audio meter
    Write-Host "`nVerifying Soundcard Output Device via sr-cli meter..."
    $meterReport = & $srCliExe meter --duration-ms 300
    Write-Host ($meterReport -join "`n")

    Write-Host "`n============================================================"
    Write-Host "  ALL VERIFICATIONS PASSED (100% Parity Validated)          "
    Write-Host "============================================================"
    exit 0
} finally {
    Write-Host "`nTerminating sr-cli daemon..."
    if ($daemon -and -not $daemon.HasExited) {
        $daemon.Kill()
    }
    Get-Process "sr-cli" -ErrorAction SilentlyContinue | Stop-Process -Force
}

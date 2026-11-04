<#
.SYNOPSIS
    Deterministic native desktop fixture for comctl32's own tool tips (tooltips_class32): the
    balloon tips edit controls show, and a tool tip shown beside a button, all without the
    mouse.

.DESCRIPTION
    Launched by run-scenario.ps1 for desktop scenarios (scenario "launch" block). Top to bottom:
    - "Quantity:" and its edit, which has the focus: a numbers-only edit (ES_NUMBER). A letter
      typed in it is refused, and the edit itself shows its balloon tip, "Unacceptable
      Character" / "You can only type a number here." (comctl32's, as Windows shows it in any
      numbers-only field).
    - "Password:" and its password edit: taking the focus, it is asked to show a balloon tip
      300 ms later (EM_SHOWBALLOONTIP), "Password" / "Use at least 8 characters.", with the
      information icon, as an application warns of a field's rule.
    - A "Save" button: taking the focus, it shows a tool tip 300 ms later below itself, "Save
      the order", for four seconds: a tooltips_class32 window the fixture creates and tracks
      (TTF_TRACK), comctl32's own class, not WinForms' (WindowsForms10.tooltips_class32...).
      Losing the focus hides it.
    The file is ASCII only. Do not change it without re-recording every scenario that uses it
    (the manifest pins its SHA-256).
#>
param([int]$ExitAfterSeconds = 900)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class SrBalloonTips {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }

    // TOOLINFOW up to lParam (TTTOOLINFOW_V2_SIZE), which every comctl32 since 4.70 takes.
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct TOOLINFO {
        public int cbSize;
        public int uFlags;
        public IntPtr hwnd;
        public IntPtr uId;
        public RECT rect;
        public IntPtr hinst;
        public string lpszText;
        public IntPtr lParam;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct EDITBALLOONTIP {
        public int cbStruct;
        public string pszTitle;
        public string pszText;
        public int ttiIcon;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct INITCOMMONCONTROLSEX { public int dwSize; public int dwICC; }

    [DllImport("comctl32.dll")]
    static extern bool InitCommonControlsEx(ref INITCOMMONCONTROLSEX icc);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern IntPtr CreateWindowEx(int exStyle, string className, string windowName,
        uint style, int x, int y, int width, int height, IntPtr parent, IntPtr menu,
        IntPtr instance, IntPtr param);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern IntPtr SendMessage(IntPtr hwnd, int msg, IntPtr wParam, ref TOOLINFO lParam);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern IntPtr SendMessage(IntPtr hwnd, int msg, IntPtr wParam, ref EDITBALLOONTIP lParam);
    [DllImport("user32.dll")]
    static extern IntPtr SendMessage(IntPtr hwnd, int msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")]
    static extern int GetWindowLong(IntPtr hwnd, int index);
    [DllImport("user32.dll")]
    static extern int SetWindowLong(IntPtr hwnd, int index, int value);

    const int GWL_STYLE = -16;
    const int ES_NUMBER = 0x2000;
    const int EM_SHOWBALLOONTIP = 0x1503;
    const int TTM_TRACKACTIVATE = 0x0411;
    const int TTM_TRACKPOSITION = 0x0412;
    const int TTM_ADDTOOLW = 0x0432;
    const int TTF_IDISHWND = 0x0001;
    const int TTF_TRACK = 0x0020;
    const int TTF_ABSOLUTE = 0x0080;
    const uint WS_POPUP = 0x80000000;
    const uint TTS_ALWAYSTIP = 0x01;
    const uint TTS_NOPREFIX = 0x02;
    const int WS_EX_TOPMOST = 0x00000008;
    const int ICC_BAR_CLASSES = 0x00000004;

    // An edit takes digits only; any other character typed shows its balloon tip.
    public static void NumbersOnly(IntPtr edit) {
        SetWindowLong(edit, GWL_STYLE, GetWindowLong(edit, GWL_STYLE) | ES_NUMBER);
    }

    // An edit's balloon tip (TTI_INFO is 1).
    public static void ShowBalloon(IntPtr edit, string title, string text, int icon) {
        var tip = new EDITBALLOONTIP {
            cbStruct = Marshal.SizeOf(typeof(EDITBALLOONTIP)),
            pszTitle = title,
            pszText = text,
            ttiIcon = icon
        };
        SendMessage(edit, EM_SHOWBALLOONTIP, IntPtr.Zero, ref tip);
    }

    static TOOLINFO Info(IntPtr owner, string text) {
        return new TOOLINFO {
            cbSize = Marshal.SizeOf(typeof(TOOLINFO)),
            uFlags = TTF_IDISHWND | TTF_TRACK | TTF_ABSOLUTE,
            hwnd = owner,
            uId = owner,
            lpszText = text
        };
    }

    // A tracking tool tip for `owner` with `text`, not shown yet.
    public static IntPtr CreateTip(IntPtr owner, string text) {
        var icc = new INITCOMMONCONTROLSEX {
            dwSize = Marshal.SizeOf(typeof(INITCOMMONCONTROLSEX)),
            dwICC = ICC_BAR_CLASSES
        };
        InitCommonControlsEx(ref icc);
        IntPtr tip = CreateWindowEx(WS_EX_TOPMOST, "tooltips_class32", null,
            WS_POPUP | TTS_NOPREFIX | TTS_ALWAYSTIP, 0, 0, 0, 0, owner, IntPtr.Zero,
            IntPtr.Zero, IntPtr.Zero);
        var info = Info(owner, text);
        SendMessage(tip, TTM_ADDTOOLW, IntPtr.Zero, ref info);
        return tip;
    }

    // Shows the tool tip with its top left corner at screen point x, y, or hides it.
    public static void Track(IntPtr tip, IntPtr owner, string text, int x, int y, bool show) {
        var info = Info(owner, text);
        if (show) {
            SendMessage(tip, TTM_TRACKPOSITION, IntPtr.Zero,
                (IntPtr)(((y & 0xFFFF) << 16) | (x & 0xFFFF)));
        }
        SendMessage(tip, TTM_TRACKACTIVATE, (IntPtr)(show ? 1 : 0), ref info);
    }
}
'@

$form = New-Object System.Windows.Forms.Form
$form.Text = 'SR Balloon Tip Fixture'
$form.ClientSize = New-Object System.Drawing.Size(520, 260)
$form.StartPosition = 'CenterScreen'
$form.Font = New-Object System.Drawing.Font('Segoe UI', 12)

$quantityLabel = New-Object System.Windows.Forms.Label -Property @{ Text = 'Quantity:'; AutoSize = $true }
$quantityLabel.Location = New-Object System.Drawing.Point(30, 33)
$quantity = New-Object System.Windows.Forms.TextBox -Property @{ TabIndex = 0 }
$quantity.Location = New-Object System.Drawing.Point(160, 30)
$quantity.Size = New-Object System.Drawing.Size(200, 30)

$passwordLabel = New-Object System.Windows.Forms.Label -Property @{ Text = 'Password:'; AutoSize = $true }
$passwordLabel.Location = New-Object System.Drawing.Point(30, 93)
$password = New-Object System.Windows.Forms.TextBox -Property @{ TabIndex = 1; UseSystemPasswordChar = $true }
$password.Location = New-Object System.Drawing.Point(160, 90)
$password.Size = New-Object System.Drawing.Size(200, 30)

$save = New-Object System.Windows.Forms.Button -Property @{ Text = 'Save'; TabIndex = 2 }
$save.Location = New-Object System.Drawing.Point(160, 150)
$save.Size = New-Object System.Drawing.Size(150, 50)

$form.Controls.AddRange(@($quantityLabel, $quantity, $passwordLabel, $password, $save))

$script:saveTip = [IntPtr]::Zero
$saveText = 'Save the order'

# The password field's balloon, 300 ms after it took the focus.
$passwordTimer = New-Object System.Windows.Forms.Timer
$passwordTimer.Interval = 300
$passwordTimer.Add_Tick({
    $passwordTimer.Stop()
    if ($password.Focused) {
        [SrBalloonTips]::ShowBalloon($password.Handle, 'Password', 'Use at least 8 characters.', 1)
    }
})
$password.Add_GotFocus({ $passwordTimer.Stop(); $passwordTimer.Start() })

# The Save button's tool tip, 300 ms after it took the focus, for four seconds.
$hideSaveTip = New-Object System.Windows.Forms.Timer
$hideSaveTip.Interval = 4000
$hideSaveTip.Add_Tick({
    $hideSaveTip.Stop()
    [SrBalloonTips]::Track($script:saveTip, $save.Handle, $saveText, 0, 0, $false)
})
$showSaveTip = New-Object System.Windows.Forms.Timer
$showSaveTip.Interval = 300
$showSaveTip.Add_Tick({
    $showSaveTip.Stop()
    if ($save.Focused) {
        $below = $save.PointToScreen((New-Object System.Drawing.Point(0, ($save.Height + 4))))
        [SrBalloonTips]::Track($script:saveTip, $save.Handle, $saveText, $below.X, $below.Y, $true)
        $hideSaveTip.Stop()
        $hideSaveTip.Start()
    }
})
$save.Add_GotFocus({ $showSaveTip.Stop(); $showSaveTip.Start() })
$save.Add_LostFocus({
    $showSaveTip.Stop()
    $hideSaveTip.Stop()
    [SrBalloonTips]::Track($script:saveTip, $save.Handle, $saveText, 0, 0, $false)
})

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $ExitAfterSeconds * 1000
$timer.Add_Tick({ $form.Close() })
$timer.Start()

$form.Add_Shown({
    [SrBalloonTips]::NumbersOnly($quantity.Handle)
    $script:saveTip = [SrBalloonTips]::CreateTip($save.Handle, $saveText)
    $quantity.Focus() | Out-Null
    $form.Activate()
})
[void]$form.ShowDialog()

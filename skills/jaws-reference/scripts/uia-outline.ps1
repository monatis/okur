# Research, never evidence: an outline of a window's UI Automation tree (raw view), for learning
# JAWS's own dialogs as a black box: their controls, classes, names and states. Run it elevated,
# as JAWS's windows are: Start-Process powershell -Verb RunAs -ArgumentList ... (old-pc elevates
# without asking). A custom control (Quick Settings' tree) shows no items: walk it with keys
# instead (jaws-ui-walk.ps1).
#   -Title    the window's title, a prefix (default: the window in front)
#   -Expand   expand collapsed tree items and menus to list what they hold
#   -Out      the file written (UTF-8)
param([string]$Title = '', [switch]$Expand, [int]$MaxDepth = 40, [int]$MaxNodes = 6000,
      [string]$Out = "$env:TEMP\uia-dump.txt")
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$A = [System.Windows.Automation.AutomationElement]
$walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
Add-Type -Namespace Dump -Name W -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();'
if ($Title) {
    $root = $A::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition) |
        Where-Object { $_.Current.Name -like "$Title*" } | Select-Object -First 1
} else {
    $root = $A::FromHandle([Dump.W]::GetForegroundWindow())
}
if (-not $root) { "no window $Title" | Out-File $Out -Encoding utf8; exit 1 }
$lines = New-Object System.Collections.Generic.List[string]
$count = 0
function State($e) {
    $parts = @()
    $p = $null
    if ($e.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$p)) { $parts += "toggle=" + $p.Current.ToggleState }
    if ($e.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$p)) { $v = $p.Current.Value; if ($v) { $parts += "value=[" + $v + "]" } }
    if ($e.TryGetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern, [ref]$p)) { if ($p.Current.IsSelected) { $parts += "selected" } }
    if ($e.TryGetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern, [ref]$p)) { $parts += "expand=" + $p.Current.ExpandCollapseState }
    if ($e.TryGetCurrentPattern([System.Windows.Automation.RangeValuePattern]::Pattern, [ref]$p)) { $parts += "range=" + $p.Current.Value + "(" + $p.Current.Minimum + ".." + $p.Current.Maximum + ")" }
    if (-not $e.Current.IsEnabled) { $parts += "disabled" }
    if ($e.Current.HasKeyboardFocus) { $parts += "FOCUS" }
    $ak = $e.Current.AccessKey; if ($ak) { $parts += "key=" + $ak }
    $parts -join ' '
}
function Walk($e, [int]$depth) {
    if ($script:count -ge $MaxNodes -or $depth -gt $MaxDepth) { return }
    $script:count++
    $c = $e.Current
    $type = $c.ControlType.ProgrammaticName -replace '^ControlType\.', ''
    $help = if ($c.HelpText) { " help=[" + $c.HelpText + "]" } else { '' }
    $lines.Add((' ' * (2 * $depth)) + "$type [" + $c.Name + "] class=" + $c.ClassName + ' ' + (State $e) + $help)
    if ($Expand) {
        $p = $null
        if ($type -in @('TreeItem', 'MenuItem') -and $e.TryGetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern, [ref]$p)) {
            if ($p.Current.ExpandCollapseState -eq 'Collapsed') { try { $p.Expand(); Start-Sleep -Milliseconds 120 } catch {} }
        }
    }
    $child = $walker.GetFirstChild($e)
    while ($child) {
        Walk $child ($depth + 1)
        $child = $walker.GetNextSibling($child)
    }
}
Walk $root 0
$lines | Out-File $Out -Encoding utf8
"$count nodes"

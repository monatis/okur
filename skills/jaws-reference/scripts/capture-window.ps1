<#
.SYNOPSIS
    Writes a window's UI Automation tree to JSON: every element's type, name, value and state,
    nested as the control view shows them. For reading JAWS's own dialogs on old-pc (Customize
    Punctuation's symbol list, Voice Adjustment, the verbosity lists): JAWS's interface, which
    AGENTS.md §1.4 allows reading, as a user would.

.DESCRIPTION
    JAWS's windows run elevated, and Windows hides an elevated window's contents from an
    ordinary process, so run this elevated (old-pc elevates without asking), through -Command
    so that its output and errors land in a log:
        $cmd = "& '<this>' -Title 'Customize Punctuation' -OutputPath '<json>' *> '<log>'"
        Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList
          '-NoProfile','-ExecutionPolicy','Bypass','-Command',$cmd
    JAWS's Settings Center exposes its controls as panes, with their names, values and the
    keyboard focus ("focused"). Keys to it go through an elevated `okur keys --allow
    SettingsCenter`; on old-pc's Turkish Q layout the I key types a dotless ı.
    The window is the first top-level window whose name contains -Title, or the window in front
    without it. Research output, never evidence.
#>
param(
    [string]$Title,
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [int]$MaxDepth = 14
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$A = [System.Windows.Automation.AutomationElement]

if ($Title) {
    $window = $A::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children,
        [System.Windows.Automation.Condition]::TrueCondition) |
        Where-Object { $_.Current.Name -like "*$Title*" } | Select-Object -First 1
} else {
    Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class CaptureWindowFront {
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}
"@
    $window = $A::FromHandle([CaptureWindowFront]::GetForegroundWindow())
}
if (-not $window) { throw "No window named like '$Title'." }

$walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker

function Get-Node($element, [int]$depth) {
    $c = $element.Current
    $node = [ordered]@{
        type = $c.ControlType.ProgrammaticName -replace '^ControlType\.', ''
        name = $c.Name
    }
    if ($c.AutomationId) { $node.automation_id = $c.AutomationId }
    if ($c.ClassName) { $node.class = $c.ClassName }
    $pattern = $null
    if ($element.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$pattern)) {
        $node.value = $pattern.Current.Value
    }
    if ($element.TryGetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern, [ref]$pattern)) {
        if ($pattern.Current.IsSelected) { $node.selected = $true }
    }
    if ($element.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$pattern)) {
        $node.toggle = [string]$pattern.Current.ToggleState
    }
    if (-not $c.IsEnabled) { $node.enabled = $false }
    if ($c.HasKeyboardFocus) { $node.focused = $true }
    if ($depth -lt $MaxDepth) {
        $children = @()
        $child = $walker.GetFirstChild($element)
        while ($child) {
            $children += ,(Get-Node $child ($depth + 1))
            $child = $walker.GetNextSibling($child)
        }
        if ($children.Count -gt 0) { $node.children = $children }
    }
    return $node
}

$tree = [ordered]@{
    captured = (Get-Date -Format o)
    window = $window.Current.Name
    process_id = $window.Current.ProcessId
    root = Get-Node $window 0
}
$dir = Split-Path -Parent $OutputPath
if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
[System.IO.File]::WriteAllText($OutputPath, ($tree | ConvertTo-Json -Depth 40), (New-Object System.Text.UTF8Encoding $false))
"wrote $OutputPath"

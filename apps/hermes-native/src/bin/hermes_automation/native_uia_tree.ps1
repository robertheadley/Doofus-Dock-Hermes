$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient

$rootElement = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]([long]$targetHwnd))
if ($null -eq $rootElement) {
    throw 'UI Automation could not resolve the Hermes native window handle.'
}

$walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker

$nodes = [System.Collections.Generic.List[object]]::new()
$queue = [System.Collections.Generic.Queue[object]]::new()
$queue.Enqueue(@{ element = $rootElement; parent = $null; depth = 0 })
$maxNodes = 256
$maxDepth = 16

while ($queue.Count -gt 0 -and $nodes.Count -lt $maxNodes) {
    $item = $queue.Dequeue()
    $current = $item.element.Current
    $role = $current.ControlType.ProgrammaticName -replace '^ControlType\.', ''
    $name = $null
    if ($role -in @('Button', 'CheckBox', 'ComboBox', 'Edit', 'Group', 'Hyperlink', 'ListItem', 'MenuItem', 'Pane', 'RadioButton', 'Slider', 'SplitButton', 'Tab', 'TabItem', 'TreeItem', 'Window')) {
        $name = [string]$current.Name
        if ($name.Length -gt 160) { $name = $name.Substring(0, 160) }
    }
    $automationId = [string]$current.AutomationId
    if ($automationId.Length -gt 128) { $automationId = $automationId.Substring(0, 128) }
    $accesskitNodeId = $null
    $runtimeId = @()
    if ($role -in @('Button', 'CheckBox', 'ComboBox', 'Edit', 'Hyperlink', 'ListItem', 'MenuItem', 'RadioButton', 'SplitButton', 'TabItem', 'TreeItem')) {
        try {
            $runtimeId = $item.element.GetRuntimeId()
            if ($runtimeId.Length -eq 7 -and $runtimeId[0] -eq 42 -and [uint64]$runtimeId[1] -eq [uint64]$targetHwnd -and $runtimeId[5] -eq 0 -and $runtimeId[6] -eq 0) {
                $high = [uint64][BitConverter]::ToUInt32([BitConverter]::GetBytes([int]$runtimeId[3]), 0)
                $low = [uint64][BitConverter]::ToUInt32([BitConverter]::GetBytes([int]$runtimeId[4]), 0)
                $accesskitNodeId = [uint64](($high -shl 32) -bor $low)
            }
        } catch { }
    }
    $bounds = $current.BoundingRectangle
    $boundsX = [double]$bounds.Left
    $boundsY = [double]$bounds.Top
    $boundsWidth = [double]$bounds.Width
    $boundsHeight = [double]$bounds.Height
    $boundsValid = -not ([double]::IsNaN($boundsX) -or [double]::IsInfinity($boundsX) -or [double]::IsNaN($boundsY) -or [double]::IsInfinity($boundsY) -or [double]::IsNaN($boundsWidth) -or [double]::IsInfinity($boundsWidth) -or [double]::IsNaN($boundsHeight) -or [double]::IsInfinity($boundsHeight))
    if (-not $boundsValid) {
        $boundsX = 0.0
        $boundsY = 0.0
        $boundsWidth = 0.0
        $boundsHeight = 0.0
    }
    $isEnabled = [bool]$current.IsEnabled
    $isFocusable = [bool]$current.IsKeyboardFocusable
    $isOffscreen = [bool]$current.IsOffscreen
    $hasKeyboardFocus = [bool]$current.HasKeyboardFocus
    $index = $nodes.Count
    $actions = [System.Collections.Generic.List[string]]::new()
    if ($role -in @('Button', 'CheckBox', 'ComboBox', 'Hyperlink', 'ListItem', 'MenuItem', 'RadioButton', 'SplitButton', 'TabItem', 'TreeItem')) { $actions.Add('click') }
    if ($role -eq 'Edit' -and $isEnabled) { $actions.Add('set_value') }
    if ($role -eq 'Edit' -and $isFocusable) { $actions.Add('focus'); $actions.Add('replace_selected_text') }
    $node = @{
        id = $index
        parent_id = $item.parent
        children = [System.Collections.Generic.List[int]]::new()
        role = $role
        name = if ([string]::IsNullOrWhiteSpace($name)) { $null } else { $name }
        automation_id = if ([string]::IsNullOrWhiteSpace($automationId)) { $null } else { $automationId }
        accesskit_node_id = $accesskitNodeId
        runtime_id = if ($runtimeId.Length -gt 0) { @($runtimeId) } else { $null }
        actions = @($actions)
        enabled = $isEnabled
        focusable = $isFocusable
        offscreen = $isOffscreen
        focused = $hasKeyboardFocus
        bounds_valid = $boundsValid
        bounds = @{
            x = $boundsX
            y = $boundsY
            width = $boundsWidth
            height = $boundsHeight
        }
    }
    $nodes.Add($node)
    if ($null -ne $item.parent) { $nodes[$item.parent].children.Add([int]$index) }

    if ($item.depth -lt $maxDepth -and $nodes.Count -lt $maxNodes) {
        $child = $walker.GetFirstChild($item.element)
        while ($null -ne $child -and $nodes.Count + $queue.Count -lt $maxNodes) {
            $queue.Enqueue(@{ element = $child; parent = $index; depth = $item.depth + 1 })
            $child = $walker.GetNextSibling($child)
        }
    }
}

@{
    root = 0
    node_count = $nodes.Count
    truncated = ($queue.Count -gt 0)
    locator_mode = 'read-only UI Automation control view'
    nodes = @($nodes)
} | ConvertTo-Json -Depth 8 -Compress

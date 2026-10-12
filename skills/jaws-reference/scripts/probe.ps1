[CmdletBinding()]
param(
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"
$sw = [System.Diagnostics.Stopwatch]::StartNew()

$result = [ordered]@{
    Success = $false
    Timestamp = (Get-Date -Format "o")
    SessionId = (Get-Process -Id $PID).SessionId
    IsInteractiveSession = ((Get-Process -Id $PID).SessionId -ne 0)
    JawsRunning = $false
    JawsProcessId = $null
    JawsPath = $null
    JawsVersion = $null
    ComProgId = "FreedomSci.JawsApi"
    ComInstantiated = $false
    SpeechHistoryConfigured = $false
    RoundTripOk = $false
    RoundTripText = $null
    ElapsedMs = 0
    Errors = @()
}

try {
    $jawsProc = Get-Process jfw -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($jawsProc) {
        $result.JawsRunning = $true
        $result.JawsProcessId = $jawsProc.Id
        $jfwExePath = "C:\Program Files\Freedom Scientific\JAWS\2025\jfw.exe"
        if (Test-Path $jfwExePath) {
            $result.JawsPath = $jfwExePath
            $vi = (Get-Item $jfwExePath).VersionInfo
            $result.JawsVersion = "$($vi.ProductVersion) (File: $($vi.FileVersion))"
        }
    } else {
        $result.Errors += "JAWS process (jfw) is not running."
    }
} catch {
    $result.Errors += "Failed querying JAWS process: $($_.Exception.Message)"
}

$jaws = $null
try {
    $jaws = New-Object -ComObject "FreedomSci.JawsApi"
    if ($jaws) {
        $result.ComInstantiated = $true
    }
} catch {
    $result.Errors += "Failed instantiating FreedomSci.JawsApi COM object: $($_.Exception.Message)"
}

$jcfPath = "C:\ProgramData\Freedom Scientific\JAWS\2025\Settings\enu\Default.JCF"
try {
    if (Test-Path $jcfPath) {
        $jcfContent = Get-Content $jcfPath -Raw
        if ($jcfContent -match "SpeechHistory\s*=\s*1") {
            $result.SpeechHistoryConfigured = $true
        }
    }
} catch {}

if ($result.ComInstantiated -and $result.JawsRunning) {
    try {
        $testPhrase = "JAWS probe test $([guid]::NewGuid().ToString().Substring(0,8))"
        [void]$jaws.RunScript("ClearSpeechHistory")
        Start-Sleep -Milliseconds 120
        $sayOk = $jaws.SayString($testPhrase, $false)
        Start-Sleep -Milliseconds 450
        $copyOk = $jaws.RunScript("CopySpeechHistoryToClipboard")
        Start-Sleep -Milliseconds 200
        $clip = Get-Clipboard
        if ($clip -and $clip.Contains($testPhrase)) {
            $result.RoundTripOk = $true
            $result.RoundTripText = $clip.Trim()
        } else {
            $result.Errors += "Synthetic speech round-trip failed. Clipboard: '$clip'"
        }
    } catch {
        $result.Errors += "Synthetic speech round-trip exception: $($_.Exception.Message)"
    }
}

$sw.Stop()
$result.ElapsedMs = $sw.ElapsedMilliseconds
$result.Success = ($result.JawsRunning -and $result.ComInstantiated -and $result.RoundTripOk -and $result.IsInteractiveSession)

$jsonOutput = $result | ConvertTo-Json -Depth 4
if (-not $Quiet) {
    Write-Output $jsonOutput
}

if ($result.Success -eq $true) {
    exit 0
} else {
    exit 1
}

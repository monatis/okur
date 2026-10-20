<#
.SYNOPSIS
Okur's development code signing: one certificate on the dev box, its public part trusted on
old-pc, so that okur-sr.exe gets UI Access there.

.DESCRIPTION
docs/references/startup-and-secure-screens-proposal.md, section 5.11. Every step needs the
owner's yes (the proposal's question Q1). Nothing here touches bcdedit, test-signing mode,
Secure Boot, or the Trusted Publishers store: UI Access needs only a signature that chains to
a root in the local machine's Trusted Root store.

Run one step at a time, with -WhatIf first to see what it would do:

  -NewCertificate  On the dev box, unelevated, once. A certificate for code signing only, an
                   end entity (not a CA), its private key not exportable, valid for 12 months,
                   in Cert:\CurrentUser\My; its public part exported to -CertificateFile.
                   Write down the thumbprint it prints.
  -Sign            After each release build, on the dev box: signs the programs in -BuildDir
                   with -Thumbprint. Without a timestamp, a signature stops verifying when the
                   certificate expires: sign again, or make a new certificate.
  -Verify          Shows each program's signature status. On the dev box "UnknownError" is
                   expected: its root is trusted only on old-pc.
  -TrustOnLab      On old-pc, elevated, once: puts -CertificateFile's public certificate in
                   Cert:\LocalMachine\Root.
  -UndoLab         On old-pc, elevated: takes it out of Cert:\LocalMachine\Root.
  -UndoDevBox      On the dev box: deletes the certificate and its key.

Never copy the private key anywhere, and never trust this root on a machine someone else uses.

.EXAMPLE
.\dev-signing.ps1 -NewCertificate -WhatIf
.\dev-signing.ps1 -Sign -Thumbprint 0123456789ABCDEF0123456789ABCDEF01234567
.\dev-signing.ps1 -TrustOnLab -CertificateFile C:\Users\ailabs\sr-harness\okur-dev-codesign.cer
#>
[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [switch]$NewCertificate,
    [switch]$Sign,
    [switch]$Verify,
    [switch]$TrustOnLab,
    [switch]$UndoLab,
    [switch]$UndoDevBox,
    # The certificate's thumbprint, as -NewCertificate printed it (for -Sign, -UndoLab, -UndoDevBox).
    [string]$Thumbprint,
    # The public certificate: written by -NewCertificate, read by -TrustOnLab.
    [string]$CertificateFile = (Join-Path $env:USERPROFILE 'okur-dev-codesign.cer'),
    # Where the release build left the programs (-Sign, -Verify).
    [string]$BuildDir = 'target\release',
    [string]$SignTool = 'C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe'
)

$ErrorActionPreference = 'Stop'

# The programs that are signed: okur-sr.exe must be, for UI Access, and okur-lab-keys.exe, the lab's
# key sender with UI Access (never installed for users); the others for consistency.
$Programs = @('okur-sr.exe', 'okur-sr-no-uiaccess.exe', 'okur-ui.exe', 'okur-setup.exe', 'okur-lab-keys.exe')

$steps = @($NewCertificate, $Sign, $Verify, $TrustOnLab, $UndoLab, $UndoDevBox) | Where-Object { $_ }
if ($steps.Count -ne 1) {
    throw 'Choose one step: -NewCertificate, -Sign, -Verify, -TrustOnLab, -UndoLab or -UndoDevBox.'
}

function Assert-Thumbprint {
    if ($Thumbprint -notmatch '^[0-9A-Fa-f]{40}$') {
        throw 'Give -Thumbprint, the 40 hexadecimal digits -NewCertificate printed.'
    }
}

function Assert-Elevated {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'This step changes the machine''s Trusted Root store: run it elevated, on old-pc.'
    }
}

if ($NewCertificate) {
    if ($PSCmdlet.ShouldProcess('Cert:\CurrentUser\My', 'Create the Okur development code-signing certificate')) {
        # 1.3.6.1.5.5.7.3.3: code signing only. 2.5.29.19 empty: an end entity, not a CA, so
        # that even trusted as a root it cannot issue certificates.
        $cert = New-SelfSignedCertificate -Type Custom `
            -Subject 'CN=Okur Development (not for release)' `
            -FriendlyName 'Okur development code signing' `
            -KeyUsage DigitalSignature `
            -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}') `
            -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 `
            -KeyExportPolicy NonExportable `
            -NotAfter (Get-Date).AddMonths(12) `
            -CertStoreLocation 'Cert:\CurrentUser\My'
        # The public part only.
        Export-Certificate -Cert $cert -FilePath $CertificateFile | Out-Null
        Write-Output "Thumbprint: $($cert.Thumbprint)"
        Write-Output "Public certificate: $CertificateFile"
    }
}

if ($Sign) {
    Assert-Thumbprint
    $files = $Programs | ForEach-Object { Join-Path $BuildDir $_ } | Where-Object { Test-Path -LiteralPath $_ }
    if (-not $files) { throw "No programs to sign in $BuildDir." }
    foreach ($file in $files) {
        if ($PSCmdlet.ShouldProcess($file, 'Sign with the Okur development certificate')) {
            if (Test-Path -LiteralPath $SignTool) {
                & $SignTool sign /fd SHA256 /sha1 $Thumbprint /s My $file
                if ($LASTEXITCODE -ne 0) { throw "signtool failed on $file" }
            } else {
                $cert = Get-Item -LiteralPath "Cert:\CurrentUser\My\$Thumbprint"
                Set-AuthenticodeSignature -FilePath $file -Certificate $cert -HashAlgorithm SHA256 | Out-Null
            }
        }
    }
}

if ($Verify) {
    foreach ($name in $Programs) {
        $file = Join-Path $BuildDir $name
        if (Test-Path -LiteralPath $file) {
            $signature = Get-AuthenticodeSignature -LiteralPath $file
            Write-Output ('{0}: {1}' -f $name, $signature.Status)
        }
    }
}

if ($TrustOnLab) {
    Assert-Elevated
    if (-not (Test-Path -LiteralPath $CertificateFile)) { throw "No certificate at $CertificateFile." }
    if ($PSCmdlet.ShouldProcess('Cert:\LocalMachine\Root', "Trust $CertificateFile")) {
        Import-Certificate -FilePath $CertificateFile -CertStoreLocation 'Cert:\LocalMachine\Root' | Out-Null
        Write-Output 'Trusted. Check: Get-AuthenticodeSignature "C:\Program Files\Okur\okur-sr.exe" (Status: Valid).'
    }
}

if ($UndoLab) {
    Assert-Elevated
    Assert-Thumbprint
    if ($PSCmdlet.ShouldProcess("Cert:\LocalMachine\Root\$Thumbprint", 'Remove the Okur development root')) {
        Remove-Item -LiteralPath "Cert:\LocalMachine\Root\$Thumbprint"
    }
}

if ($UndoDevBox) {
    Assert-Thumbprint
    if ($PSCmdlet.ShouldProcess("Cert:\CurrentUser\My\$Thumbprint", 'Delete the Okur development certificate and its key')) {
        Remove-Item -LiteralPath "Cert:\CurrentUser\My\$Thumbprint" -DeleteKey
    }
}

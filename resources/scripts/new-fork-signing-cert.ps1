# Creates a self-signed code signing certificate for fork builds.
#
# Usage (from the repo root, in a normal PowerShell window):
#   ./resources/scripts/new-fork-signing-cert.ps1
#
# Fork builds request UIAccess (to manage elevated windows), which Windows
# only grants to executables signed by a certificate in the machine's
# Trusted Root store. This script:
#  - Prompts for a password to protect the private key.
#  - Exports the certificate with its private key to
#    `~/Downloads/glazewm-fork-signing.pfx`, which is used by CI via the
#    `FORK_SIGNING_PFX_BASE64` and `FORK_SIGNING_PFX_PASSWORD` secrets.
#  - Exports the public certificate to `resources/fork-signing.cer`, which
#    `update-fork.ps1` installs as trusted on each machine.
#
# Keep the `.pfx` file and its password private (e.g. in a password
# manager). Anything signed with it is trusted by machines that have the
# certificate installed.

$ErrorActionPreference = 'Stop'

$PfxPath = Join-Path $HOME 'Downloads\glazewm-fork-signing.pfx'
$CerPath = Join-Path $PSScriptRoot '..\fork-signing.cer'

$password = Read-Host -AsSecureString -Prompt 'Password for the private key'

$certificate = New-SelfSignedCertificate `
  -Type CodeSigningCert `
  -Subject 'CN=GlazeWM fork (vladkosarev)' `
  -KeyAlgorithm RSA `
  -KeyLength 3072 `
  -HashAlgorithm SHA256 `
  -KeyExportPolicy Exportable `
  -NotAfter (Get-Date).AddYears(10) `
  -CertStoreLocation 'Cert:\CurrentUser\My'

try {
  Export-PfxCertificate -Cert $certificate -FilePath $PfxPath `
    -Password $password | Out-Null

  Export-Certificate -Cert $certificate -FilePath $CerPath -Type CERT |
    Out-Null
}
finally {
  # The private key only needs to exist in the exported `.pfx` file.
  Remove-Item -Path "Cert:\CurrentUser\My\$($certificate.Thumbprint)"
}

Write-Output "Private key: $PfxPath"
Write-Output "Public certificate: $((Resolve-Path $CerPath).Path)"
Write-Output "Thumbprint: $($certificate.Thumbprint)"

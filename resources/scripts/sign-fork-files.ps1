# Signs files with the fork's code signing certificate (see
# `new-fork-signing-cert.ps1`). Used by the `fork-release` workflow.
#
# Usage: ./resources/scripts/sign-fork-files.ps1 <file> [<file>...]
#
# Reads the base64-encoded `.pfx` and its password from the
# `FORK_SIGNING_PFX_BASE64` and `FORK_SIGNING_PFX_PASSWORD` environment
# variables.

$ErrorActionPreference = 'Stop'

if (!$env:FORK_SIGNING_PFX_BASE64 -or !$env:FORK_SIGNING_PFX_PASSWORD) {
  throw 'Signing secrets are not set.'
}

if ($args.Count -eq 0) {
  throw 'No files to sign.'
}

$signtool = Get-ChildItem `
  'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe' |
  Sort-Object FullName -Descending |
  Select-Object -First 1

if (!$signtool) {
  throw 'signtool.exe not found.'
}

$pfxPath = Join-Path ([IO.Path]::GetTempPath()) "$([Guid]::NewGuid()).pfx"
[IO.File]::WriteAllBytes(
  $pfxPath,
  [Convert]::FromBase64String($env:FORK_SIGNING_PFX_BASE64)
)

try {
  & $signtool.FullName sign /fd SHA256 /f $pfxPath `
    /p $env:FORK_SIGNING_PFX_PASSWORD @args

  if ($LASTEXITCODE -ne 0) {
    throw "signtool failed with exit code $LASTEXITCODE."
  }
}
finally {
  Remove-Item $pfxPath -Force
}

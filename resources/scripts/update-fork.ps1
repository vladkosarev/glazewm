# Installs or updates the fork build of GlazeWM over an existing official
# installation.
#
# Usage (from any PowerShell window, no admin needed):
#   irm https://raw.githubusercontent.com/vladkosarev/glazewm/feat/scrolling-layout/resources/scripts/update-fork.ps1 | iex
#
# Downloads the latest `fork-latest` release, stops GlazeWM, replaces its
# executables (asking for admin via UAC), and restarts GlazeWM as the
# current user. The official executables are backed up once as `*.orig`.
#
# Fork builds request UIAccess to manage elevated windows, which requires
# their signing certificate to be trusted. The certificate published with
# the release is added to the machine's Trusted Root store on first install.
# Set `$env:GLAZEWM_FORK_FORCE = '1'` to reinstall when already up to date.

# Runs in a child scope, such that settings like `$ErrorActionPreference`
# don't leak into the caller's session when invoked via `iex`.
& {
  $ErrorActionPreference = 'Stop'
  $ProgressPreference = 'SilentlyContinue'

  $Repo = 'vladkosarev/glazewm'
  $ReleaseTag = 'fork-latest'
  $InstallDir = Join-Path $env:ProgramFiles 'glzr.io\GlazeWM'
  $VersionFile = Join-Path $InstallDir 'fork-commit.txt'
  $CertificateAsset = 'fork-signing.cer'

  # Release assets and the paths they replace, relative to the install dir.
  $Files = @(
    @{ Asset = 'glazewm.exe'; Path = 'glazewm.exe' },
    @{ Asset = 'glazewm-cli.exe'; Path = 'cli\glazewm.exe' },
    @{ Asset = 'glazewm-watcher.exe'; Path = 'glazewm-watcher.exe' }
  )

  function Get-ReleaseAssetUrl([string]$Asset) {
    "https://github.com/$Repo/releases/download/$ReleaseTag/$Asset"
  }

  # Stops GlazeWM and its watcher, waiting for them to exit.
  function Stop-GlazeWM() {
    $processNames = @('glazewm', 'glazewm-watcher')

    if (!(Get-Process -Name $processNames -ErrorAction SilentlyContinue)) {
      return
    }

    Write-Output 'Stopping GlazeWM...'
    $cli = Join-Path $InstallDir 'cli\glazewm.exe'
    & $cli command wm-exit | Out-Null

    # Give GlazeWM time to restore windows before forcing it to stop.
    for ($i = 0; $i -lt 20; $i++) {
      if (!(Get-Process -Name $processNames -ErrorAction SilentlyContinue)) {
        return
      }

      Start-Sleep -Milliseconds 500
    }

    Get-Process -Name $processNames -ErrorAction SilentlyContinue |
      Stop-Process -Force
  }

  # Trusts the signing certificate and copies the downloaded executables
  # into the install dir from an elevated process. Only this step runs as
  # admin, such that GlazeWM itself is later started as the current user.
  function Install-Files([string]$DownloadDir, [string]$Commit) {
    $certificatePath = Join-Path $DownloadDir $CertificateAsset
    $thumbprint = (
      New-Object Security.Cryptography.X509Certificates.X509Certificate2 `
        $certificatePath
    ).Thumbprint

    if (!(Test-Path "Cert:\LocalMachine\Root\$thumbprint")) {
      Write-Output (
        "This installs the fork's code signing certificate " +
        "('GlazeWM fork (vladkosarev)') into this machine's Trusted Root " +
        'store, such that its UIAccess build can manage admin windows. ' +
        'Programs signed with this certificate will be trusted on this ' +
        'machine. Decline the admin prompt to cancel.'
      )
    }

    $trustCommand = (
      "if (!(Test-Path 'Cert:\LocalMachine\Root\$thumbprint')) { " +
      "Import-Certificate -FilePath '$certificatePath' " +
      "-CertStoreLocation 'Cert:\LocalMachine\Root' | Out-Null }"
    )

    $copyCommands = foreach ($file in $Files) {
      $source = Join-Path $DownloadDir $file.Asset
      $target = Join-Path $InstallDir $file.Path

      "if (!(Test-Path '$target.orig')) { Copy-Item '$target' '$target.orig' }"
      "Copy-Item '$source' '$target' -Force"
    }

    $script = @(
      "`$ErrorActionPreference = 'Stop'"
      $trustCommand
      $copyCommands
      "Set-Content -Path '$VersionFile' -Value '$Commit'"
    ) -join "`n"

    $encodedScript =
      [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($script))

    $process = Start-Process powershell -Verb RunAs -Wait -PassThru `
      -ArgumentList '-NoProfile', '-EncodedCommand', $encodedScript

    if ($process.ExitCode -ne 0) {
      throw "Failed to replace GlazeWM executables (exit code $($process.ExitCode))."
    }
  }

  if (!(Test-Path (Join-Path $InstallDir 'glazewm.exe'))) {
    throw "GlazeWM is not installed at '$InstallDir'. Install the official release first: https://github.com/glzr-io/glazewm/releases"
  }

  $latestCommit =
    (Invoke-WebRequest (Get-ReleaseAssetUrl 'commit.txt') -UseBasicParsing).Content.Trim()

  $installedCommit = if (Test-Path $VersionFile) {
    (Get-Content $VersionFile -Raw).Trim()
  }

  if ($installedCommit -eq $latestCommit -and $env:GLAZEWM_FORK_FORCE -ne '1') {
    Write-Output "Already up to date ($($latestCommit.Substring(0, 7)))."
    return
  }

  $downloadDir = Join-Path $env:TEMP "glazewm-fork-$latestCommit"
  New-Item -ItemType Directory -Force -Path $downloadDir | Out-Null

  $assets = @($Files | ForEach-Object { $_.Asset }) + $CertificateAsset

  foreach ($asset in $assets) {
    Write-Output "Downloading $asset..."
    Invoke-WebRequest (Get-ReleaseAssetUrl $asset) `
      -OutFile (Join-Path $downloadDir $asset) -UseBasicParsing
  }

  Stop-GlazeWM

  # Always restart GlazeWM, even if replacing the executables fails (e.g. if
  # the admin prompt is declined).
  try {
    Write-Output 'Replacing executables (approve the admin prompt)...'
    Install-Files $downloadDir $latestCommit
    Write-Output "Installed fork build $($latestCommit.Substring(0, 7))."
  }
  finally {
    Write-Output 'Starting GlazeWM...'
    Start-Process (Join-Path $InstallDir 'glazewm.exe')
    Remove-Item -Recurse -Force $downloadDir -ErrorAction SilentlyContinue
  }
}

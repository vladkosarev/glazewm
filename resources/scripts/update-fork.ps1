# Installs or updates the fork build of GlazeWM.
#
# Usage (from any PowerShell window, no admin needed):
#   irm https://raw.githubusercontent.com/vladkosarev/glazewm/release/resources/scripts/update-fork.ps1 | iex
#
# Downloads the installer from the latest `fork-latest` release, stops
# GlazeWM, runs the installer (asking for admin via UAC), and restarts
# GlazeWM as the current user. Upgrades official installations as well.
# Set `$env:GLAZEWM_FORK_FORCE = '1'` to reinstall when already up to date.
#
# Fork builds request UIAccess to manage elevated windows, which requires
# their signing certificate to be trusted. The installer adds it to the
# machine's Trusted Root store, and removes it on uninstall.

# Runs in a child scope, such that settings like `$ErrorActionPreference`
# don't leak into the caller's session when invoked via `iex`.
& {
  $ErrorActionPreference = 'Stop'
  $ProgressPreference = 'SilentlyContinue'

  $Repo = 'vladkosarev/glazewm'
  $ReleaseTag = 'fork-latest'
  $MsiName = 'glazewm-fork-x64.msi'
  $InstallDir = Join-Path $env:ProgramFiles 'glzr.io\GlazeWM'
  $RegistryKey = 'HKLM:\SOFTWARE\glzr.io\GlazeWM'
  $CertificateName = 'GlazeWM fork (vladkosarev)'

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

    if (Test-Path $cli) {
      & $cli command wm-exit | Out-Null
    }

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

  # `Invoke-WebRequest` returns release assets as bytes, since GitHub
  # serves them as binary.
  $latestCommit = (New-Object Net.WebClient).DownloadString(
    (Get-ReleaseAssetUrl 'commit.txt')
  ).Trim()

  $installedCommit = Get-ItemProperty -Path $RegistryKey `
    -Name 'ForkCommit' -ErrorAction SilentlyContinue |
    Select-Object -ExpandProperty 'ForkCommit'

  if ($installedCommit -eq $latestCommit -and $env:GLAZEWM_FORK_FORCE -ne '1') {
    Write-Output "Already up to date ($($latestCommit.Substring(0, 7)))."
    return
  }

  $msiPath = Join-Path $env:TEMP "glazewm-fork-$latestCommit.msi"

  Write-Output "Downloading $MsiName..."
  Invoke-WebRequest (Get-ReleaseAssetUrl $MsiName) `
    -OutFile $msiPath -UseBasicParsing

  $isCertificateTrusted = Get-ChildItem 'Cert:\LocalMachine\Root' |
    Where-Object { $_.Subject -eq "CN=$CertificateName" }

  if (!$isCertificateTrusted) {
    Write-Output (
      "The installer adds the fork's code signing certificate " +
      "('$CertificateName') to this machine's Trusted Root store, such " +
      'that its UIAccess build can manage admin windows. Programs signed ' +
      'with this certificate will be trusted on this machine. Decline ' +
      'the admin prompt to cancel.'
    )
  }

  Stop-GlazeWM

  # Always restart GlazeWM, even if the installer fails (e.g. if the admin
  # prompt is declined).
  try {
    Write-Output 'Running installer (approve the admin prompt)...'

    $process = Start-Process msiexec -Wait -PassThru -ArgumentList `
      '/i', "`"$msiPath`"", '/passive', '/norestart'

    # Exit code 3010 indicates success, with a restart being required.
    if ($process.ExitCode -notin 0, 3010) {
      throw "Installer failed with exit code $($process.ExitCode)."
    }

    Write-Output "Installed fork build $($latestCommit.Substring(0, 7))."
  }
  finally {
    $exePath = Join-Path $InstallDir 'glazewm.exe'

    if (Test-Path $exePath) {
      Write-Output 'Starting GlazeWM...'
      Start-Process $exePath
    }

    Remove-Item $msiPath -Force -ErrorAction SilentlyContinue
  }
}

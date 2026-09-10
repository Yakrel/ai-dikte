# AI Dikte Windows Installer (PowerShell)
# Recommended usage:
#   irm https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.ps1 | iex

[CmdletBinding()]
param([string]$ArtifactDirectory)

$ErrorActionPreference = "Stop"

$BOLD   = "$([char]27)[1m"
$BLUE   = "$([char]27)[0;34m"
$GREEN  = "$([char]27)[0;32m"
$YELLOW = "$([char]27)[0;33m"
$RED    = "$([char]27)[0;31m"
$NC     = "$([char]27)[0m"

Write-Host "${BOLD}${BLUE}==>${NC} ${BOLD}AI Dikte Installer for Windows${NC}"

$installDir = Join-Path $env:LOCALAPPDATA "Programs\AI-Dikte"
$exePath = Join-Path $installDir "ai-dikte.exe"
$stage = Join-Path $env:TEMP ("ai-dikte-" + [guid]::NewGuid())

function Set-PreferredUserPath {
    param(
        [Parameter(Mandatory = $true)][string]$Preferred,
        [string[]]$Remove = @()
    )

    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    $removeSet = @($Preferred) + $Remove
    $parts = @()

    if (-not [string]::IsNullOrWhiteSpace($current)) {
        foreach ($part in ($current -split ';')) {
            $trimmed = $part.Trim()
            if ([string]::IsNullOrWhiteSpace($trimmed)) { continue }

            $shouldRemove = $false
            foreach ($candidate in $removeSet) {
                if ($trimmed.TrimEnd('\') -ieq $candidate.TrimEnd('\')) {
                    $shouldRemove = $true
                    break
                }
            }
            if (-not $shouldRemove) {
                $parts += $trimmed
            }
        }
    }

    $newPath = (@($Preferred) + $parts) -join ';'
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    $env:Path = "$Preferred;$env:Path"
}

function Start-DaemonProcess {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [string]$Arguments = "daemon"
    )

    Write-Host "${BOLD}${BLUE}==>${NC} Starting background hotkey listener..."
    $daemon = Start-Process -FilePath $Executable -ArgumentList $Arguments -PassThru
    Start-Sleep -Seconds 1
    $daemon.Refresh()
    if ($daemon.HasExited) { throw "Background listener exited during startup (code $($daemon.ExitCode))." }
    Write-Host "${GREEN}[OK]${NC} Background listener process is running."
}

function Stop-InstalledDaemon {
    param([Parameter(Mandatory = $true)][string[]]$ExecutablePaths)

    $processes = Get-CimInstance Win32_Process |
        Where-Object { $_.ExecutablePath -and $ExecutablePaths -contains $_.ExecutablePath }
    foreach ($process in $processes) {
        Stop-Process -Id $process.ProcessId -Force -ErrorAction SilentlyContinue
    }
    foreach ($process in $processes) {
        Wait-Process -Id $process.ProcessId -Timeout 10 -ErrorAction SilentlyContinue
    }
}

New-Item -ItemType Directory -Path $installDir -Force | Out-Null
New-Item -ItemType Directory -Path $stage | Out-Null

$assetName = 'ai-dikte-windows.exe'
$installedName = 'ai-dikte.exe'

try {
    if (-not $ArtifactDirectory) {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/Yakrel/ai-dikte/releases/tags/latest" `
            -Headers @{ "User-Agent" = "ai-dikte-installer"; "Accept" = "application/vnd.github+json" }
    }

    foreach ($name in @($assetName, ($assetName + '.sha256'))) {
        $destination = Join-Path $stage $name
        if ($ArtifactDirectory) {
            Copy-Item -LiteralPath (Join-Path $ArtifactDirectory $name) -Destination $destination
        } else {
            $assets = @($release.assets | Where-Object { $_.name -eq $name })
            if ($assets.Count -ne 1) { throw "Release must contain exactly one $name" }
            Invoke-WebRequest -Uri $assets[0].browser_download_url -OutFile $destination
        }
    }

    $download = Join-Path $stage $assetName
    $checksumText = (Get-Content -LiteralPath "$download.sha256" -Raw).Trim()
    $match = [regex]::Match($checksumText, '\A([a-fA-F0-9]{64})(?:\s|$)')
    if (-not $match.Success) { throw "Malformed checksum: $assetName" }
    if ((Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash -ine $match.Groups[1].Value) {
        throw "SHA-256 checksum mismatch: $assetName"
    }
    Unblock-File -LiteralPath $download

    # Verify self-test before replacing installation
    $proc = Start-Process -FilePath $download -ArgumentList '--self-test' -Wait -PassThru
    if ($proc.ExitCode -ne 0) { throw "Self-test failed: $assetName" }

    Stop-InstalledDaemon -ExecutablePaths @($exePath)

    $destination = Join-Path $installDir $installedName
    $backup = Join-Path $stage ($installedName + '.backup')
    if (Test-Path -LiteralPath $destination) {
        Copy-Item -LiteralPath $destination -Destination $backup
    }

    try {
        Move-Item -LiteralPath $download -Destination $destination -Force
    } catch {
        if (Test-Path -LiteralPath $backup) {
            Copy-Item -LiteralPath $backup -Destination $destination -Force
        }
        throw
    }
} catch {
    throw "AI Dikte installation failed: $($_.Exception.Message). No alternative installation was attempted."
} finally {
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
}

Set-PreferredUserPath -Preferred $installDir

Write-Host "${BOLD}${BLUE}==>${NC} Running initial configuration..."
$setupProc = Start-Process -FilePath $exePath -ArgumentList "setup" -Wait -PassThru -NoNewWindow
if ($setupProc.ExitCode -ne 0) {
    throw "AI Dikte setup failed with exit code $($setupProc.ExitCode)"
}

Write-Host "${BOLD}${BLUE}==>${NC} Running diagnostic checks..."
$doctorProc = Start-Process -FilePath $exePath -ArgumentList "check-config" -Wait -PassThru -NoNewWindow
if ($doctorProc.ExitCode -ne 0) {
    throw "AI Dikte diagnostics failed. Resolve the reported problem before starting dictation."
}

# Preserve an existing opt-in startup entry, now targeting the unified binary with daemon argument.
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
if (Get-ItemProperty -Path $runKey -Name 'AI-Dikte' -ErrorAction SilentlyContinue) {
    Set-ItemProperty -Path $runKey -Name 'AI-Dikte' -Value ('"' + $exePath + '" daemon')
}
Start-DaemonProcess -Executable $exePath -Arguments "daemon"

Write-Host ""
Write-Host "${GREEN}${BOLD}Setup complete!${NC} AI Dikte is running. Toggle recording with Win+Z."
Write-Host "Command: ${BOLD}ai-dikte${NC}"
exit 0

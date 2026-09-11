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
$commandDir = Join-Path $installDir "bin"
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

function Install-CommandLauncher {
    param([Parameter(Mandatory = $true)][string]$Directory)

    $bin = Join-Path $Directory 'bin'
    New-Item -ItemType Directory -Path $bin -Force | Out-Null
    # CMD waits for GUI executables when invoked from a batch file. PowerShell
    # must wait for this launcher rather than compete with the menu for stdin.
    $launcher = "@echo off`r`n`"%~dp0..\ai-dikte.exe`" %*`r`nexit /b %errorlevel%`r`n"
    [IO.File]::WriteAllText((Join-Path $bin 'ai-dikte.cmd'), $launcher, [Text.Encoding]::ASCII)
}

function Invoke-InstallerCommand {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string]$Arguments
    )

    # Own the process handle from creation: Windows PowerShell's Start-Process
    # can return a Process whose ExitCode stays null after WaitForExit().
    # Wait only for this process, never the daemon it may start.
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo.FileName = $FilePath
    $process.StartInfo.Arguments = $Arguments
    $process.StartInfo.UseShellExecute = $false
    try {
        if (-not $process.Start()) {
            throw "Cannot start $Arguments"
        }
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) {
            throw "$Arguments failed with exit code $($process.ExitCode)"
        }
    } finally {
        $process.Dispose()
    }
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
    Invoke-InstallerCommand -FilePath $download -Arguments '--self-test'

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

Install-CommandLauncher -Directory $installDir
Set-PreferredUserPath -Preferred $commandDir -Remove @($installDir)

# `setup` is intentionally one-shot. It validates and saves the key, starts the
# daemon and enables sign-in startup, then returns control to this installer.
Write-Host "${BOLD}${BLUE}==>${NC} Running initial configuration..."
Invoke-InstallerCommand -FilePath $exePath -Arguments 'setup'

Write-Host "${BOLD}${BLUE}==>${NC} Running diagnostic checks..."
Invoke-InstallerCommand -FilePath $exePath -Arguments 'check-config'

Write-Host ""
Write-Host "${GREEN}${BOLD}Setup complete!${NC} AI Dikte is running. Toggle recording with Win+Z."
Write-Host "Command: ${BOLD}ai-dikte${NC}"

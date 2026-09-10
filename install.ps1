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
$hostPath = Join-Path $installDir "ai-dikte-background.exe"
$programsFolder = [Environment]::GetFolderPath("Programs")
$startMenuLnk = Join-Path $programsFolder "AI Dikte.lnk"
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
        [string]$Arguments = ""
    )

    Write-Host "${BOLD}${BLUE}==>${NC} Starting background hotkey listener..."
    $daemon = Start-Process -FilePath $Executable -PassThru
    Start-Sleep -Seconds 1
    $daemon.Refresh()
    if ($daemon.HasExited) { throw "Background listener exited during startup (code $($daemon.ExitCode))." }
    Write-Host "${GREEN}[OK]${NC} Background listener process is running."
}

function Write-StartMenuShortcut {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [string]$Arguments = "daemon"
    )

    $wscript = New-Object -ComObject WScript.Shell
    $shortcut = $wscript.CreateShortcut($startMenuLnk)
    $shortcut.TargetPath = $Executable
    $shortcut.Arguments = $Arguments
    $shortcut.WorkingDirectory = Split-Path $Executable
    $shortcut.Description = "AI Dikte - Minimal Dictation using Gemini 3.5"
    $shortcut.IconLocation = "$Executable,0"
    $shortcut.Save()
    Write-Host "${BOLD}${BLUE}==>${NC} Created Start Menu shortcut: $startMenuLnk"
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
$files = @(
    @{Asset='ai-dikte-windows.exe'; Installed='ai-dikte.exe'},
    @{Asset='ai-dikte-background.exe'; Installed='ai-dikte-background.exe'}
)
try {
    if (-not $ArtifactDirectory) {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/Yakrel/ai-dikte/releases/tags/latest" `
            -Headers @{ "User-Agent" = "ai-dikte-installer"; "Accept" = "application/vnd.github+json" }
    }
    foreach ($file in $files) {
        foreach ($name in @($file.Asset, ($file.Asset + '.sha256'))) {
            $destination = Join-Path $stage $name
            if ($ArtifactDirectory) {
                Copy-Item -LiteralPath (Join-Path $ArtifactDirectory $name) -Destination $destination
            } else {
                $assets = @($release.assets | Where-Object { $_.name -eq $name })
                if ($assets.Count -ne 1) { throw "Release must contain exactly one $name" }
                Invoke-WebRequest -Uri $assets[0].browser_download_url -OutFile $destination
            }
        }
        $download = Join-Path $stage $file.Asset
        $checksumText = (Get-Content -LiteralPath "$download.sha256" -Raw).Trim()
        $match = [regex]::Match($checksumText, '\A([a-fA-F0-9]{64})(?:\s|$)')
        if (-not $match.Success) { throw "Malformed checksum: $($file.Asset)" }
        if ((Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash -ine $match.Groups[1].Value) {
            throw "SHA-256 checksum mismatch: $($file.Asset)"
        }
        Unblock-File -LiteralPath $download
    }
    # Verify the complete pair before stopping or replacing the existing installation.
    foreach ($file in $files) {
        $proc = Start-Process -FilePath (Join-Path $stage $file.Asset) -ArgumentList '--self-test' -Wait -PassThru
        if ($proc.ExitCode -ne 0) { throw "Self-test failed: $($file.Asset)" }
    }
    Stop-InstalledDaemon -ExecutablePaths @($exePath, $hostPath)
    $backups = @{}
    foreach ($file in $files) {
        $destination = Join-Path $installDir $file.Installed
        if (Test-Path -LiteralPath $destination) {
            $backup = Join-Path $stage ($file.Installed + '.backup')
            Copy-Item -LiteralPath $destination -Destination $backup
            $backups[$file.Installed] = $backup
        }
    }
    try {
        foreach ($file in $files) {
            Move-Item -LiteralPath (Join-Path $stage $file.Asset) -Destination (Join-Path $installDir $file.Installed) -Force
        }
    } catch {
        foreach ($file in $files) {
            $destination = Join-Path $installDir $file.Installed
            if ($backups.ContainsKey($file.Installed)) {
                Copy-Item -LiteralPath $backups[$file.Installed] -Destination $destination -Force
            } else {
                Remove-Item -LiteralPath $destination -Force -ErrorAction SilentlyContinue
            }
        }
        throw
    }
} catch {
    throw "AI Dikte installation failed: $($_.Exception.Message). No alternative installation was attempted."
} finally {
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
}
# The executable is now a real console application; the old batch wrapper is obsolete.
Remove-Item -LiteralPath (Join-Path $installDir 'ai-dikte.cmd') -Force -ErrorAction SilentlyContinue
Set-PreferredUserPath -Preferred $installDir
Write-StartMenuShortcut -Executable $exePath -Arguments ""

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

# Preserve an existing opt-in startup entry, now targeting the console-free host.
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
if (Get-ItemProperty -Path $runKey -Name 'AI-Dikte' -ErrorAction SilentlyContinue) {
    Set-ItemProperty -Path $runKey -Name 'AI-Dikte' -Value ('"' + $hostPath + '"')
}
Start-DaemonProcess -Executable $hostPath

Write-Host ""
Write-Host "${GREEN}${BOLD}Setup complete!${NC} AI Dikte is running. Sign-in startup can be enabled from the tray menu."
Write-Host "Command: ${BOLD}ai-dikte${NC}"
exit 0

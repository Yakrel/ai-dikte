# AI Dikte Windows Installer (PowerShell)
# Recommended usage:
#   irm https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.ps1 | iex

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$BOLD   = "$([char]27)[1m"
$BLUE   = "$([char]27)[0;34m"
$GREEN  = "$([char]27)[0;32m"
$YELLOW = "$([char]27)[0;33m"
$RED    = "$([char]27)[0;31m"
$NC     = "$([char]27)[0m"

Write-Host "${BOLD}${BLUE}==>${NC} ${BOLD}AI Dikte Installer for Windows${NC}"

$installDir = Join-Path $env:LOCALAPPDATA "Programs\AI-Dikte"
$fallbackDir = Join-Path $env:LOCALAPPDATA "ai-dikte"
$exePath = Join-Path $installDir "ai-dikte.exe"
$cmdLauncher = Join-Path $installDir "ai-dikte.cmd"
$runKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$appName = "AI-Dikte"
$programsFolder = [Environment]::GetFolderPath("Programs")
$startMenuLnk = Join-Path $programsFolder "AI Dikte.lnk"
$runtimeOverride = Join-Path $installDir "ai-dikte"
$downloadExe = Join-Path $env:TEMP "ai-dikte-windows-$PID.exe"
$downloadChecksum = "$downloadExe.sha256"


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

function Set-RegistryStartup {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [string]$Arguments = "daemon"
    )

    $cmd = "`"$Executable`" $Arguments"
    Set-ItemProperty -Path $runKey -Name $appName -Value $cmd -Force
    Write-Host "${BOLD}${BLUE}==>${NC} Configured Windows sign-in startup via Registry: $appName"
}

function Remove-LegacyStartupFiles {
    $startupFolder = [Environment]::GetFolderPath("Startup")
    Remove-Item -Path (Join-Path $startupFolder "ai-dikte-startup.vbs") -Force -ErrorAction SilentlyContinue
    Remove-Item -Path (Join-Path $startupFolder "ai-dikte-startup.cmd") -Force -ErrorAction SilentlyContinue
}

function Start-DaemonProcess {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [string]$Arguments = "daemon"
    )

    Write-Host "${BOLD}${BLUE}==>${NC} Starting background hotkey listener..."
    Start-Process -FilePath $Executable -ArgumentList $Arguments -WindowStyle Hidden | Out-Null
    Start-Sleep -Milliseconds 500
    Write-Host "${GREEN}[OK]${NC} Background listener started."
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
$installedStandalone = $false

try {
    Write-Host "${BOLD}${BLUE}==>${NC} Looking for the latest standalone Windows release..."
    $release = Invoke-RestMethod `
        -Uri "https://api.github.com/repos/Yakrel/ai-dikte/releases/tags/latest" `
        -Headers @{ "User-Agent" = "ai-dikte-installer"; "Accept" = "application/vnd.github+json" }

    $asset = $release.assets |
        Where-Object { $_.name -eq "ai-dikte-windows.exe" } |
        Select-Object -First 1
    $checksumAsset = $release.assets |
        Where-Object { $_.name -eq "ai-dikte-windows.exe.sha256" } |
        Select-Object -First 1
    if (-not $asset -or -not $checksumAsset) {
        throw "Latest release is missing the executable or its SHA-256 checksum"
    }

    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $downloadExe
    Invoke-WebRequest -Uri $checksumAsset.browser_download_url -OutFile $downloadChecksum

    $checksumText = (Get-Content -Path $downloadChecksum -Raw).Trim()
    $checksumMatch = [regex]::Match($checksumText, '\A([a-fA-F0-9]{64})(?:\s|$)')
    if (-not $checksumMatch.Success) {
        throw "Release checksum file is malformed"
    }
    $expectedHash = $checksumMatch.Groups[1].Value.ToLowerInvariant()
    $actualHash = (Get-FileHash -Path $downloadExe -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualHash -ne $expectedHash) {
        throw "Standalone executable SHA-256 checksum mismatch"
    }

    Unblock-File -Path $downloadExe
    Write-Host "${BOLD}${BLUE}==>${NC} Verifying standalone executable..."
    $verifyProc = Start-Process -FilePath $downloadExe -ArgumentList "--self-test" -Wait -PassThru -NoNewWindow
    if ($verifyProc.ExitCode -ne 0) {
        throw "Standalone executable self-test failed with exit code $($verifyProc.ExitCode)"
    }

    Stop-InstalledDaemon -ExecutablePaths @($exePath)
    Move-Item -Path $downloadExe -Destination $exePath -Force
    Remove-Item -Path $runtimeOverride -Force -ErrorAction SilentlyContinue
    $installedStandalone = $true
    Write-Host "${GREEN}[OK]${NC} Installed verified standalone executable: $exePath"
}
catch {
    $downloadError = $_.Exception.Message
    if (Test-Path $exePath) {
        Write-Host "${YELLOW}[WARN]${NC} Download failed; checking the existing installation: $downloadError"
        $checkProc = Start-Process -FilePath $exePath -ArgumentList "--self-test" -Wait -PassThru -NoNewWindow
        if ($checkProc.ExitCode -eq 0) {
            $installedStandalone = $true
            Write-Host "${GREEN}[OK]${NC} Keeping the existing verified standalone executable."
        }
    }
    if (-not $installedStandalone) {
        Write-Host "${YELLOW}[WARN]${NC} No usable standalone release is available: $downloadError"
        Write-Host "${YELLOW}[WARN]${NC} Falling back to the Python-based installer path."
    }
}
finally {
    Remove-Item -Path $downloadExe, $downloadChecksum -Force -ErrorAction SilentlyContinue
}

if ($installedStandalone) {
    ('@echo off' + "`r`n" + '"%LOCALAPPDATA%\Programs\AI-Dikte\ai-dikte.exe" %*') | Out-File -FilePath $cmdLauncher -Encoding ASCII -Force
    Set-PreferredUserPath -Preferred $installDir -Remove @($fallbackDir)
    Remove-LegacyStartupFiles
    Set-RegistryStartup -Executable $exePath -Arguments "daemon"
    Write-StartMenuShortcut -Executable $exePath -Arguments "daemon"

    Write-Host "${BOLD}${BLUE}==>${NC} Running initial configuration..."
    $setupProc = Start-Process -FilePath $exePath -ArgumentList "setup" -Wait -PassThru -NoNewWindow
    if ($setupProc.ExitCode -ne 0) {
        throw "AI Dikte setup failed with exit code $($setupProc.ExitCode)"
    }

    Write-Host "${BOLD}${BLUE}==>${NC} Running diagnostic checks..."
    $doctorProc = Start-Process -FilePath $exePath -ArgumentList "doctor" -Wait -PassThru -NoNewWindow
    if ($doctorProc.ExitCode -ne 0) {
        Write-Host "${YELLOW}[WARN]${NC} Doctor reported one or more incomplete checks. Review the output above."
    }

    Start-DaemonProcess -Executable $exePath -Arguments "daemon"

    Write-Host ""
    Write-Host "${GREEN}${BOLD}Setup complete!${NC} AI Dikte is running now and will start automatically when you sign in."
    Write-Host "Command: ${BOLD}ai-dikte${NC}"
    exit 0
}

# Python/source fallback for development or a temporary Release outage.
$pythonCmd = $null
if (Get-Command python -ErrorAction SilentlyContinue) {
    $pythonCmd = "python"
} elseif (Get-Command py -ErrorAction SilentlyContinue) {
    $pythonCmd = "py"
}

if (-not $pythonCmd) {
    Write-Host "${RED}[ERROR]${NC} No usable standalone release is available and Python was not found."
    Write-Host "Install Python 3.10+ and rerun this installer."
    exit 1
}

$pythonVersion = & $pythonCmd --version 2>&1
Write-Host "${BOLD}${BLUE}==>${NC} Found $pythonVersion"
& $pythonCmd -m pip install --quiet --upgrade pip
& $pythonCmd -m pip install --quiet -r "https://raw.githubusercontent.com/Yakrel/ai-dikte/main/requirements.txt"

New-Item -ItemType Directory -Path $fallbackDir -Force | Out-Null
$scriptPath = Join-Path $fallbackDir "ai-dikte"
$pyWrapperPath = Join-Path $fallbackDir "ai_dikte.py"
$baseUrl = "https://raw.githubusercontent.com/Yakrel/ai-dikte/main"
Invoke-WebRequest -Uri "$baseUrl/ai-dikte" -OutFile $scriptPath
Invoke-WebRequest -Uri "$baseUrl/ai_dikte.py" -OutFile $pyWrapperPath

$fallbackLauncher = Join-Path $fallbackDir "ai-dikte.cmd"
('@echo off' + "`r`n" + "$pythonCmd `"%LOCALAPPDATA%\ai-dikte\ai_dikte.py`" %*") | Out-File -FilePath $fallbackLauncher -Encoding ASCII -Force
Set-PreferredUserPath -Preferred $fallbackDir -Remove @($installDir)

$pythonExe = (& $pythonCmd -c "import sys; print(sys.executable)").Trim()
$pythonwExe = Join-Path (Split-Path $pythonExe) "pythonw.exe"
if (-not (Test-Path $pythonwExe)) {
    $pythonwExe = $pythonExe
}
Remove-LegacyStartupFiles
Set-RegistryStartup -Executable $pythonwExe -Arguments "`"$pyWrapperPath`" daemon"
Write-StartMenuShortcut -Executable $pythonwExe -Arguments "`"$pyWrapperPath`" daemon"
& $pythonCmd $pyWrapperPath setup
if ($LASTEXITCODE -ne 0) {
    throw "AI Dikte setup failed with exit code $LASTEXITCODE"
}

Write-Host "${BOLD}${BLUE}==>${NC} Running diagnostic checks..."
& $pythonCmd $pyWrapperPath doctor
if ($LASTEXITCODE -ne 0) {
    Write-Host "${YELLOW}[WARN]${NC} Doctor reported one or more incomplete checks. Review the output above."
}

Start-DaemonProcess -Executable $pythonwExe -Arguments "`"$pyWrapperPath`" daemon"

Write-Host ""
Write-Host "${GREEN}${BOLD}Setup complete!${NC} Python fallback installation is active, running now, and configured to start automatically at sign-in."

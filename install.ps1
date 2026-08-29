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
$startupFolder = [Environment]::GetFolderPath("Startup")
$startupVbs = Join-Path $startupFolder "ai-dikte-startup.vbs"
$legacyStartupCmd = Join-Path $startupFolder "ai-dikte-startup.cmd"

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

function Write-HiddenStartupLauncher {
    param([Parameter(Mandatory = $true)][string]$Executable)

    Remove-Item -Path $legacyStartupCmd -Force -ErrorAction SilentlyContinue

    $escapedExecutable = $Executable.Replace('"', '""')
    $vbs = @"
Set shell = CreateObject("WScript.Shell")
shell.Run Chr(34) & "$escapedExecutable" & Chr(34) & " daemon", 0, False
"@
    $vbs | Out-File -FilePath $startupVbs -Encoding Unicode -Force
    Write-Host "${BOLD}${BLUE}==>${NC} Created hidden Startup launcher: $startupVbs"
}

function Start-HiddenStartupLauncher {
    if (-not (Test-Path $startupVbs)) {
        throw "Startup launcher was not created: $startupVbs"
    }

    $wscript = Join-Path $env:WINDIR "System32\wscript.exe"
    Write-Host "${BOLD}${BLUE}==>${NC} Starting background hotkey listener..."
    Start-Process -FilePath $wscript -ArgumentList "`"$startupVbs`"" -WindowStyle Hidden | Out-Null
    Start-Sleep -Milliseconds 500
    Write-Host "${GREEN}[OK]${NC} Background listener start requested."
}

New-Item -ItemType Directory -Path $installDir -Force | Out-Null
$installedStandalone = $false

try {
    Write-Host "${BOLD}${BLUE}==>${NC} Looking for the latest standalone Windows release..."
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/Yakrel/ai-dikte/releases/latest" -Headers @{ "User-Agent" = "ai-dikte-installer"; "Accept" = "application/vnd.github+json" }

    $asset = $release.assets | Where-Object { $_.name -eq "ai-dikte-windows.exe" } | Select-Object -First 1
    if (-not $asset) {
        throw "Latest release does not contain ai-dikte-windows.exe"
    }

    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $exePath

    Write-Host "${BOLD}${BLUE}==>${NC} Verifying standalone executable..."
    & $exePath --self-test
    if ($LASTEXITCODE -ne 0) {
        throw "Standalone executable self-test failed with exit code $LASTEXITCODE"
    }

    $installedStandalone = $true
    Write-Host "${GREEN}[OK]${NC} Installed standalone executable: $exePath"
}
catch {
    Remove-Item -Path $exePath -Force -ErrorAction SilentlyContinue
    Write-Host "${YELLOW}[WARN]${NC} No usable standalone release is available: $($_.Exception.Message)"
    Write-Host "${YELLOW}[WARN]${NC} Falling back to the Python-based installer path."
}

if ($installedStandalone) {
    ('@echo off' + "`r`n" + '"%LOCALAPPDATA%\Programs\AI-Dikte\ai-dikte.exe" %*') | Out-File -FilePath $cmdLauncher -Encoding ASCII -Force
    Set-PreferredUserPath -Preferred $installDir -Remove @($fallbackDir)
    Write-HiddenStartupLauncher -Executable $exePath

    Write-Host "${BOLD}${BLUE}==>${NC} Running initial configuration..."
    & $exePath setup
    if ($LASTEXITCODE -ne 0) {
        throw "AI Dikte setup failed with exit code $LASTEXITCODE"
    }

    Write-Host "${BOLD}${BLUE}==>${NC} Running diagnostic checks..."
    & $exePath doctor
    if ($LASTEXITCODE -ne 0) {
        Write-Host "${YELLOW}[WARN]${NC} Doctor reported one or more incomplete checks. Review the output above."
    }

    Start-HiddenStartupLauncher

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
Remove-Item -Path $legacyStartupCmd -Force -ErrorAction SilentlyContinue

# Python fallback needs the wrapper/script arguments in Startup.
$escapedPython = $pythonwExe.Replace('"', '""')
$escapedWrapper = $pyWrapperPath.Replace('"', '""')
$vbs = @"
Set shell = CreateObject("WScript.Shell")
shell.Run Chr(34) & "$escapedPython" & Chr(34) & " " & Chr(34) & "$escapedWrapper" & Chr(34) & " daemon", 0, False
"@
$vbs | Out-File -FilePath $startupVbs -Encoding Unicode -Force

Write-Host "${BOLD}${BLUE}==>${NC} Running initial configuration..."
& $pythonCmd $pyWrapperPath setup
if ($LASTEXITCODE -ne 0) {
    throw "AI Dikte setup failed with exit code $LASTEXITCODE"
}

Write-Host "${BOLD}${BLUE}==>${NC} Running diagnostic checks..."
& $pythonCmd $pyWrapperPath doctor
if ($LASTEXITCODE -ne 0) {
    Write-Host "${YELLOW}[WARN]${NC} Doctor reported one or more incomplete checks. Review the output above."
}

Start-HiddenStartupLauncher

Write-Host ""
Write-Host "${GREEN}${BOLD}Setup complete!${NC} Python fallback installation is active, running now, and configured to start automatically at sign-in."

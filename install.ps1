# AI Dikte Windows One-Line / Standalone Installer (PowerShell)
# Usage in PowerShell:
#   irm https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.ps1 | iex
# Or running locally:
#   .\install.ps1

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$BOLD  = "$([char]27)[1m"
$BLUE  = "$([char]27)[0;34m"
$GREEN = "$([char]27)[0;32m"
$RED   = "$([char]27)[0;31m"
$NC    = "$([char]27)[0m"

Write-Host "${BOLD}${BLUE}==>${NC} ${BOLD}AI Dikte Installer for Windows${NC}"

# Check for Python
$pythonCmd = $null
if (Get-Command python -ErrorAction SilentlyContinue) {
    $pythonCmd = "python"
} elseif (Get-Command py -ErrorAction SilentlyContinue) {
    $pythonCmd = "py"
}

if (-not $pythonCmd) {
    Write-Host "${RED}[ERROR]${NC} Python is not found in PATH."
    Write-Host "Please install Python 3.10+ from https://www.python.org/downloads/ or via winget:"
    Write-Host "  ${BOLD}winget install Python.Python.3.12${NC}"
    Write-Host "Make sure to check 'Add Python to PATH' during installation."
    exit 1
}

$pythonVersion = & $pythonCmd --version 2>&1
Write-Host "${BOLD}${BLUE}==>${NC} Found $pythonVersion"

# Install required Python packages
Write-Host "${BOLD}${BLUE}==>${NC} Installing required Python packages (sounddevice, websockets, keyboard, pystray, pillow)..."
& $pythonCmd -m pip install --quiet --upgrade pip
& $pythonCmd -m pip install --quiet websockets sounddevice keyboard pystray pillow

# Target installation directories
$installDir = Join-Path $env:LOCALAPPDATA "ai-dikte"
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

$scriptPath = Join-Path $installDir "ai-dikte"
$pyWrapperPath = Join-Path $installDir "ai_dikte.py"

# Download or copy scripts
if (Test-Path (Join-Path $PSScriptRoot "ai-dikte")) {
    Copy-Item (Join-Path $PSScriptRoot "ai-dikte") $scriptPath -Force
    Copy-Item (Join-Path $PSScriptRoot "ai_dikte.py") $pyWrapperPath -Force
} else {
    Write-Host "${BOLD}${BLUE}==>${NC} Downloading latest AI Dikte from GitHub..."
    $baseUrl = "https://raw.githubusercontent.com/Yakrel/ai-dikte/main"
    Invoke-RestMethod "$baseUrl/ai-dikte" -OutFile $scriptPath
    Invoke-RestMethod "$baseUrl/ai_dikte.py" -OutFile $pyWrapperPath
}

# Create a batch launcher in installDir (ai-dikte.cmd)
$cmdLauncher = Join-Path $installDir "ai-dikte.cmd"
"@echo off`n$pythonCmd `"$pyWrapperPath`" %*" | Out-File -FilePath $cmdLauncher -Encoding ASCII

# Add to User PATH if not present
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    Write-Host "${BOLD}${BLUE}==>${NC} Adding $installDir to User PATH..."
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    $env:Path += ";$installDir"
}

# Setup Windows Startup launcher for background Daemon mode (silent pythonw.exe)
$startupFolder = [Environment]::GetFolderPath("Startup")
$vbsPath = Join-Path $startupFolder "ai-dikte-startup.vbs"

$pythonwCmd = "pythonw.exe"
$vbsContent = @"
Set WshShell = CreateObject("WScript.Shell")
WshShell.Run "$pythonwCmd ""$pyWrapperPath"" daemon", 0, False
"@

$vbsContent | Out-File -FilePath $vbsPath -Encoding ASCII
Write-Host "${BOLD}${BLUE}==>${NC} Created Windows Startup shortcut (silent background daemon): $vbsPath"

Write-Host ""
Write-Host "${GREEN}==>${NC} ${BOLD}AI Dikte installed successfully on Windows!${NC}"
Write-Host ""

# Run interactive setup
if ([Environment]::UserInteractive) {
    Write-Host "${BOLD}${BLUE}==>${NC} Running initial configuration..."
    & $pythonCmd $pyWrapperPath setup
    Write-Host ""
    Write-Host "${BOLD}${BLUE}==>${NC} Running diagnostic checks..."
    & $pythonCmd $pyWrapperPath doctor
}

Write-Host ""
Write-Host "${GREEN}${BOLD}Setup complete!${NC} AI Dikte will run automatically on Windows startup in the background."
Write-Host "To start the background listener now, run: ${BOLD}ai-dikte daemon${NC} (or click tray icon)."

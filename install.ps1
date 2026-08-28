# AI Dikte Windows Installer (PowerShell)
# Recommended usage:
#   irm https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.ps1 | iex

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$BOLD  = "$([char]27)[1m"
$BLUE  = "$([char]27)[0;34m"
$GREEN = "$([char]27)[0;32m"
$YELLOW = "$([char]27)[0;33m"
$RED   = "$([char]27)[0;31m"
$NC    = "$([char]27)[0m"

Write-Host "${BOLD}${BLUE}==>${NC} ${BOLD}AI Dikte Installer for Windows${NC}"

$installDir = Join-Path $env:LOCALAPPDATA "Programs\AI-Dikte"
$exePath = Join-Path $installDir "ai-dikte.exe"
$cmdLauncher = Join-Path $installDir "ai-dikte.cmd"
$startupFolder = [Environment]::GetFolderPath("Startup")
$startupCmd = Join-Path $startupFolder "ai-dikte-startup.cmd"

New-Item -ItemType Directory -Path $installDir -Force | Out-Null

$installedStandalone = $false

try {
    Write-Host "${BOLD}${BLUE}==>${NC} Looking for the latest standalone Windows release..."
    $release = Invoke-RestMethod \
        -Uri "https://api.github.com/repos/Yakrel/ai-dikte/releases/latest" \
        -Headers @{ "User-Agent" = "ai-dikte-installer" }

    $asset = $release.assets | Where-Object { $_.name -eq "ai-dikte-windows.exe" } | Select-Object -First 1
    if (-not $asset) {
        throw "Latest release does not contain ai-dikte-windows.exe"
    }

    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $exePath
    $installedStandalone = $true
    Write-Host "${GREEN}[OK]${NC} Installed standalone executable: $exePath"
}
catch {
    Write-Host "${YELLOW}[WARN]${NC} No usable standalone release is available yet."
    Write-Host "${YELLOW}[WARN]${NC} Falling back to the Python-based installer path."
}

if ($installedStandalone) {
    "@echo off`r`n`"$exePath`" %*" | Out-File -FilePath $cmdLauncher -Encoding ASCII

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$installDir*") {
        $newPath = if ([string]::IsNullOrWhiteSpace($userPath)) { $installDir } else { "$userPath;$installDir" }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:Path += ";$installDir"
        Write-Host "${BOLD}${BLUE}==>${NC} Added $installDir to User PATH."
    }

    "@echo off`r`nstart `"`" /min `"$exePath`" daemon" | Out-File -FilePath $startupCmd -Encoding ASCII
    Write-Host "${BOLD}${BLUE}==>${NC} Created Startup launcher: $startupCmd"

    Write-Host "${BOLD}${BLUE}==>${NC} Running initial configuration..."
    & $exePath setup
    if ($LASTEXITCODE -ne 0) { throw "AI Dikte setup failed with exit code $LASTEXITCODE" }

    Write-Host "${BOLD}${BLUE}==>${NC} Running diagnostic checks..."
    & $exePath doctor
    if ($LASTEXITCODE -ne 0) {
        Write-Host "${YELLOW}[WARN]${NC} Doctor reported one or more incomplete checks. Review the output above."
    }

    Write-Host ""
    Write-Host "${GREEN}${BOLD}Setup complete!${NC} AI Dikte will start automatically when you sign in."
    Write-Host "Command: ${BOLD}ai-dikte${NC}"
    exit 0
}

# Source/Python fallback for development and for repositories that do not have a release yet.
$pythonCmd = $null
if (Get-Command python -ErrorAction SilentlyContinue) {
    $pythonCmd = "python"
} elseif (Get-Command py -ErrorAction SilentlyContinue) {
    $pythonCmd = "py"
}

if (-not $pythonCmd) {
    Write-Host "${RED}[ERROR]${NC} No standalone release is available and Python was not found."
    Write-Host "Install Python 3.10+ and rerun this installer, or publish a tagged GitHub release."
    exit 1
}

$pythonVersion = & $pythonCmd --version 2>&1
Write-Host "${BOLD}${BLUE}==>${NC} Found $pythonVersion"
& $pythonCmd -m pip install --quiet --upgrade pip
& $pythonCmd -m pip install --quiet websockets sounddevice keyboard pystray pillow

$fallbackDir = Join-Path $env:LOCALAPPDATA "ai-dikte"
New-Item -ItemType Directory -Path $fallbackDir -Force | Out-Null
$scriptPath = Join-Path $fallbackDir "ai-dikte"
$pyWrapperPath = Join-Path $fallbackDir "ai_dikte.py"
$baseUrl = "https://raw.githubusercontent.com/Yakrel/ai-dikte/main"
Invoke-RestMethod "$baseUrl/ai-dikte" -OutFile $scriptPath
Invoke-RestMethod "$baseUrl/ai_dikte.py" -OutFile $pyWrapperPath

$fallbackLauncher = Join-Path $fallbackDir "ai-dikte.cmd"
"@echo off`r`n$pythonCmd `"$pyWrapperPath`" %*" | Out-File -FilePath $fallbackLauncher -Encoding ASCII

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$fallbackDir*") {
    $newPath = if ([string]::IsNullOrWhiteSpace($userPath)) { $fallbackDir } else { "$userPath;$fallbackDir" }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    $env:Path += ";$fallbackDir"
}

$pythonExe = & $pythonCmd -c "import sys; print(sys.executable)"
$pythonwExe = Join-Path (Split-Path $pythonExe) "pythonw.exe"
if (-not (Test-Path $pythonwExe)) {
    $pythonwExe = $pythonExe
}

"@echo off`r`nstart `"`" /min `"$pythonwExe`" `"$pyWrapperPath`" daemon" | Out-File -FilePath $startupCmd -Encoding ASCII

Write-Host "${BOLD}${BLUE}==>${NC} Running initial configuration..."
& $pythonCmd $pyWrapperPath setup
Write-Host "${BOLD}${BLUE}==>${NC} Running diagnostic checks..."
& $pythonCmd $pyWrapperPath doctor

Write-Host ""
Write-Host "${GREEN}${BOLD}Setup complete!${NC} Python fallback installation is active."

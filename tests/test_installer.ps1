# Regression: failed EXE download/checksum must never launch an old EXE or install Python.
$ErrorActionPreference = 'Stop'
$installer = Join-Path (Split-Path $PSScriptRoot) 'install.ps1'
$previousLocal = $env:LOCALAPPDATA
$previousTemp = $env:TEMP
$testDir = Join-Path ([IO.Path]::GetTempPath()) ('ai-dikte-test-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $testDir | Out-Null
try {
    $env:LOCALAPPDATA = $testDir
    $env:TEMP = $testDir
    $installedDir = Join-Path $testDir 'Programs\AI-Dikte'
    New-Item -ItemType Directory -Path $installedDir -Force | Out-Null
    $existingExe = Join-Path $installedDir 'ai-dikte.exe'
    Set-Content $existingExe 'existing-installation' -NoNewline
    $existingHost = Join-Path $installedDir 'ai-dikte-background.exe'
    Set-Content $existingHost 'existing-host' -NoNewline
    $payload = Join-Path $testDir 'payload'
    Set-Content $payload 'downloaded-test-file'
    $script:validHash = (Get-FileHash $payload -Algorithm SHA256).Hash
    foreach ($scenario in @('download', 'checksum', 'missing-host', 'host-checksum')) {
        $script:scenario = $scenario
        $script:launched = $false
        function Invoke-RestMethod {
            if ($script:scenario -eq 'download') { throw 'simulated download failure' }
            $assets = @(
                @{name='ai-dikte-windows.exe';browser_download_url='https://example.invalid/app'},
                @{name='ai-dikte-windows.exe.sha256';browser_download_url='https://example.invalid/hash'},
                @{name='ai-dikte-background.exe';browser_download_url='https://example.invalid/host'},
                @{name='ai-dikte-background.exe.sha256';browser_download_url='https://example.invalid/host/hash'}
            )
            if ($script:scenario -eq 'missing-host') { $assets = @($assets | Where-Object { $_.name -notlike '*background*' }) }
            return @{assets=$assets}
        }
        function Invoke-WebRequest {
            param($Uri, $OutFile)
            if ($Uri -like '*/hash') {
                $bad = $script:scenario -eq 'checksum' -or ($script:scenario -eq 'host-checksum' -and $Uri -like '*/host/hash')
                Set-Content $OutFile $(if ($bad) { '0' * 64 } else { $script:validHash })
            }
            else { Set-Content $OutFile 'downloaded-test-file' }
        }
        function Start-Process { $script:launched = $true; throw 'Must not launch anything after failed verification' }
        $failed = $false
        try { & $installer } catch { $failed = $true }
        if (-not $failed) { throw "$scenario failure was reported as success" }
        if ($script:launched) { throw "$scenario failure launched an executable" }
        if ((Get-Content $existingExe -Raw) -ne 'existing-installation') { throw 'Existing installation was replaced' }
        if ((Get-Content $existingHost -Raw) -ne 'existing-host') { throw 'Existing background host was replaced' }
        Write-Host "[OK] $scenario failure stops without fallback"
    }
} finally {
    $env:LOCALAPPDATA = $previousLocal
    $env:TEMP = $previousTemp
    Remove-Item $testDir -Recurse -Force
}

param([string]$BinaryPath)

# Regression: failed EXE download/checksum must never launch an old EXE or corrupt installation.
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
    $payload = Join-Path $testDir 'payload'
    Set-Content $payload 'downloaded-test-file'
    $script:validHash = (Get-FileHash $payload -Algorithm SHA256).Hash
    foreach ($scenario in @('download', 'checksum')) {
        $script:scenario = $scenario
        $script:launched = $false
        function Invoke-RestMethod {
            if ($script:scenario -eq 'download') { throw 'simulated download failure' }
            $assets = @(
                @{name='ai-dikte-windows.exe';browser_download_url='https://example.invalid/app'},
                @{name='ai-dikte-windows.exe.sha256';browser_download_url='https://example.invalid/hash'}
            )
            return @{assets=$assets}
        }
        function Invoke-WebRequest {
            param($Uri, $OutFile)
            if ($Uri -like '*/hash') {
                $bad = $script:scenario -eq 'checksum'
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
        Write-Host "[OK] $scenario failure stops without fallback"
    }

    Remove-Item Function:\Start-Process
    $tokens = $null
    $errors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile(
        $installer, [ref]$tokens, [ref]$errors)
    if ($errors.Count) { throw "Installer has syntax errors" }
    $command = $ast.Find({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
        $node.Name -eq 'Invoke-InstallerCommand'
    }, $true)
    . ([scriptblock]::Create($command.Extent.Text))
    $shell = (Get-Process -Id $PID).Path
    Invoke-InstallerCommand -FilePath $shell -Arguments '-NoProfile -Command "exit 0"'
    Write-Host '[OK] Immediate successful exit is reported as success'
    $child = $null
    $previousPidFile = $env:AI_DIKTE_TEST_PID_FILE
    try {
        $env:AI_DIKTE_TEST_PID_FILE = Join-Path $testDir 'child.pid'
        $code = @'
$shell = (Get-Process -Id $PID).Path
$child = Start-Process -FilePath $shell -ArgumentList '-NoProfile -Command "Start-Sleep -Seconds 30"' -PassThru -NoNewWindow
[IO.File]::WriteAllText($env:AI_DIKTE_TEST_PID_FILE, [string]$child.Id)
exit 0
'@
        $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($code))
        Invoke-InstallerCommand -FilePath $shell -Arguments "-NoProfile -EncodedCommand $encoded"
        $child = Get-Process -Id ([int][IO.File]::ReadAllText($env:AI_DIKTE_TEST_PID_FILE))
        if ($child.HasExited) { throw "Installer waited for the background descendant" }
        Write-Host '[OK] Setup returns while its background descendant remains running'

        $failed = $false
        try {
            Invoke-InstallerCommand -FilePath $shell -Arguments '-NoProfile -Command "exit 23"'
        } catch {
            if ($_.Exception.Message -notlike '*exit code 23*') { throw }
            $failed = $true
        }
        if (-not $failed) { throw 'Nonzero setup exit was reported as success' }
        Write-Host '[OK] Nonzero command exit is propagated'

        if ($BinaryPath) {
            $launcherFunction = $ast.Find({
                param($node)
                $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
                $node.Name -eq 'Install-CommandLauncher'
            }, $true)
            . ([scriptblock]::Create($launcherFunction.Extent.Text))
            $launcherDir = Join-Path $testDir 'installation with spaces'
            Install-CommandLauncher -Directory $launcherDir
            Copy-Item -LiteralPath $BinaryPath -Destination (Join-Path $launcherDir 'ai-dikte.exe')

            $menu = New-Object System.Diagnostics.Process
            $menu.StartInfo.FileName = $shell
            $menu.StartInfo.Arguments = '-NoProfile -Command "$global:LASTEXITCODE = 0; ai-dikte check-config; $code = $LASTEXITCODE; Write-Output SHELL_RETURNED; exit $code"'
            $menu.StartInfo.UseShellExecute = $false
            $menu.StartInfo.RedirectStandardInput = $true
            $menu.StartInfo.RedirectStandardOutput = $true
            $menu.StartInfo.RedirectStandardError = $true
            $menu.StartInfo.EnvironmentVariables['PATH'] = (Join-Path $launcherDir 'bin') + ';' + $env:PATH
            $menu.StartInfo.EnvironmentVariables['APPDATA'] = Join-Path $testDir 'unconfigured'
            try {
                if (-not $menu.Start()) { throw 'Cannot start launcher regression' }
                $stdout = $menu.StandardOutput.ReadToEndAsync()
                $stderr = $menu.StandardError.ReadToEndAsync()
                $menu.StandardInput.Close()
                if (-not $menu.WaitForExit(15000)) { throw 'Launcher did not return after configuration check' }
                $output = $stdout.GetAwaiter().GetResult()
                $errorOutput = $stderr.GetAwaiter().GetResult()
                if ($menu.ExitCode -ne 1) { throw "Launcher lost command failure exit code: $($menu.ExitCode). $output $errorOutput" }
                if (-not $output.Contains('SHELL_RETURNED')) {
                    throw "Shell did not resume after configuration check: $output $errorOutput"
                }
                Write-Host '[OK] Bare ai-dikte command waits for GUI execution and preserves its failure exit code'
            } finally {
                if (-not $menu.HasExited) {
                    & "$env:WINDIR\System32\taskkill.exe" /PID $menu.Id /T /F
                    $menu.WaitForExit()
                }
                $menu.Dispose()
            }
        }
    } finally {
        if ($child -and -not $child.HasExited) {
            $child.Kill()
            $child.WaitForExit()
        }
        if ($child) { $child.Dispose() }
        $env:AI_DIKTE_TEST_PID_FILE = $previousPidFile
    }
} finally {
    $env:LOCALAPPDATA = $previousLocal
    $env:TEMP = $previousTemp
    Remove-Item $testDir -Recurse -Force
}

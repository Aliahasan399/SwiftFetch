$ErrorActionPreference = 'Stop'
$script:verifyResults = @()

function Invoke-Check {
    param([string] $Name, [string] $Command, [string[]] $Arguments)

    Write-Host "`n==> $Name"
    $startedAt = Get-Date
    # Windows PowerShell treats native stderr (e.g. cargo progress) as an error
    # under 'Stop'. Temporarily relax it; exit code remains the source of truth.
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $Command @Arguments } finally { $ErrorActionPreference = $previous }
    if ($LASTEXITCODE -ne 0) {
        $script:verifyResults += [ordered]@{ name = $Name; command = $Command; arguments = $Arguments -join ' '; passed = $false; exit_code = $LASTEXITCODE; started_at = $startedAt.ToString('o') }
        throw "$Name failed with exit code $LASTEXITCODE."
    }
    $script:verifyResults += [ordered]@{ name = $Name; command = $Command; arguments = $Arguments -join ' '; passed = $true; exit_code = 0; started_at = $startedAt.ToString('o') }
}

function Write-VerifyReport {
    param([bool] $Success)
    $root = Split-Path -Parent $PSScriptRoot
    $out = Join-Path $root 'artifacts/verification'
    $null = New-Item -ItemType Directory -Force -Path $out
    $path = Join-Path $out "report-$(Get-Date -Format 'yyyyMMdd-HHmmss').json"
    [ordered]@{
        schema = 'swiftfetch.verification/1'; timestamp = (Get-Date).ToString('o');
        app_version = (Get-Content -LiteralPath (Join-Path $root 'package.json') -Raw | ConvertFrom-Json).version;
        success = $Success; checks = $script:verifyResults;
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $path -Encoding utf8
    Write-Host "Verification report: $path"
}

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    Invoke-Check 'HTTP fixture self-test' python @('-B', 'tools/fixtures/selftest.py')
    Invoke-Check 'Rust compilation' cargo @('check', '--locked', '--all-targets', '--manifest-path', 'src-tauri/Cargo.toml')
    Invoke-Check 'Rust tests' cargo @('test', '--locked', '--manifest-path', 'src-tauri/Cargo.toml')
    Invoke-Check 'Desktop UI build' npm.cmd @('run', 'build', '--', '--emptyOutDir', 'false')
    Invoke-Check 'Desktop UI regressions' node @('src/testing/verify.mjs')
    Invoke-Check 'Browser protocol and package tests' node @('--test', 'browser-extension/test/test_protocol.js', 'browser-extension/test/test_release.js')
    Invoke-Check 'Firefox background behavior tests' node @('--test', 'browser-extension/test/test_firefox_background.test.mjs')
    Write-Host "`nAll verification steps passed."
    Write-VerifyReport -Success $true
} catch {
    Write-VerifyReport -Success $false
    throw
} finally {
    Pop-Location
}

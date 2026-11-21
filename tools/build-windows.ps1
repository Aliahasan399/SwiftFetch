[CmdletBinding()]
param([string] $SigningConfig)

$ErrorActionPreference = 'Stop'

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    # A new frontend directory prevents stale assets from entering the bundle.
    $runId = [Guid]::NewGuid().ToString('N')
    $frontend = "artifacts/candidate-$runId/ui"
    $runDirectory = Split-Path -Parent $frontend
    $null = New-Item -ItemType Directory -Path $runDirectory
    $overlay = @{
        build = @{
            beforeBuildCommand = "npm run build -- --outDir $frontend"
            frontendDist = "../$frontend"
        }
    }
    $channel = @($env:SWIFTFETCH_UPDATE_ENDPOINT, $env:SWIFTFETCH_UPDATE_ARCHIVE, $env:SWIFTFETCH_UPDATE_PUBLIC_KEY)
    $configured = @($channel | Where-Object { $_ }).Count
    if ($configured -gt 0 -and $configured -ne 3) { throw 'Configure all three SWIFTFETCH_UPDATE_ENDPOINT, SWIFTFETCH_UPDATE_ARCHIVE and SWIFTFETCH_UPDATE_PUBLIC_KEY values.' }
    if ($configured -eq 3) {
        if (-not $env:TAURI_SIGNING_PRIVATE_KEY -and -not $env:TAURI_SIGNING_PRIVATE_KEY_PATH) { throw 'A configured release channel requires its Tauri update-signing key in the release environment.' }
        $overlay.bundle = @{ createUpdaterArtifacts = $true }
    }
    $overlayPath = Join-Path (Get-Location).Path "$runDirectory/tauri.json"
    $overlay | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $overlayPath -Encoding ascii

    $buildArguments = @('run', 'tauri', 'build', '--', '--ci', '--config', $overlayPath)
    if ($SigningConfig) { $buildArguments += @('--config', (Resolve-Path -LiteralPath $SigningConfig).Path) }
    $buildArguments += @('--', '--locked')
    # Windows PowerShell treats native stderr (e.g. npm Info lines) as errors
    # under 'Stop'. Temporarily relax it; $LASTEXITCODE is the source of truth.
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & npm.cmd @buildArguments } finally { $ErrorActionPreference = $previous }
    if ($LASTEXITCODE -ne 0) { throw "Tauri build failed with exit code $LASTEXITCODE." }

    $target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'src-tauri/target' }
    Write-Host "Windows bundles are in $target/release/bundle"
} finally {
    Pop-Location
}

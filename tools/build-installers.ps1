[CmdletBinding()]
param(
    [string] $SigningConfig,
    [string] $SourceDateEpoch,
    [switch] $InspectOnly
)

# Phase 2 installer tooling.
# Purpose: give the Windows MSI + NSIS build one auditable entry point that records the
# toolchain, lockfiles and artifact hashes, and that refuses to silently turn on the
# updater channel. It deliberately does NOT publish, tag, or sign with a production
# certificate. Signing remains opt-in through an external config supplied by the owner.
#
# Reproducibility is NOT claimed by this script. It records the inputs needed to
# ATTEMPT an independent rebuild; byte-identical output must still be proven by two
# real builds. Use -InspectOnly to audit current artifacts without rebuilding.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $evidenceDir = Join-Path $root 'artifacts/installer-builds'
    $null = New-Item -ItemType Directory -Force -Path $evidenceDir

    $required = @(
        'package.json', 'package-lock.json',
        'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock',
        'src-tauri/tauri.conf.json', 'tools/build-windows.ps1'
    )
    $missing = @($required | Where-Object { -not (Test-Path -LiteralPath $_ -PathType Leaf) })
    if ($missing.Count -gt 0) { throw "Missing required build inputs: $($missing -join ', ')" }

    # The update channel must be an explicit, owner-approved decision. build-windows.ps1
    # already fails on a partial channel; here we fail on ANY channel unless the owner
    # passes an explicit opt-in switch, so a routine build cannot activate updates.
    $channel = @($env:SWIFTFETCH_UPDATE_ENDPOINT, $env:SWIFTFETCH_UPDATE_ARCHIVE, $env:SWIFTFETCH_UPDATE_PUBLIC_KEY)
    $updaterConfigured = @($channel | Where-Object { $_ }).Count -gt 0
    if ($updaterConfigured) {
        throw 'Update-channel environment variables are set. Refusing to build without an explicit owner decision; unset SWIFTFETCH_UPDATE_* before running this script.'
    }

    $epoch = if ($SourceDateEpoch) { $SourceDateEpoch } elseif ($env:SOURCE_DATE_EPOCH) { $env:SOURCE_DATE_EPOCH } else { $null }
    if ($epoch) { $env:SOURCE_DATE_EPOCH = $epoch }

    $tool = [ordered]@{}
    $tool.rustc = (& rustc -V 2>$null).Trim()
    $tool.cargo = (& cargo -V 2>$null).Trim()
    $tool.node = (& node -v 2>$null).Trim()
    $tool.npm = (& npm -v 2>$null).Trim()

    $gitHead = $null
    if (Test-Path -LiteralPath '.git') {
        $gitHead = (& git rev-parse HEAD 2>$null).Trim()
    }

    $built = $false
    $buildCommand = $null
    if (-not $InspectOnly) {
        $buildArgs = @()
        if ($SigningConfig) { $buildArgs += @('-SigningConfig', (Resolve-Path -LiteralPath $SigningConfig).Path) }
        Write-Host 'Building Windows installers via tools/build-windows.ps1 (no publishing, no updater).'
        & (Join-Path $PSScriptRoot 'build-windows.ps1') @buildArgs
        $built = $true
        $buildCommand = 'tools/build-windows.ps1' + $(if ($SigningConfig) { " -SigningConfig $SigningConfig" } else { '' })
    }

    $artifacts = @()
    $candidates = @(
        @{ kind = 'msi'; path = 'src-tauri/target/release/bundle/msi/SwiftFetch_0.1.0_x64_en-US.msi' },
        @{ kind = 'nsis'; path = 'src-tauri/target/release/bundle/nsis/SwiftFetch_0.1.0_x64-setup.exe' }
    )
    foreach ($c in $candidates) {
        if (-not (Test-Path -LiteralPath $c.path -PathType Leaf)) {
            $artifacts += [ordered]@{ kind = $c.kind; path = $c.path; present = $false }
            continue
        }
        $item = Get-Item -LiteralPath $c.path
        $hash = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        $sig = Get-AuthenticodeSignature -LiteralPath $item.FullName
        $artifacts += [ordered]@{
            kind = $c.kind; path = $item.FullName; present = $true
            bytes = $item.Length; sha256 = $hash
            modified_utc = $item.LastWriteTimeUtc.ToString('o')
            product_version = $item.VersionInfo.ProductVersion
            authenticode = $sig.Status.ToString()
        }
    }

    $report = [ordered]@{
        schema = 'swiftfetch.installer-build/1'
        timestamp = (Get-Date).ToString('o')
        mode = if ($InspectOnly) { 'inspect-only' } else { 'build' }
        built = $built
        build_command = $buildCommand
        signing_config_supplied = [bool] $SigningConfig
        updater_env_configured = $updaterConfigured
        source_date_epoch = $epoch
        reproducible_build_claimed = $false
        git_head = $gitHead
        toolchain = $tool
        artifacts = $artifacts
    }
    $evidencePath = Join-Path $evidenceDir "build-$stamp.json"
    $report | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $evidencePath -Encoding utf8
    Write-Output "Installer build evidence: $evidencePath"
    $artifacts | Where-Object { $_.present } | ForEach-Object {
        Write-Output ("{0}: {1} bytes, sha256={2}, authenticode={3}" -f $_.kind, $_.bytes, $_.sha256, $_.authenticode)
    }
    if ($InspectOnly) { Write-Output 'INSPECT-ONLY: no build was run; existing artifacts were measured.' }
} finally {
    Pop-Location
}

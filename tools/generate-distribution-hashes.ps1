[CmdletBinding()]
param(
    [string] $Msi = 'src-tauri/target/release/bundle/msi/SwiftFetch_0.1.0_x64_en-US.msi',
    [string] $Nsis = 'src-tauri/target/release/bundle/nsis/SwiftFetch_0.1.0_x64-setup.exe',
    [string] $ExpectedPublisher
)

# Generates distribution hashes ONLY for Authenticode-valid, signed installers.
# Unsigned hashes are historical build evidence (see docs/release/INSTALLER_HASH_BASELINE.md)
# and must never be emitted as final distribution hashes.
# This script does not sign, publish, or create a release.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $artifacts = @()
    foreach ($entry in @(@{ kind = 'msi'; path = $Msi }, @{ kind = 'nsis'; path = $Nsis })) {
        if (-not (Test-Path -LiteralPath $entry.path -PathType Leaf)) { throw "Artifact missing: $($entry.path)" }
        $item = Get-Item -LiteralPath $entry.path
        $sig = Get-AuthenticodeSignature -LiteralPath $item.FullName
        if ($sig.Status -ne 'Valid') {
            throw "Refusing to generate distribution hashes: $($entry.kind) is $($sig.Status), not signed and valid. Unsigned hashes are historical evidence only."
        }
        if ($ExpectedPublisher -and $sig.SignerCertificate.Subject -notlike "*$ExpectedPublisher*") {
            throw "Publisher mismatch for $($entry.kind): $($sig.SignerCertificate.Subject)"
        }
        $artifacts += [ordered]@{
            kind = $entry.kind; path = $item.FullName; bytes = $item.Length
            sha256 = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            authenticode = $sig.Status.ToString(); signer_subject = $sig.SignerCertificate.Subject
        }
    }

    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $out = Join-Path $root "artifacts/distribution-hashes/hashes-$stamp"
    $null = New-Item -ItemType Directory -Force -Path $out
    $sums = Join-Path $out 'SHA256SUMS'
    $artifacts | ForEach-Object { "{0}  {1}" -f $_.sha256, (Split-Path -Leaf $_.path) } | Set-Content -LiteralPath $sums -Encoding ascii
    [ordered]@{
        schema = 'swiftfetch.distribution-hashes/1'
        timestamp = (Get-Date).ToString('o')
        signed = $true
        note = 'Generated only from Authenticode-valid artifacts. No publication performed.'
        artifacts = $artifacts
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $out 'distribution-hashes.json') -Encoding utf8
    Write-Output "Distribution hashes (signed artifacts only): $out"
    Get-Content -LiteralPath $sums
} finally { Pop-Location }

[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidatePattern('^[0-9a-fA-F]{40}$')]
    [string] $PublisherThumbprint,
    [string] $TargetDirectory = 'src-tauri/target/release',
    [string] $OutputDirectory = 'artifacts/release'
)

$ErrorActionPreference = 'Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    $config = Get-Content -LiteralPath 'src-tauri/tauri.conf.json' -Raw | ConvertFrom-Json
    $package = Get-Content -LiteralPath 'package.json' -Raw | ConvertFrom-Json
    $version = $config.version
    if ($version -notmatch '^\d+\.\d+\.\d+$' -or $package.version -ne $version) {
        throw 'The npm and Tauri versions must match and use three numeric components.'
    }
    $installer = "SwiftFetch_${version}_x64-setup.exe"
    $msi = "SwiftFetch_${version}_x64_en-US.msi"
    $appPath = Join-Path $TargetDirectory 'swiftfetch.exe'
    $payload = @(
        (Join-Path $TargetDirectory "bundle/nsis/$installer"),
        (Join-Path $TargetDirectory "bundle/msi/$msi")
    )
    foreach ($path in @($appPath) + $payload) {
        $file = Get-Item -LiteralPath $path
        $signature = Get-AuthenticodeSignature -FilePath $file.FullName
        if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Thumbprint -ne $PublisherThumbprint) {
            throw "$($file.Name) must have a valid Authenticode signature from the selected publisher. Found $($signature.Status)."
        }
    }

    $releasePath = Join-Path $OutputDirectory "SwiftFetch-$version-windows-x64"
    if (Test-Path -LiteralPath $releasePath) { throw "The release directory already exists at $releasePath." }
    $null = New-Item -ItemType Directory -Path $releasePath
    $files = foreach ($path in $payload) {
        $file = Get-Item -LiteralPath $path
        $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        $target = Join-Path $releasePath $file.Name
        Copy-Item -LiteralPath $file.FullName -Destination $target
        if ((Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash.ToLowerInvariant() -ne $hash) {
            throw "$($file.Name) changed during staging. Discard that release folder."
        }
        [ordered]@{ name = $file.Name; size = $file.Length; sha256 = $hash }
    }
    $sums = foreach ($file in $files) { "$($file.sha256)  $($file.name)" }
    Set-Content -LiteralPath (Join-Path $releasePath 'SHA256SUMS') -Value $sums -Encoding ascii
    $manifest = [ordered]@{
        product = $config.identifier
        version = $version
        platform = 'windows-x64'
        publisher_thumbprint = $PublisherThumbprint.ToUpperInvariant()
        artifacts = $files
    }
    $manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $releasePath 'release-manifest.json') -Encoding ascii
    Write-Host "Signed Windows installers staged in $releasePath"
} finally {
    Pop-Location
}

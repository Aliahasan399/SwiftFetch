[CmdletBinding()]
param([string] $OutputDirectory = 'artifacts/source')

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'archive.ps1')
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $version = (Get-Content -LiteralPath 'package.json' -Raw | ConvertFrom-Json).version
    $run = "$version-$([Guid]::NewGuid().ToString('N'))"
    $output = Join-Path $OutputDirectory $run
    $source = Join-Path $output 'SwiftFetch-source'
    $null = New-Item -ItemType Directory -Path $source
    # Use a positive allowlist so databases, personal downloads, credentials,
    # previous bundles and adjacent backups cannot enter a source package.
    $files = @(
        '.gitignore', 'package.json', 'package-lock.json', 'index.html',
        'tsconfig.json', 'tsconfig.node.json', 'vite.config.ts',
        'README.md', 'ARCHITECTURE.md', 'CONTRACT.md', 'COMPLETION.md', 'CHANGELOG.md', 'RELEASE.md', 'RELEASE-READINESS.md', 'VERIFICATION.md',
        'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock', 'src-tauri/build.rs',
        'src-tauri/tauri.conf.json', 'src-tauri/.gitignore',
        'browser-extension/package.json', 'browser-extension/README.md', 'browser-extension/SETUP.md'
    )
    $folders = @(
        '.github', 'docs', 'src', 'public', 'tools', 'src-tauri/src', 'src-tauri/tests',
        'src-tauri/icons', 'src-tauri/capabilities',
        'browser-extension/chrome', 'browser-extension/firefox',
        'browser-extension/shared', 'browser-extension/native-host',
        'browser-extension/tools', 'browser-extension/test'
    )
    foreach ($folder in $folders) {
        if (-not (Test-Path -LiteralPath $folder)) { continue }
        $files += Get-ChildItem -LiteralPath $folder -Recurse -File | ForEach-Object {
            $_.FullName.Substring($root.Length + 1).Replace('\', '/')
        }
    }
    foreach ($relative in ($files | Sort-Object -Unique)) {
        if ($relative -match '\.bak-|(^|/)(__pycache__|node_modules|target|dist)/|\.(sqlite(-shm|-wal)?|part|key|pfx|p12|log)$') { continue }
        if ($relative -match '(FRONTEND_NOTES|INTEGRATION-NOTES)\.md$') { continue }
        $target = Join-Path $source $relative
        $null = New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target)
        Copy-Item -LiteralPath $relative -Destination $target -ErrorAction Stop
    }
    $zip = Join-Path $output "SwiftFetch-$version-source.zip"
    New-ProjectArchive -SourceDirectory $source -DestinationPath $zip -IncludeRoot
    $hash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS') -Value "$hash  $(Split-Path -Leaf $zip)" -Encoding ascii
    Write-Host "Source snapshot: $zip"
    Write-Host 'This command prepares a private snapshot. The release owner must approve licensing and distribution.'
} finally {
    Pop-Location
}

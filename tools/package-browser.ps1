$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'archive.ps1')
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    $directory = (& node browser-extension/tools/package.mjs | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Browser packaging failed.' }
    $version = (Get-Content -LiteralPath 'browser-extension/package.json' -Raw | ConvertFrom-Json).version
    $sums = foreach ($browser in @('chromium', 'firefox')) {
        $archive = "SwiftFetch-$browser-$version-preview.zip"
        $path = Join-Path $directory $archive
        New-ProjectArchive -SourceDirectory (Join-Path $directory $browser) -DestinationPath $path
        $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $archive"
    }
    Set-Content -LiteralPath (Join-Path $directory 'SHA256SUMS') -Value $sums -Encoding ascii
    Write-Host "Browser preview packages: $directory"
} finally { Pop-Location }

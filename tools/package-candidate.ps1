[CmdletBinding()]
param([string] $BrowserPackageDirectory, [string] $NoticesDirectory)

$ErrorActionPreference = 'Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    $version = (Get-Content -LiteralPath 'package.json' -Raw | ConvertFrom-Json).version
    $output = "artifacts/SwiftFetch-$version-candidate-$([Guid]::NewGuid().ToString('N'))"
    $null = New-Item -ItemType Directory -Path $output
    $paths = @(
        'src-tauri/target/release/swiftfetch.exe',
        "src-tauri/target/release/bundle/nsis/SwiftFetch_${version}_x64-setup.exe",
        "src-tauri/target/release/bundle/msi/SwiftFetch_${version}_x64_en-US.msi",
        'README.md', 'ARCHITECTURE.md', 'COMPLETION.md', 'CHANGELOG.md', 'RELEASE.md', 'RELEASE-READINESS.md', 'VERIFICATION.md'
    )
    if ($BrowserPackageDirectory) {
        $paths += Get-ChildItem -LiteralPath $BrowserPackageDirectory -Filter '*.zip' -File | ForEach-Object FullName
    }
    $documentationRoot=(Get-Item -LiteralPath 'docs').FullName
    $paths += Get-ChildItem -LiteralPath $documentationRoot -Recurse -File | Where-Object { $_.Name -notmatch '\.bak-' } | ForEach-Object FullName
    $noticesRoot=$null
    if($NoticesDirectory){
        $noticesRoot=(Get-Item -LiteralPath $NoticesDirectory).FullName
        $paths += Get-ChildItem -LiteralPath $noticesRoot -Recurse -File | ForEach-Object FullName
    }
    $files = foreach ($path in $paths) {
        $file = Get-Item -LiteralPath $path
        $relative=$file.Name
        if($file.FullName.StartsWith($documentationRoot+'\',[StringComparison]::OrdinalIgnoreCase)){$relative='docs/'+$file.FullName.Substring($documentationRoot.Length+1).Replace('\','/')}
        if($noticesRoot -and $file.FullName.StartsWith($noticesRoot+'\',[StringComparison]::OrdinalIgnoreCase)){$relative='notices/'+$file.FullName.Substring($noticesRoot.Length+1).Replace('\','/')}
        $target = Join-Path $output $relative
        $null=New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target)
        Copy-Item -LiteralPath $file.FullName -Destination $target -ErrorAction Stop
        $signature = if ($file.Extension -in @('.exe', '.msi')) {
            (Get-AuthenticodeSignature -FilePath $target).Status.ToString()
        } else { $null }
        [ordered]@{
            name = $relative
            size = $file.Length
            sha256 = (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash.ToLowerInvariant()
            authenticode = $signature
        }
    }
    [ordered]@{
        product = 'SwiftFetch'
        version = $version
        kind = 'private-release-candidate'
        ready_for_public_release = $false
        release_requirements = 'See RELEASE-READINESS.md'
        artifacts = $files
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $output 'candidate-manifest.json') -Encoding ascii
    $sums = foreach ($file in $files) { "$($file.sha256)  $($file.name)" }
    Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS') -Value $sums -Encoding ascii
    Write-Output ([IO.Path]::GetFullPath((Join-Path (Get-Location) $output)))
} finally { Pop-Location }

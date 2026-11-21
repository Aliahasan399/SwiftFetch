[CmdletBinding()]
param(
    [Parameter(Mandatory)] [uri] $ArtifactBaseUrl,
    [Parameter(Mandatory)] [string] $Installer,
    [string] $OutputDirectory = 'artifacts/update-channel',
    [string] $Notes = 'SwiftFetch Windows update'
)
$ErrorActionPreference = 'Stop'
if ($ArtifactBaseUrl.Scheme -ne 'https' -or $ArtifactBaseUrl.UserInfo) { throw 'The artifact base URL must use HTTPS without credentials.' }
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    $version = (Get-Content -LiteralPath 'src-tauri/tauri.conf.json' -Raw | ConvertFrom-Json).version
    $file = Get-Item -LiteralPath $Installer
    $signaturePath = "$($file.FullName).sig"
    $signature = (Get-Content -LiteralPath $signaturePath -Raw).Trim()
    if (-not $signature) { throw 'Build and sign the updater artifact before creating a manifest.' }
    $decoded = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($signature))
    $trusted = @($decoded -split "`r?`n" | Where-Object { $_.StartsWith('trusted comment: ') })
    $signedVersion = @($trusted -split "`t" | Where-Object { $_.StartsWith('version:') } | ForEach-Object { $_.Substring(8) })
    if ($trusted.Count -ne 1 -or $signedVersion.Count -ne 1 -or $signedVersion[0] -cne $version) {
        throw 'Use the current Tauri signer with --app-version so the signature binds the release version.'
    }
    $uri = [uri]::new($ArtifactBaseUrl.AbsoluteUri.TrimEnd('/') + '/' + $file.Name)
    $payload = [ordered]@{
        version = $version
        notes = $Notes
        pub_date = [DateTime]::UtcNow.ToString('o')
        platforms = @{ 'windows-x86_64' = @{ url = $uri.AbsoluteUri; signature = $signature } }
    }
    $versionDirectory = [IO.Path]::GetFullPath((Join-Path $OutputDirectory $version))
    if (Test-Path -LiteralPath $versionDirectory) { throw 'An immutable version directory already exists; choose another output location.' }
    $null = New-Item -ItemType Directory -Path $versionDirectory
    $json = $payload | ConvertTo-Json -Depth 5
    [IO.File]::WriteAllText((Join-Path $versionDirectory 'latest.json'), $json, [Text.UTF8Encoding]::new($false))
    Copy-Item -LiteralPath $file.FullName -Destination $versionDirectory
    Copy-Item -LiteralPath $signaturePath -Destination $versionDirectory
    Write-Output "Prepared immutable manifest: $versionDirectory/latest.json"
    Write-Output 'Publish this version directory and update the latest manifest only after release approval.'
} finally { Pop-Location }

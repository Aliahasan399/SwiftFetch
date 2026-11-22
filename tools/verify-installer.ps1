[CmdletBinding()]
param(
    [string] $Msi = 'src-tauri/target/release/bundle/msi/SwiftFetch_0.1.0_x64_en-US.msi',
    [string] $Nsis = 'src-tauri/target/release/bundle/nsis/SwiftFetch_0.1.0_x64-setup.exe',
    [string] $ExpectedMsiSha256,
    [string] $ExpectedNsisSha256,
    [switch] $SkipNsis
)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    $installer = (Get-Item -LiteralPath $Msi).FullName
    $folder = Join-Path (Get-Location) ("artifacts/installer-" + [Guid]::NewGuid().ToString('N'))
    $null = New-Item -ItemType Directory -Path $folder

    $msiHash = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash.ToLowerInvariant()
    $msiSig = (Get-AuthenticodeSignature -LiteralPath $installer).Status.ToString()
    if ($ExpectedMsiSha256 -and $msiHash -ne $ExpectedMsiSha256.ToLowerInvariant()) {
        throw "MSI hash mismatch: measured $msiHash, expected $ExpectedMsiSha256"
    }

    # Administrative extraction validates the MSI payload without registering
    # or replacing an installed application in this Windows user's profile.
    $arguments = @('/a', ('"' + $installer + '"'), '/qn', ('TARGETDIR="' + $folder + '"'), '/l*v', ('"' + $folder + '/msi.log"'))
    $process = Start-Process -FilePath "$env:windir/System32/msiexec.exe" -ArgumentList $arguments -PassThru -Wait
    if ($process.ExitCode -notin @(0,3010)) { throw "MSI extraction failed ($($process.ExitCode)); inspect $folder/msi.log" }
    $app = Get-ChildItem -LiteralPath $folder -Recurse -File -Filter swiftfetch.exe | Select-Object -First 1
    if (-not $app) { throw 'The MSI did not contain the application executable.' }
    $version = (Get-Content -LiteralPath 'src-tauri/tauri.conf.json' -Raw | ConvertFrom-Json).version
    if ($app.VersionInfo.ProductVersion -notlike "$version*") { throw 'The packaged application version does not match the release.' }
    & node tools/smoke-desktop.mjs $app.FullName
    if ($LASTEXITCODE -ne 0) { throw 'The application extracted from the MSI failed its runtime acceptance checks.' }

    # NSIS is verified statically here (metadata + hash). Running its installer/uninstaller
    # would change this machine, so real install/upgrade/uninstall remains a clean-VM task.
    $nsisInfo = [ordered]@{ verified = $false; reason = 'skipped' }
    if (-not $SkipNsis) {
        if (-not (Test-Path -LiteralPath $Nsis -PathType Leaf)) { throw "NSIS installer not found: $Nsis" }
        $nsisFile = Get-Item -LiteralPath $Nsis
        $nsisHash = (Get-FileHash -LiteralPath $nsisFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($ExpectedNsisSha256 -and $nsisHash -ne $ExpectedNsisSha256.ToLowerInvariant()) {
            throw "NSIS hash mismatch: measured $nsisHash, expected $ExpectedNsisSha256"
        }
        $nsisProductVersion = $nsisFile.VersionInfo.ProductVersion
        if ($nsisProductVersion -notlike "$version*") { throw 'NSIS installer version does not match the release.' }
        $nsisInfo = [ordered]@{
            verified = $true; path = $nsisFile.FullName; bytes = $nsisFile.Length
            sha256 = $nsisHash; product_version = $nsisProductVersion
            authenticode = (Get-AuthenticodeSignature -LiteralPath $nsisFile.FullName).Status.ToString()
            clean_machine_execution = $false
        }
    }

    [ordered]@{
        kind='administrative-extraction-and-runtime'; clean_machine=$false; msi=$installer
        msi_sha256=$msiHash; msi_authenticode=$msiSig
        executable=$app.FullName; nsis=$nsisInfo; passed=$true
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath "$folder/result.json" -Encoding ascii
    Write-Output "MSI payload and runtime verified: $folder"
    Write-Output "MSI sha256=$msiHash authenticode=$msiSig"
    if ($nsisInfo.verified) { Write-Output "NSIS sha256=$($nsisInfo.sha256) authenticode=$($nsisInfo.authenticode) (static only)" }
} finally { Pop-Location }

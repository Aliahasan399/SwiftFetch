[CmdletBinding()]
param([string] $Msi = 'src-tauri/target/release/bundle/msi/SwiftFetch_0.1.0_x64_en-US.msi')
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    $installer = (Get-Item -LiteralPath $Msi).FullName
    $folder = Join-Path (Get-Location) ("artifacts/installer-" + [Guid]::NewGuid().ToString('N'))
    $null = New-Item -ItemType Directory -Path $folder
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
    [ordered]@{ kind='administrative-extraction-and-runtime'; clean_machine=$false; msi=$installer; executable=$app.FullName; passed=$true } |
        ConvertTo-Json | Set-Content -LiteralPath "$folder/result.json" -Encoding ascii
    Write-Output "MSI payload and runtime verified: $folder"
} finally { Pop-Location }

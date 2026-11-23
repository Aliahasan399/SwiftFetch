[CmdletBinding(SupportsShouldProcess)]
param(
    [Parameter(Mandatory)] [ValidateSet('Chrome','Edge','Firefox')] [string] $Browser,
    [Parameter(Mandatory)] [string] $ExecutablePath,
    [string] $ExtensionId,
    [string] $ManifestDirectory = (Join-Path $env:LOCALAPPDATA 'SwiftFetch/NativeHost'),
    [switch] $AllowUnsignedPreview
)

$ErrorActionPreference = 'Stop'
$fullManifestDirectory = [IO.Path]::GetFullPath($ManifestDirectory)
$tempRoot = [IO.Path]::GetFullPath($env:TEMP).TrimEnd('\') + '\'
if ($fullManifestDirectory.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -or
    $fullManifestDirectory -match '(?i)scratch|hermes|\\temp\\') {
    throw 'Refusing to register the native host in a temporary or scratch directory. Use the stable per-user default under %LOCALAPPDATA%\SwiftFetch\NativeHost.'
}
$exe = Get-Item -LiteralPath $ExecutablePath
if ($exe.FullName.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to register a native host executable from a temporary directory. Install the development build to a stable path first.'
}
if ($exe.PSIsContainer -or $exe.Extension -ne '.exe') { throw 'Select the installed SwiftFetch executable.' }
$signature = Get-AuthenticodeSignature -FilePath $exe.FullName
if (-not $AllowUnsignedPreview -and $signature.Status -ne 'Valid') {
    throw 'Select a signed SwiftFetch build. Local candidate testing requires -AllowUnsignedPreview.'
}

$manifest = [ordered]@{
    name = 'com.swiftfetch.app'
    description = 'SwiftFetch download handoff'
    path = $exe.FullName
    type = 'stdio'
}
$key = switch ($Browser) {
    'Chrome' { 'HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.swiftfetch.app' }
    'Edge' { 'HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\com.swiftfetch.app' }
    'Firefox' { 'HKCU:\Software\Mozilla\NativeMessagingHosts\com.swiftfetch.app' }
}
if ($Browser -eq 'Firefox') {
    $manifest.allowed_extensions = @('swiftfetch@localhost')
} else {
    if ($ExtensionId -cnotmatch '^[a-p]{32}$') { throw 'Provide the installed browser extension ID (32 lowercase a-p letters).' }
    $manifest.allowed_origins = @("chrome-extension://$ExtensionId/")
}
$directory = Join-Path $ManifestDirectory $Browser
$path = [IO.Path]::GetFullPath((Join-Path $directory 'com.swiftfetch.app.json'))
$json = $manifest | ConvertTo-Json -Depth 4
if (Test-Path -LiteralPath $key) {
    $registered = (Get-Item -LiteralPath $key).GetValue('')
    if ($registered -ne $path) { throw 'An existing registration points elsewhere. Review that registration before replacing it.' }
}
if (Test-Path -LiteralPath $path) {
    $current = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
    if (($current | ConvertTo-Json -Depth 4) -ne $json) { throw 'A different host manifest already exists. Review it before replacing it.' }
}
if ($PSCmdlet.ShouldProcess($Browser, "Register $($exe.FullName) for the selected extension in the current user profile")) {
    $null = New-Item -ItemType Directory -Force -Path $directory
    if (-not (Test-Path -LiteralPath $path)) { [IO.File]::WriteAllText($path, $json, [Text.UTF8Encoding]::new($false)) }
    $null = New-Item -Path $key -Force
    Set-Item -LiteralPath $key -Value $path
    Write-Output "Registered $Browser host at $path"
}

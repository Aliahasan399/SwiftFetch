param(
    [Parameter(Mandatory)] [ValidateSet('Register','Unregister')] [string] $Action,
    [Parameter(Mandatory)] [string] $ManifestPath,
    [string] $ExecutablePath,
    [ValidatePattern('^[a-p]{32}$')] [string] $ExtensionId,
    [ValidateSet('Edge','Chrome')] [string] $Browser = 'Edge'
)
$ErrorActionPreference = 'Stop'
$vendor = if ($Browser -eq 'Chrome') { 'Google\Chrome' } else { 'Microsoft\Edge' }
$key = "HKCU:\Software\$vendor\NativeMessagingHosts\com.swiftfetch.app"
$manifest = [IO.Path]::GetFullPath($ManifestPath)
$allowed = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../artifacts')) + [IO.Path]::DirectorySeparatorChar
if (-not $manifest.StartsWith($allowed,[StringComparison]::OrdinalIgnoreCase)) { throw 'The test manifest must be under project artifacts.' }
if ($Action -eq 'Register') {
    if (Test-Path -LiteralPath $key) { throw "An existing SwiftFetch $Browser host is registered; the test will not replace it." }
    $executable = (Get-Item -LiteralPath $ExecutablePath).FullName
    $value = @{name='com.swiftfetch.app';description='SwiftFetch isolated acceptance test';type='stdio';path=$executable;allowed_origins=@("chrome-extension://$ExtensionId/")}
    $null = New-Item -ItemType Directory -Force -Path (Split-Path -Parent $manifest)
    if (Test-Path -LiteralPath $manifest) { throw 'The test manifest already exists.' }
    [IO.File]::WriteAllText($manifest,($value|ConvertTo-Json),[Text.UTF8Encoding]::new($false))
    $null = New-Item -Path $key
    Set-Item -LiteralPath $key -Value $manifest
} elseif (Test-Path -LiteralPath $key) {
    if ((Get-Item -LiteralPath $key).GetValue('') -ne $manifest) { throw 'Registration changed during the test; leave it for review.' }
    # This exact key was absent before this test. Never remove a preexisting registration.
    Remove-Item -LiteralPath $key
}

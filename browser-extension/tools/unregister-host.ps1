[CmdletBinding(SupportsShouldProcess)]
param(
    [Parameter(Mandatory)] [ValidateSet('Chrome','Edge','Firefox')] [string] $Browser,
    [string] $ManifestDirectory = (Join-Path $env:LOCALAPPDATA 'SwiftFetch/NativeHost')
)

$ErrorActionPreference = 'Stop'
$key = switch ($Browser) {
    'Chrome' { 'HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.swiftfetch.app' }
    'Edge' { 'HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\com.swiftfetch.app' }
    'Firefox' { 'HKCU:\Software\Mozilla\NativeMessagingHosts\com.swiftfetch.app' }
}
$path = [IO.Path]::GetFullPath((Join-Path (Join-Path $ManifestDirectory $Browser) 'com.swiftfetch.app.json'))

if (Test-Path -LiteralPath $key) {
    $registered = (Get-Item -LiteralPath $key).GetValue('')
    if ($registered -and [IO.Path]::GetFullPath($registered) -ne $path) {
        throw "The registered host manifest ($registered) is not the SwiftFetch-managed manifest. Refusing to remove an unrelated registration."
    }
    if ($PSCmdlet.ShouldProcess($key, "Remove $Browser native host registration")) {
        Remove-Item -LiteralPath $key -Force
        Write-Output "Removed registry key: $key"
    }
}

if (Test-Path -LiteralPath $path) {
    if ($PSCmdlet.ShouldProcess($path, 'Remove SwiftFetch native host manifest')) {
        Remove-Item -LiteralPath $path -Force
        Write-Output "Removed manifest: $path"
    }
}
Write-Output 'SwiftFetch host unregistration complete. Browser settings and unrelated registry entries were left untouched.'

[CmdletBinding()]
param(
    [string] $SourceDirectory = 'src-tauri/target/release/nsis/x64',
    [string] $OutputPath = 'src-tauri/target/release/bundle/nsis/SwiftFetch_0.1.0_x64-setup.exe',
    [switch] $SkipPatch
)

# Recompiles the NSIS installer from a post-build-patched script into an isolated
# working directory, so the produced .exe matches the corrected uninstaller logic.
# Tauri regenerates installer.nsi on every build; build-installers.ps1 calls
# patch-nsis-cleanup.ps1 and then this script after a real build.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    if (-not $SkipPatch) { & (Join-Path $PSScriptRoot 'patch-nsis-cleanup.ps1') }
    if (-not (Test-Path -LiteralPath $SourceDirectory -PathType Container)) { throw "NSIS source directory missing: $SourceDirectory" }
    $makensis = Join-Path $env:LOCALAPPDATA 'tauri\NSIS\makensis.exe'
    if (-not (Test-Path -LiteralPath $makensis -PathType Leaf)) { throw "makensis not found: $makensis" }

    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $work = Join-Path $root ("artifacts/nsis-rebuild/nsis-" + $stamp)
    $null = New-Item -ItemType Directory -Force -Path $work
    Copy-Item -Path (Join-Path $SourceDirectory '*') -Destination $work -Recurse -Force

    $nsiPath = Join-Path $work 'installer.nsi'
    $target = (Resolve-Path -LiteralPath (Split-Path -Parent $OutputPath)).Path.TrimEnd('\') + '\' + (Split-Path -Leaf $OutputPath)
    $text = [System.IO.File]::ReadAllText($nsiPath, [System.Text.Encoding]::UTF8)
    $pattern = '!define OUTFILE\s+"[^"]*"'
    if (-not [regex]::IsMatch($text, $pattern)) { throw 'Could not find the OUTFILE define in installer.nsi.' }
    $text = [regex]::Replace($text, $pattern, ('!define OUTFILE "' + $target.Replace('\','/') + '"'), 1)
    $utf8 = New-Object System.Text.UTF8Encoding($true)
    [System.IO.File]::WriteAllText($nsiPath, $text, $utf8)

    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $makensis /V2 $nsiPath 2>&1 | Out-String | Write-Output } finally { $ErrorActionPreference = $previous }
    if ($LASTEXITCODE -ne 0) { throw "makensis failed with exit code $LASTEXITCODE (work dir: $work)" }
    if (-not (Test-Path -LiteralPath $target -PathType Leaf)) { throw "Recompiled installer not found at $target" }

    $item = Get-Item -LiteralPath $target
    $hash = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    $reportPath = Join-Path $work 'rebuild-result.json'
    [ordered]@{
        schema = 'swiftfetch.nsis-rebuild/1'; timestamp = (Get-Date).ToString('o')
        source_script = (Resolve-Path -LiteralPath $SourceDirectory).Path
        work_directory = $work; output = $item.FullName; bytes = $item.Length
        sha256 = $hash; exit_code = 0
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $reportPath -Encoding utf8
    Write-Output "NSIS rebuilt: $target ($($item.Length) bytes, sha256=$hash)"
    Write-Output "Rebuild evidence: $reportPath"
} finally { Pop-Location }

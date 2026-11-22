[CmdletBinding()]
param(
    [string] $InstallerNsi = 'src-tauri/target/release/nsis/x64/installer.nsi',
    [string] $DataDirectoryName = 'swiftfetch',
    [switch] $CheckOnly
)

# Corrects Tauri's generated NSIS uninstaller so that the explicit "Delete app data"
# option removes only SwiftFetch's real per-user data directory.
#
# Before: RmDir /r "$APPDATA\${BUNDLEID}" and "$LOCALAPPDATA\${BUNDLEID}"
#         (BUNDLEID = com.swiftfetch.app, which is NOT where the app stores data).
# After:  RmDir /r "$APPDATA\swiftfetch" only.
#
# Guarantees retained: deletion still requires the user to tick the checkbox AND the
# run must not be an update. No LOCALAPPDATA directory is removed, so unrelated files
# are never touched. The generated script is rebuilt by Tauri on every build, so this
# script must be called after each build (build-installers.ps1 does this).

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    if (-not (Test-Path -LiteralPath $InstallerNsi -PathType Leaf)) { throw "NSIS script not found: $InstallerNsi" }
    $item = Get-Item -LiteralPath $InstallerNsi
    # Generated NSIS scripts are UTF-8 with BOM and CRLF; preserve that encoding.
    $original = [System.IO.File]::ReadAllText($item.FullName, [System.Text.Encoding]::UTF8)
    $beforeHash = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()

    $oldRoaming = 'RmDir /r "$APPDATA\${BUNDLEID}"'
    $oldLocal = 'RmDir /r "$LOCALAPPDATA\${BUNDLEID}"'
    $newRoaming = 'RmDir /r "$APPDATA\' + $DataDirectoryName + '"'
    $newComment = @(
        '    ; SwiftFetch data is resolved in src-tauri/src/lib.rs as %APPDATA%/swiftfetch.'
        '    ; Only this directory is removed, and only on explicit opt-in outside update mode.'
        '    ; No LOCALAPPDATA path is removed, so unrelated user files are never deleted.'
    ) -join "`r`n"

    $hasOld = $original.Contains($oldRoaming) -or $original.Contains($oldLocal)
    $hasNew = $original.Contains($newRoaming)
    if ($hasOld -and $hasNew) { throw 'NSIS script contains both old and new cleanup paths; refusing ambiguous patch.' }
    if (-not $hasOld -and -not $hasNew) { throw 'NSIS script contains neither expected cleanup path; refusing unknown template.' }

    $patched = $original
    $oldBlock = ($oldRoaming + "`r`n" + '    ' + $oldLocal)
    if ($original.Contains($oldBlock)) {
        $patched = $original.Replace($oldBlock, ($newComment + "`r`n" + '    ' + $newRoaming))
    } else {
        # Handles LF line endings or a single leftover line.
        $patched = $patched.Replace($oldLocal + "`r`n", '')
        $patched = $patched.Replace($oldLocal, '')
        $patched = $patched.Replace($oldRoaming, ($newComment + "`r`n" + '    ' + $newRoaming))
    }

    # Post-conditions: exactly one removal, correct target, guards still intact.
    $removalCount = @([regex]::Matches($patched, [regex]::Escape('RmDir /r '))).Count
    if (-not $patched.Contains($newRoaming)) { throw 'Patched script does not contain the corrected data path.' }
    if ($patched.Contains($oldRoaming) -or $patched.Contains($oldLocal)) { throw 'Old cleanup path still present after patch.' }
    if ($removalCount -ne 1) { throw "Expected exactly one recursive app-data removal, found $removalCount." }
    if (-not $patched.Contains('$DeleteAppDataCheckboxState = 1') -or -not $patched.Contains('$UpdateMode <> 1')) {
        throw 'Explicit opt-in / not-update guards were lost during patching.'
    }

    $status = if ($hasNew) { 'already-patched' } else { 'patched' }
    if (-not $CheckOnly -and -not $hasNew) {
        $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
        $backupDir = Join-Path $root 'artifacts/evidence-backups'
        $null = New-Item -ItemType Directory -Force -Path $backupDir
        Copy-Item -LiteralPath $item.FullName -Destination (Join-Path $backupDir "installer.nsi.bak-$stamp") -Force
        [System.IO.File]::WriteAllText($item.FullName, $patched, (New-Object System.Text.UTF8Encoding($true)))
        $afterHash = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    } else {
        $afterHash = $beforeHash
    }

    $stamp2 = Get-Date -Format 'yyyyMMdd-HHmmss'
    $outDir = Join-Path $root 'artifacts/installer-nsis-patch'
    $null = New-Item -ItemType Directory -Force -Path $outDir
    $reportPath = Join-Path $outDir "patch-$stamp2.json"
    [ordered]@{
        schema = 'swiftfetch.nsis-cleanup-patch/1'
        timestamp = (Get-Date).ToString('o')
        script = $item.FullName
        data_directory = $DataDirectoryName
        status = $status
        check_only = [bool] $CheckOnly
        before_sha256 = $beforeHash
        after_sha256 = $afterHash
        removal_count = $removalCount
        guard_optin_present = $patched.Contains('$DeleteAppDataCheckboxState = 1')
        guard_not_update_present = $patched.Contains('$UpdateMode <> 1')
        localappdata_removal_present = $patched.Contains('$LOCALAPPDATA\${BUNDLEID}')
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $reportPath -Encoding utf8
    Write-Output "NSIS cleanup patch: $status (evidence: $reportPath)"
    if ($CheckOnly) { Write-Output 'CHECK-ONLY: no file was modified.' }
} finally { Pop-Location }

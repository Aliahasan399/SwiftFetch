[CmdletBinding()]
param(
    [string] $ExpectedMsiSha256 = '58df1a1410cd43fb7480e08bb708b5e9f71d7db17c224c1f481907a92d588f32',
    [string] $ExpectedNsisSha256 = '2abe7ec122240b3bac4f8f22d61e5a5540da843ca2367faacaf8c9a6505e4454'
)

# Static installer-behaviour audit. This script NEVER installs, upgrades or uninstalls
# anything. It reads the generated WiX/NSIS sources and the app data-dir code, compares
# them with the accepted baseline, and records hashes of the current artifacts.
# Anything requiring a real Windows machine is reported as a blocker, not simulated.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $findings = [System.Collections.Generic.List[object]]::new()
    function Add-Finding {
        param([string] $Id, [string] $Severity, [string] $Detail, [string] $Source)
        $findings.Add([ordered]@{ id = $Id; severity = $Severity; detail = $Detail; source = $Source })
    }

    $wxsPath = 'src-tauri/target/release/wix/x64/main.wxs'
    $nsiPath = 'src-tauri/target/release/nsis/x64/installer.nsi'
    $libPath = 'src-tauri/src/lib.rs'
    $wxs = if (Test-Path -LiteralPath $wxsPath) { Get-Content -LiteralPath $wxsPath -Raw } else { '' }
    $nsi = if (Test-Path -LiteralPath $nsiPath) { Get-Content -LiteralPath $nsiPath -Raw } else { '' }
    $lib = if (Test-Path -LiteralPath $libPath) { Get-Content -LiteralPath $libPath -Raw } else { '' }

    if (-not $wxs) { Add-Finding 'WIX-SOURCE-MISSING' 'FAIL' 'Generated WiX source not available for static audit.' $wxsPath }
    if (-not $nsi) { Add-Finding 'NSIS-SOURCE-MISSING' 'FAIL' 'Generated NSIS source not available for static audit.' $nsiPath }
    if (-not $lib) { Add-Finding 'RUNTIME-SOURCE-MISSING' 'FAIL' 'Runtime data-dir source not available.' $libPath }

    # --- MSI upgrade identity -------------------------------------------------
    $upgradeCode = if ($wxs -match 'UpgradeCode="([^"]+)"') { $Matches[1] } else { $null }
    $expectedUpgradeCode = '1b0348ae-0c8d-5327-8c38-b0e4d61e9ff1'
    if ($upgradeCode -eq $expectedUpgradeCode) {
        Add-Finding 'MSI-UPGRADECODE' 'PASS' "UpgradeCode is stable: $upgradeCode" "$wxsPath:18"
    } else {
        Add-Finding 'MSI-UPGRADECODE' 'FAIL' "UpgradeCode changed or missing: $upgradeCode (expected $expectedUpgradeCode)" $wxsPath
    }

    $majorSchedule = if ($wxs -match '<MajorUpgrade\s+Schedule="([^"]+)"') { $Matches[1] } else { $null }
    if ($majorSchedule -eq 'afterInstallInitialize') {
        Add-Finding 'MSI-MAJORUPGRADE-SCHEDULE' 'PASS' 'MajorUpgrade runs afterInstallInitialize (old product removed before new files).' "$wxsPath:40"
    } else {
        Add-Finding 'MSI-MAJORUPGRADE-SCHEDULE' 'WARN' "Unexpected MajorUpgrade schedule: $majorSchedule" $wxsPath
    }

    if ($wxs -match 'MajorUpgrade[^>]*AllowDowngrades="yes"') {
        Add-Finding 'MSI-DOWNGRADE' 'WARN' 'MSI allows downgrades; a downgrade path must be tested before release.' "$wxsPath:40"
    } else {
        Add-Finding 'MSI-DOWNGRADE' 'PASS' 'MSI does not explicitly allow downgrades.' $wxsPath
    }

    if ($wxs -match 'InstallScope="perMachine"') {
        Add-Finding 'MSI-SCOPE' 'PASS' 'MSI is per-machine (Program Files), so user data under %APPDATA% lives outside INSTALLDIR.' "$wxsPath:28"
    } else {
        Add-Finding 'MSI-SCOPE' 'WARN' 'MSI install scope is not perMachine; re-check data separation.' $wxsPath
    }

    if ($wxs -match 'RegistrySearch[^>]*Name="InstallDir"') {
        Add-Finding 'MSI-INSTALLDIR-PRESERVED' 'PASS' 'MSI searches the registry for a previous InstallDir before choosing a target.' "$wxsPath:59-65"
    } else {
        Add-Finding 'MSI-INSTALLDIR-PRESERVED' 'WARN' 'No InstallDir registry search found.' $wxsPath
    }

    if ($wxs -match 'RemoveFolder Id="INSTALLDIR"\s+On="uninstall"') {
        Add-Finding 'MSI-UNSCOPE' 'INFO' 'MSI removes only its INSTALLDIR tree on uninstall; no app-data removal is authored.' "$wxsPath:128-129"
    }

    # --- NSIS upgrade / uninstall ---------------------------------------------
    $nsisAllowDowngrades = if ($nsi -match '!define\s+ALLOWDOWNGRADES\s+"([^"]+)"') { $Matches[1] } else { $null }
    if ($nsisAllowDowngrades -eq 'true') {
        Add-Finding 'NSIS-DOWNGRADE' 'WARN' 'NSIS ALLOWDOWNGRADES=true; downgrade and rollback paths must be tested.' "$nsiPath:54"
    } else {
        Add-Finding 'NSIS-DOWNGRADE' 'PASS' "NSIS ALLOWDOWNGRADES=$nsisAllowDowngrades" $nsiPath
    }

    $bundleId = if ($nsi -match '!define\s+BUNDLEID\s+"([^"]+)"') { $Matches[1] } else { $null }
    if ($nsi -match 'RmDir\s+/r\s+"\$APPDATA\\\$\{BUNDLEID\}"') {
        Add-Finding 'NSIS-DATA-DELETE-PATH' 'WARN' "NSIS opt-in app-data deletion targets `$APPDATA\\$bundleId only." "$nsiPath:863-879"
    }

    if ($nsi -match '\$DeleteAppDataCheckboxState\s*=\s*1[\s\S]{0,200}\$UpdateMode\s*<>\s*1') {
        Add-Finding 'NSIS-DATA-DELETE-GUARD' 'PASS' 'NSIS deletes app data only when the checkbox is selected AND not in update mode.' "$nsiPath:865-866"
    } else {
        Add-Finding 'NSIS-DATA-DELETE-GUARD' 'FAIL' 'Could not confirm the NSIS app-data deletion guards.' $nsiPath
    }

    if ($nsi -match 'RequestExecutionLevel\s+admin') {
        Add-Finding 'NSIS-ELEVATION' 'INFO' 'NSIS requests admin elevation in per-machine mode; per-user mode uses user level.' "$nsiPath:101-105"
    }

    # --- Runtime data path vs installer cleanup path --------------------------
    $runtimeDataPath = if ($lib -match '\.join\("swiftfetch"\)\.join\("state\.sqlite"\)') { '%APPDATA%/swiftfetch/state.sqlite' } else { $null }
    if ($runtimeDataPath) {
        Add-Finding 'RUNTIME-DATA-PATH' 'PASS' "Runtime database resolves to $runtimeDataPath." "$libPath:60-62"
    } else {
        Add-Finding 'RUNTIME-DATA-PATH' 'FAIL' 'Could not confirm the runtime data path in lib.rs.' $libPath
    }
    if ($runtimeDataPath -and $bundleId -and $bundleId -ne 'swiftfetch') {
        Add-Finding 'DATA-PATH-MISMATCH' 'WARN' "Runtime uses %APPDATA%/swiftfetch but NSIS cleanup targets `$APPDATA/$bundleId; the opt-in delete would not remove the real data directory. Preserve-by-default holds, but opt-in cleanup is ineffective." "$nsiPath:877 / $libPath:61"
    }

    # --- Artifact hashes -------------------------------------------------------
    $artifacts = @()
    foreach ($entry in @(
        @{ kind = 'msi'; path = 'src-tauri/target/release/bundle/msi/SwiftFetch_0.1.0_x64_en-US.msi'; expected = $ExpectedMsiSha256 },
        @{ kind = 'nsis'; path = 'src-tauri/target/release/bundle/nsis/SwiftFetch_0.1.0_x64-setup.exe'; expected = $ExpectedNsisSha256 }
    )) {
        if (-not (Test-Path -LiteralPath $entry.path -PathType Leaf)) {
            Add-Finding "$($entry.kind.ToUpper())-ARTIFACT" 'FAIL' "Artifact missing: $($entry.path)" $entry.path
            continue
        }
        $item = Get-Item -LiteralPath $entry.path
        $hash = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        $match = $hash -eq $entry.expected.ToLowerInvariant()
        Add-Finding "$($entry.kind.ToUpper())-HASH" $(if ($match) { 'PASS' } else { 'FAIL' }) "$($entry.kind) sha256=$hash (baseline=$($entry.expected))" $entry.path
        $sig = (Get-AuthenticodeSignature -LiteralPath $item.FullName).Status.ToString()
        if ($sig -ne 'Valid') {
            Add-Finding "$($entry.kind.ToUpper())-SIGNATURE" 'WARN' "Authenticode status is $sig; a trusted signed artifact is required before public release." $entry.path
        }
        $artifacts += [ordered]@{ kind=$entry.kind; path=$item.FullName; bytes=$item.Length; sha256=$hash; hash_matches_baseline=$match; authenticode=$sig }
    }

    Add-Finding 'CLEAN-VM' 'BLOCKER' 'No real clean-Windows VM install/upgrade/downgrade/uninstall evidence exists in this environment. Static and offline checks are not a substitute.' 'docs/release/CLEAN_MACHINE_ACCEPTANCE.md'
    Add-Finding 'ROLLBACK' 'BLOCKER' 'Failed-upgrade rollback requirements are documented but not executed; rollback restores the bound metadata snapshot only after a real test.' 'docs/release/INSTALLER_ENGINEERING.md'

    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $outDir = Join-Path $root 'artifacts/installer-audit'
    $null = New-Item -ItemType Directory -Force -Path $outDir
    $reportPath = Join-Path $outDir "audit-$stamp.json"
    [ordered]@{
        schema = 'swiftfetch.installer-audit/1'
        timestamp = (Get-Date).ToString('o')
        execution_mode = 'static-inspection-only'
        artifacts = $artifacts
        findings = @($findings)
        blocker_count = @($findings | Where-Object { $_.severity -eq 'BLOCKER' }).Count
        fail_count = @($findings | Where-Object { $_.severity -eq 'FAIL' }).Count
        warn_count = @($findings | Where-Object { $_.severity -eq 'WARN' }).Count
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $reportPath -Encoding utf8
    Write-Output "Installer audit report: $reportPath"
    Write-Output ("findings: {0} total, {1} blocker, {2} fail, {3} warn" -f $findings.Count,
        @($findings | Where-Object { $_.severity -eq 'BLOCKER' }).Count,
        @($findings | Where-Object { $_.severity -eq 'FAIL' }).Count,
        @($findings | Where-Object { $_.severity -eq 'WARN' }).Count)
    if (@($findings | Where-Object { $_.severity -eq 'FAIL' }).Count -gt 0) { throw 'Installer audit found FAIL findings; inspect the report.' }
} finally { Pop-Location }

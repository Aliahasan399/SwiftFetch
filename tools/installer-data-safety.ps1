[CmdletBinding()]
param([string] $DataRoot)

# Offline data-preservation / deletion simulation for SwiftFetch installers.
# This is NOT a clean-machine test and does NOT run any installer. It models the
# app-data file operations authored in the generated installer sources against a
# synthetic per-user tree, then asserts both preserve-by-default and explicit opt-in
# deletion behaviour, plus that unrelated user files are never touched.
#
# Modelled sources:
#   MSI : src-tauri/target/release/wix/x64/main.wxs (no app-data removal authored)
#   NSIS: src-tauri/target/release/nsis/x64/installer.nsi:863-880 (patched: checkbox=1
#         AND not UpdateMode, removes only $APPDATA\swiftfetch)
#   App : src-tauri/src/lib.rs:60-62 (%APPDATA%/swiftfetch/state.sqlite)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $outDir = Join-Path $root 'artifacts/installer-data-safety'
    $null = New-Item -ItemType Directory -Force -Path $outDir
    if (-not $DataRoot) { $DataRoot = Join-Path $outDir "sim-$stamp" }
    $base = (Resolve-Path -LiteralPath $DataRoot -ErrorAction SilentlyContinue)
    if (-not $base) { $base = (New-Item -ItemType Directory -Force -Path $DataRoot).FullName }

    function Get-Manifest {
        param([string] $Path)
        $map = @{}
        if (-not (Test-Path -LiteralPath $Path)) { return $map }
        $prefix = (Get-Item -LiteralPath $Path).FullName.TrimEnd('\') + '\'
        Get-ChildItem -LiteralPath $Path -Recurse -File -Force | ForEach-Object {
            $map[$_.FullName.Substring($prefix.Length)] = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        }
        return $map
    }

    $source = Join-Path $base 'source\AppData'
    $roaming = Join-Path $source 'Roaming'
    $local = Join-Path $source 'Local'

    # Runtime-owned data (the preservation contract).
    $owned = @('state.sqlite', 'settings.json', 'history/history.json',
               'queues/queue.json', 'downloads/sample.bin', 'partial/part.dat')
    foreach ($f in $owned) {
        $target = Join-Path (Join-Path $roaming 'swiftfetch') $f
        $null = New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target)
        [System.IO.File]::WriteAllText($target, "owned:$f")
    }
    # Unrelated data that must survive every scenario.
    $unrelated = @('Roaming\com.swiftfetch.app\stale.txt', 'Roaming\OtherApp\config.json',
                   'Local\swiftfetch\local.dat')
    foreach ($f in $unrelated) {
        $target = Join-Path $source $f
        $null = New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target)
        [System.IO.File]::WriteAllText($target, "unrelated:$f")
    }

    $baseline = Get-Manifest -Path $source
    $ownedKeys = @($baseline.Keys | Where-Object { $_ -like 'Roaming\swiftfetch\*' })
    $unrelatedKeys = @($baseline.Keys | Where-Object { $_ -like 'Roaming\com.swiftfetch.app\*' -or $_ -like 'Roaming\OtherApp\*' -or $_ -like 'Local\swiftfetch\*' })
    $scenarios = [System.Collections.Generic.List[object]]::new()

    function Invoke-Scenario {
        param([string] $Name, [string] $Description, [string] $Expectation, [scriptblock] $Operation)
        $caseRoot = Join-Path $base "case-$Name"
        $null = New-Item -ItemType Directory -Force -Path $caseRoot
        $dest = Join-Path $caseRoot 'AppData'
        Copy-Item -LiteralPath $source -Destination $dest -Recurse -Force
        & $Operation $dest
        $after = Get-Manifest -Path $dest
        $ownedMissing = @($ownedKeys | Where-Object { -not $after.ContainsKey($_) })
        $ownedChanged = @($ownedKeys | Where-Object { $after.ContainsKey($_) -and $after[$_] -ne $baseline[$_] })
        $unrelatedMissing = @($unrelatedKeys | Where-Object { -not $after.ContainsKey($_) })
        $unrelatedChanged = @($unrelatedKeys | Where-Object { $after.ContainsKey($_) -and $after[$_] -ne $baseline[$_] })
        $ownedAbsent = ($ownedMissing.Count -eq $ownedKeys.Count)
        $expectationMet = if ($Expectation -eq 'delete-consented') { $ownedAbsent } else { ($ownedMissing.Count -eq 0 -and $ownedChanged.Count -eq 0) }
        $unrelatedIntact = ($unrelatedMissing.Count -eq 0 -and $unrelatedChanged.Count -eq 0)
        $scenarios.Add([ordered]@{
            name = $Name; description = $Description; expectation = $Expectation
            expectation_met = $expectationMet; unrelated_files_intact = $unrelatedIntact
            owned_missing = $ownedMissing; owned_changed = $ownedChanged
            unrelated_missing = $unrelatedMissing; unrelated_changed = $unrelatedChanged
            owned_absent_after = $ownedAbsent
        })
    }

    Invoke-Scenario 'msi-major-upgrade' 'MajorUpgrade afterInstallInitialize touches INSTALLDIR only; no app-data operation is authored.' 'preserve' { param($d) }
    Invoke-Scenario 'msi-uninstall' 'MSI uninstall removes INSTALLDIR and shortcuts only; no app-data removal authored.' 'preserve' { param($d) }
    Invoke-Scenario 'nsis-update-mode' 'NSIS UpdateMode=1 suppresses the app-data deletion branch entirely.' 'preserve' { param($d) }
    Invoke-Scenario 'nsis-uninstall-default' 'Delete-app-data checkbox unchecked; NSIS leaves all data untouched.' 'preserve' { param($d) }
    Invoke-Scenario 'nsis-uninstall-optin' 'Checkbox checked outside update mode: NSIS removes only $APPDATA\swiftfetch.' 'delete-consented' {
        param($d)
        $target = Join-Path $d 'Roaming\swiftfetch'
        if (Test-Path -LiteralPath $target) { Remove-Item -LiteralPath $target -Recurse -Force }
    }

    $failed = @($scenarios | Where-Object { -not $_.expectation_met -or -not $_.unrelated_files_intact })
    $reportPath = Join-Path $outDir "data-safety-$stamp.json"
    [ordered]@{
        schema = 'swiftfetch.installer-data-safety/2'
        timestamp = (Get-Date).ToString('o')
        execution_mode = 'offline-simulation'
        clean_machine = $false
        modeled_after = @('src-tauri/target/release/wix/x64/main.wxs', 'src-tauri/target/release/nsis/x64/installer.nsi:863-880', 'src-tauri/src/lib.rs:60-62')
        data_root = $base
        baseline_file_count = $baseline.Count
        owned_file_count = $ownedKeys.Count
        unrelated_file_count = $unrelatedKeys.Count
        scenarios = @($scenarios)
        all_expectations_met = ($failed.Count -eq 0)
        note = 'Offline modelling only. A real clean-Windows VM test is still required.'
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $reportPath -Encoding utf8
    Write-Output "Data-safety simulation report: $reportPath"
    $scenarios | ForEach-Object {
        Write-Output ("{0,-26} expectation={1,-16} met={2} unrelated_intact={3}" -f $_.name, $_.expectation, $_.expectation_met, $_.unrelated_files_intact)
    }
    if ($failed.Count -gt 0) { throw 'Data-safety simulation failed; inspect the report.' }
} finally { Pop-Location }

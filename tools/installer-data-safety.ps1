[CmdletBinding()]
param([string] $DataRoot)

# Offline data-preservation simulation for SwiftFetch installers.
# This is NOT a clean-machine test and does NOT run any installer. It models the
# app-data file operations that the generated MSI/NSIS sources actually author,
# against a synthetic %APPDATA% tree, and reports whether downloads, settings,
# history, queues and partial files survive each scenario.
#
# Evidence for the modelled operations:
#   MSI : src-tauri/target/release/wix/x64/main.wxs (no app-data removal authored;
#         install scope perMachine; INSTALLDIR is Program Files)
#   NSIS: src-tauri/target/release/nsis/x64/installer.nsi:863-879
#         (app data deletion only when checkbox=1 AND not UpdateMode, and only
#          $APPDATA\com.swiftfetch.app / $LOCALAPPDATA\com.swiftfetch.app)
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
            $rel = $_.FullName.Substring($prefix.Length)
            $map[$rel] = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        }
        return $map
    }

    # Build a synthetic per-user data tree that mirrors what the app writes.
    $appData = Join-Path $base 'AppData\Roaming'
    $appDir = Join-Path $appData 'swiftfetch'
    $files = @(
        'state.sqlite', 'settings.json', 'history/history.json',
        'queues/queue.json', 'downloads/sample.bin', 'partial/part.dat'
    )
    foreach ($f in $files) {
        $target = Join-Path $appDir $f
        $null = New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target)
        [System.IO.File]::WriteAllText($target, "sentinel:$f")
    }
    # Decoy matching the NSIS cleanup path, to show the path mismatch explicitly.
    $nsisDir = Join-Path $appData 'com.swiftfetch.app'
    $null = New-Item -ItemType Directory -Force -Path $nsisDir
    [System.IO.File]::WriteAllText((Join-Path $nsisDir 'stale.txt'), 'nsis-cleanup-target')

    $baseline = Get-Manifest -Path $appData
    $scenarios = [System.Collections.Generic.List[object]]::new()

    function Invoke-Scenario {
        param([string] $Name, [string] $Description, [scriptblock] $Operation)
        $caseRoot = Join-Path $base "case-$Name"
        $null = New-Item -ItemType Directory -Force -Path $caseRoot
        $dest = Join-Path $caseRoot 'AppData\Roaming'
        Copy-Item -LiteralPath $appData -Destination $dest -Recurse -Force
        & $Operation $dest
        $after = Get-Manifest -Path $dest
        # Only runtime-owned data under swiftfetch\ is the preservation contract.
        # The com.swiftfetch.app decoy exists solely to model the NSIS cleanup path.
        $owned = @($baseline.Keys | Where-Object { $_ -like 'swiftfetch\*' })
        $missing = @($owned | Where-Object { -not $after.ContainsKey($_) })
        $changed = @($owned | Where-Object { $after.ContainsKey($_) -and $after[$_] -ne $baseline[$_] })
        $pass = ($missing.Count -eq 0 -and $changed.Count -eq 0)
        $scenarios.Add([ordered]@{
            name = $Name; description = $Description; preserved = $pass
            missing_count = $missing.Count; changed_count = $changed.Count
            missing = @($missing); changed = @($changed); files_after = $after.Count
            nsis_cleanup_target_removed = (-not (Test-Path -LiteralPath (Join-Path $dest 'com.swiftfetch.app')))
        })
    }

    $expectedPath = 'swiftfetch\state.sqlite'
    Invoke-Scenario 'msi-major-upgrade' 'MajorUpgrade afterInstallInitialize touches INSTALLDIR only; no app-data operation is authored.' {
        param($d) # no-op by design
    }
    Invoke-Scenario 'msi-uninstall' 'MSI uninstall removes INSTALLDIR and shortcuts only; no app-data removal authored.' {
        param($d) # no-op by design
    }
    Invoke-Scenario 'nsis-update-mode' 'NSIS UpdateMode=1 suppresses the app-data deletion branch entirely.' {
        param($d) # no-op by design
    }
    Invoke-Scenario 'nsis-uninstall-default' 'Delete-app-data checkbox unchecked; NSIS leaves all app data untouched.' {
        param($d) # no-op by design
    }
    Invoke-Scenario 'nsis-uninstall-optin' 'Checkbox checked: NSIS removes only $APPDATA\com.swiftfetch.app; the real swiftfetch tree is untouched.' {
        param($d)
        $target = Join-Path $d 'com.swiftfetch.app'
        if (Test-Path -LiteralPath $target) { Remove-Item -LiteralPath $target -Recurse -Force }
    }

    $reportPath = Join-Path $outDir "data-safety-$stamp.json"
    [ordered]@{
        schema = 'swiftfetch.installer-data-safety/1'
        timestamp = (Get-Date).ToString('o')
        execution_mode = 'offline-simulation'
        clean_machine = $false
        modeled_after = @('src-tauri/target/release/wix/x64/main.wxs', 'src-tauri/target/release/nsis/x64/installer.nsi:863-879', 'src-tauri/src/lib.rs:60-62')
        data_root = $base
        baseline_file_count = $baseline.Count
        expected_runtime_file = $expectedPath
        scenarios = @($scenarios)
        all_preserved = @($scenarios | Where-Object { -not $_.preserved }).Count -eq 0
        note = 'Offline modelling only. A real clean-Windows VM install/upgrade/uninstall test is still required.'
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $reportPath -Encoding utf8
    Write-Output "Data-safety simulation report: $reportPath"
    $scenarios | ForEach-Object { Write-Output ("{0}: preserved={1}" -f $_.name, $_.preserved) }
    if (@($scenarios | Where-Object { -not $_.preserved }).Count -gt 0) { throw 'Data preservation simulation failed; inspect the report.' }
} finally { Pop-Location }

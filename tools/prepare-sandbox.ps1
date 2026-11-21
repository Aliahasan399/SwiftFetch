[CmdletBinding()]
param([string] $CandidateDirectory)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    $candidate = (Get-Item -LiteralPath $CandidateDirectory).FullName
    $output = Join-Path (Get-Location) ("artifacts/sandbox-" + [Guid]::NewGuid().ToString('N'))
    $null = New-Item -ItemType Directory -Path $output
    $source = [Security.SecurityElement]::Escape($candidate)
    $target = [Security.SecurityElement]::Escape($output)
    $configuration = @"
<Configuration>
  <Networking>Enable</Networking>
  <MappedFolders>
    <MappedFolder><HostFolder>$source</HostFolder><SandboxFolder>C:\Candidate</SandboxFolder><ReadOnly>true</ReadOnly></MappedFolder>
    <MappedFolder><HostFolder>$target</HostFolder><SandboxFolder>C:\Evidence</SandboxFolder><ReadOnly>false</ReadOnly></MappedFolder>
  </MappedFolders>
  <LogonCommand><Command>powershell.exe -NoProfile -Command "Get-ChildItem C:\Candidate; Write-Host 'Install the candidate, verify publisher and download controls, then upgrade/rollback/uninstall without removing downloads. Save results under C:\Evidence.'; Read-Host 'Press Enter to continue'"</Command></LogonCommand>
</Configuration>
"@
    Set-Content -LiteralPath "$output/acceptance.wsb" -Value $configuration -Encoding utf8
    Write-Output "Clean-Windows acceptance configuration: $output/acceptance.wsb"
    if (-not (Test-Path -LiteralPath "$env:windir/System32/WindowsSandbox.exe")) {
        Write-Output 'Windows Sandbox is not installed on this machine. No Windows features were changed.'
    }
} finally { Pop-Location }

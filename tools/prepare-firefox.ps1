[CmdletBinding()]
param([string] $Version='160.0.1')
$ErrorActionPreference='Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
try{
    $root=Join-Path (Get-Location) ("artifacts/firefox-runtime-"+[Guid]::NewGuid().ToString('N'))
    $null=New-Item -ItemType Directory -Path $root
    $installer=Join-Path $root 'firefox-setup.exe'
    $url="https://archive.mozilla.org/pub/firefox/releases/$Version/win64/en-US/Firefox%20Setup%20$Version.exe"
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $installer
    $signature=Get-AuthenticodeSignature -FilePath $installer
    if($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'Mozilla Corporation'){
        throw "The downloaded Firefox installer must have a valid Mozilla signature. Found $($signature.Status)."
    }
    $script=@'
from pathlib import Path
import mmap,shutil,sys
root=Path(sys.argv[1])
with (root/'firefox-setup.exe').open('rb') as src:
    with mmap.mmap(src.fileno(),0,access=mmap.ACCESS_READ) as mapped:
        position=mapped.find(bytes([55,122,188,175,39,28]))
    if position<0: raise RuntimeError('The signed installer has no recognizable 7z payload')
    src.seek(position)
    with (root/'payload.7z').open('xb') as dst:shutil.copyfileobj(src,dst)
'@
    $script | python -B - $root
    if($LASTEXITCODE -ne 0){throw 'Could not inspect the signed installer payload.'}
    $extract=Join-Path $root 'extracted';$null=New-Item -ItemType Directory -Path $extract
    & "$env:windir/System32/tar.exe" -xf (Join-Path $root 'payload.7z') -C $extract
    if($LASTEXITCODE -ne 0){throw 'Could not extract the signed Firefox payload.'}
    $firefox=Get-ChildItem -LiteralPath $extract -Recurse -File -Filter 'firefox.exe' | Select-Object -First 1
    if(-not $firefox){throw 'Firefox executable missing from the extracted payload.'}
    $python=(Get-Command python.exe).Source
    if(Get-Command py.exe -ErrorAction SilentlyContinue){$python=(& py -3.12 -c 'import sys;print(sys.executable)'|Out-String).Trim()}
    & $python -m pip install --disable-pip-version-check --target (Join-Path $root 'python') 'marionette-driver==3.7.1'
    if($LASTEXITCODE -ne 0){throw 'Could not stage the pinned Mozilla automation driver.'}
    @{version=$Version;firefox=$firefox.FullName;python=$python;python_path=(Join-Path $root 'python');installer_sha256=(Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash}|
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root 'runtime.json') -Encoding utf8
    Write-Output "Firefox isolated runtime: $root/runtime.json"
}finally{Pop-Location}

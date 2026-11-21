function New-ProjectArchive {
    param(
        [Parameter(Mandatory)] [string] $SourceDirectory,
        [Parameter(Mandatory)] [string] $DestinationPath,
        [switch] $IncludeRoot
    )
    Add-Type -AssemblyName System.IO.Compression
    $root = (Get-Item -LiteralPath $SourceDirectory).FullName.TrimEnd('\', '/')
    $file = [IO.File]::Open([IO.Path]::GetFullPath($DestinationPath), [IO.FileMode]::CreateNew)
    $zip = $null
    try {
        $zip = [IO.Compression.ZipArchive]::new($file, [IO.Compression.ZipArchiveMode]::Create)
        Get-ChildItem -LiteralPath $root -Recurse -File -Force | ForEach-Object {
            $relative = $_.FullName.Substring($root.Length + 1).Replace('\', '/')
            if ($IncludeRoot) { $relative = "$(Split-Path -Leaf $root)/$relative" }
            $entry = $zip.CreateEntry($relative, [IO.Compression.CompressionLevel]::Optimal)
            $inputStream = [IO.File]::OpenRead($_.FullName)
            $outputStream = $null
            try {
                $outputStream = $entry.Open()
                $inputStream.CopyTo($outputStream)
            } finally {
                if ($outputStream) { $outputStream.Dispose() }
                $inputStream.Dispose()
            }
        }
    } finally {
        if ($zip) { $zip.Dispose() } else { $file.Dispose() }
    }
}

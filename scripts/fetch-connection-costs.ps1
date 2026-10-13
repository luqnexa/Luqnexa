# SPDX-License-Identifier: AGPL-3.0-only
param([string]$OutputPath = '.cache/statistical-lattice/connection.txt')
$ErrorActionPreference = 'Stop'
$revision = '18e76511af9371a9a4409b1d13c3e30884b53e9c'
$expectedHash = '68823a1f6617372ccb89afdbcb334d7a0bcfa82c9c5dda03fc1f80f1cab5c48e'
$expectedIds = '07a05a268c3783b7e02b36e4aa591555a2dd19a1f3c93891576740761c154f2d'
$idPath = Join-Path $PSScriptRoot '../dictionaries/mozc-id.def'
if ((Get-FileHash -LiteralPath $idPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedIds) {
    throw 'Local POS definitions do not match pinned connection data.'
}
$destination = [IO.Path]::GetFullPath($OutputPath)
if (Test-Path -LiteralPath $destination) {
    if ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedHash) {
        throw 'Existing output has a different hash; refusing to overwrite.'
    }
    Write-Output "Verified existing connection data: $expectedHash"
    return
}
[IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($destination)) | Out-Null
$temporary = "$destination.$([Guid]::NewGuid().ToString('N')).download"
try {
    Invoke-WebRequest -Uri "https://raw.githubusercontent.com/google/mozc/$revision/src/data/dictionary_oss/connection_single_column.txt" -OutFile $temporary
    if ((Get-FileHash -LiteralPath $temporary -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedHash) {
        throw 'Connection data hash mismatch.'
    }
    # No overwrite if another operation created the destination meanwhile.
    [IO.File]::Move($temporary, $destination)
} finally {
    if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary }
}
Write-Output "Downloaded and verified connection data: $expectedHash"

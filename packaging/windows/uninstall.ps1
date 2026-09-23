$ErrorActionPreference = 'Stop'

$installDirectory = Join-Path $env:LOCALAPPDATA 'Programs\Atlas'
$startMenuDirectory = Join-Path ([Environment]::GetFolderPath('Programs')) 'Atlas'

if (Test-Path -LiteralPath $startMenuDirectory) {
    Remove-Item -LiteralPath $startMenuDirectory -Recurse -Force
}
if (Test-Path -LiteralPath $installDirectory) {
    Remove-Item -LiteralPath $installDirectory -Recurse -Force
}

Write-Host 'Atlas was removed. Your game files and settings in AppData were kept.'

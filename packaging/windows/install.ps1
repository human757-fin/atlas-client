$ErrorActionPreference = 'Stop'

$sourceDirectory = $PSScriptRoot
$installDirectory = Join-Path $env:LOCALAPPDATA 'Programs\Atlas'
$programsDirectory = [Environment]::GetFolderPath('Programs')
$startMenuDirectory = Join-Path $programsDirectory 'Atlas'
$executable = Join-Path $sourceDirectory 'atlas-client.exe'

if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw 'atlas-client.exe must be beside this installer. Extract the release archive first.'
}

New-Item -ItemType Directory -Force -Path $installDirectory, $startMenuDirectory | Out-Null
Copy-Item -LiteralPath $executable -Destination (Join-Path $installDirectory 'atlas-client.exe') -Force
Copy-Item -LiteralPath (Join-Path $sourceDirectory 'uninstall.ps1') -Destination (Join-Path $installDirectory 'uninstall.ps1') -Force

$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut((Join-Path $startMenuDirectory 'Atlas.lnk'))
$shortcut.TargetPath = Join-Path $installDirectory 'atlas-client.exe'
$shortcut.WorkingDirectory = $installDirectory
$shortcut.Description = 'Performance-focused Minecraft launcher'
$shortcut.Save()

Write-Host "Atlas installed. Find it in the Start Menu under Atlas."

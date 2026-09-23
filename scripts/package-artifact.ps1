$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $root 'src-tauri\target\release\pocketdrop-m0.exe'
if (-not (Test-Path -LiteralPath $exe)) { throw 'Run npm.cmd run package first.' }
$out = Join-Path $root 'artifacts'
$stage = Join-Path $out 'PocketDrop-M0-windows-x64'
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item -LiteralPath $exe -Destination "$stage\PocketDrop-M0.exe" -Force
Copy-Item -LiteralPath "$root\README.md" -Destination $stage -Force
foreach ($folder in @('docs','fixtures','evidence')) { Copy-Item -LiteralPath "$root\$folder" -Destination $stage -Recurse -Force }
$hash = Get-FileHash -LiteralPath "$stage\PocketDrop-M0.exe" -Algorithm SHA256
"$($hash.Hash)  PocketDrop-M0.exe" | Set-Content -LiteralPath "$stage\SHA256SUMS.txt" -Encoding ASCII
Compress-Archive -LiteralPath $stage -DestinationPath "$out\PocketDrop-M0-windows-x64.zip" -Force
Get-Item "$stage\PocketDrop-M0.exe", "$out\PocketDrop-M0-windows-x64.zip" | Select-Object FullName,Length
Write-Output $hash.Hash

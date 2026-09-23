$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$version = (Get-Content -LiteralPath "$root/src-tauri/tauri.conf.json" -Raw | ConvertFrom-Json).version
$artifact = Join-Path $root "artifacts/PocketDrop-Phone-Trial-$version"
New-Item -ItemType Directory -Path $artifact -Force | Out-Null
$sourceExe = "$root/src-tauri/target/release/pocketdrop-m0.exe"
$targetExe = "$artifact/PocketDrop.exe"
# A validated running build need not be overwritten if its bytes already match.
if (-not (Test-Path -LiteralPath $targetExe) -or (Get-FileHash -LiteralPath $sourceExe).Hash -ne (Get-FileHash -LiteralPath $targetExe).Hash) {
    Copy-Item -LiteralPath $sourceExe -Destination $targetExe -Force
}
Copy-Item -LiteralPath "$root/README.md" -Destination "$artifact/README.md" -Force
New-Item -ItemType Directory -Path "$artifact/docs" -Force | Out-Null
Get-ChildItem -LiteralPath "$root/docs" -Filter '*.md' -File | Copy-Item -Destination "$artifact/docs" -Force
Copy-Item -LiteralPath "$root/docs/PHONE_TRIAL_ACCEPTANCE.md" -Destination "$artifact/PHONE_TRIAL_ACCEPTANCE.md" -Force
Copy-Item -LiteralPath "$root/docs/PHONE_TRIAL_PROTOCOL.md" -Destination "$artifact/PHONE_TRIAL_PROTOCOL.md" -Force
Copy-Item -LiteralPath "$root/android/app/build/outputs/apk/debug/app-debug.apk" -Destination "$root/artifacts/PocketDrop-Android-$version.apk" -Force
$hashes = @("PocketDrop.exe SHA256: $((Get-FileHash -LiteralPath "$artifact/PocketDrop.exe" -Algorithm SHA256).Hash)", "PocketDrop-Android-$version.apk SHA256: $((Get-FileHash -LiteralPath "$root/artifacts/PocketDrop-Android-$version.apk" -Algorithm SHA256).Hash)")
Set-Content -LiteralPath "$artifact/SHA256SUMS.txt" -Value $hashes -Encoding UTF8
Copy-Item -LiteralPath "$artifact/SHA256SUMS.txt" -Destination "$root/artifacts/PHONE-TRIAL-SHA256SUMS.txt" -Force
Compress-Archive -Path "$artifact/*" -DestinationPath "$root/artifacts/PocketDrop-Phone-Trial-$version-windows-x64.zip" -Force
Get-Item "$root/artifacts/PocketDrop-Android-$version.apk", "$root/artifacts/PocketDrop-Phone-Trial-$version-windows-x64.zip" | Select-Object Name,Length

param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
$version = (Get-Content -Raw package.json | ConvertFrom-Json).version
if (-not $SkipBuild) {
    npm.cmd run package
    if ($LASTEXITCODE -ne 0) { throw 'Windows packaging failed' }
    & "$PSScriptRoot/android-build.ps1"
}
$output = Join-Path $root "artifacts/release-$version"
New-Item -ItemType Directory -Force -Path $output | Out-Null
$files = @(
    @{ Source = "src-tauri/target/release/bundle/nsis/PocketDrop_${version}_x64-setup.exe"; Name = "PocketDrop-$version-Windows-x64-Setup.exe" },
    @{ Source = 'android/app/build/outputs/apk/release/app-release.apk'; Name = "PocketDrop-$version-Android.apk" }
)
$lines = foreach ($item in $files) {
    if (-not (Test-Path -LiteralPath $item.Source)) { throw "Missing artifact: $($item.Source)" }
    $destination = Join-Path $output $item.Name
    Copy-Item -LiteralPath $item.Source -Destination $destination -Force
    "{0}  {1}" -f (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant(), $item.Name
}
[IO.File]::WriteAllLines((Join-Path $output 'SHA256SUMS.txt'), $lines, [Text.UTF8Encoding]::new($false))
Write-Output $output

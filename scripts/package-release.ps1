param([switch]$SkipBuild, [switch]$WindowsOnly)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
$version = (Get-Content -Raw package.json | ConvertFrom-Json).version
if (-not $SkipBuild) {
    npm.cmd run package
    if ($LASTEXITCODE -ne 0) { throw 'Windows packaging failed' }
    if (-not $WindowsOnly) { & "$PSScriptRoot/android-build.ps1" }
}
$output = Join-Path $root "artifacts/release-$version"
New-Item -ItemType Directory -Force -Path $output | Out-Null
$files = @(
    @{ Source = "src-tauri/target/release/bundle/nsis/PocketDrop_${version}_x64-setup.exe"; Name = "PocketDrop-$version-Windows-x64-Setup.exe" },
    @{ Source = 'android/app/build/outputs/apk/release/app-release.apk'; Name = "PocketDrop-$version-Android.apk" }
)
if ($WindowsOnly) { $files = @($files[0]) }
$files += @{ Source = "src-tauri/target/release/bundle/nsis/PocketDrop_${version}_x64-setup.exe.sig"; Name = "PocketDrop-$version-Windows-x64-Setup.exe.sig" }
$lines = @(foreach ($item in $files) {
    if (-not (Test-Path -LiteralPath $item.Source)) { throw "Missing artifact: $($item.Source)" }
    $destination = Join-Path $output $item.Name
    Copy-Item -LiteralPath $item.Source -Destination $destination -Force
    "{0}  {1}" -f (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant(), $item.Name
})
$signature = [IO.File]::ReadAllText((Join-Path $output "PocketDrop-$version-Windows-x64-Setup.exe.sig")).Trim()
$notesPath = Join-Path $root "docs/RELEASE_$version.md"
if (-not (Test-Path -LiteralPath $notesPath)) { throw "Missing release notes: $notesPath" }
$manifest = @{
    version = $version
    notes = [IO.File]::ReadAllText($notesPath)
    pub_date = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
    platforms = @{
        'windows-x86_64' = @{
            signature = $signature
            url = "https://github.com/OverGreen996/PocketDrop/releases/download/v$version/PocketDrop-$version-Windows-x64-Setup.exe"
        }
    }
} | ConvertTo-Json -Depth 5
$manifestPath = Join-Path $output 'latest.json'
[IO.File]::WriteAllText($manifestPath, $manifest, [Text.UTF8Encoding]::new($false))
$lines += "{0}  latest.json" -f (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllLines((Join-Path $output 'SHA256SUMS.txt'), $lines, [Text.UTF8Encoding]::new($false))
Write-Output $output

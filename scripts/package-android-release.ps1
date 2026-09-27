param([switch]$SkipBuild)
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
Set-Location $root
if (-not $SkipBuild) { & "$PSScriptRoot/android-build.ps1" }
$config=Get-Content -Raw android/app/build.gradle
if ($config -notmatch "versionCode (\d+); versionName '([^']+)'") { throw 'Android version not found' }
$code=[int]$Matches[1];$version=$Matches[2]
$env:JAVA_HOME=(Get-ChildItem "$root/.tools/android-build/jdk" -Directory | Select-Object -First 1).FullName
$apk=Join-Path $root 'android/app/build/outputs/apk/release/app-release.apk'
$cert=& "$root/.tools/android-sdk/build-tools/35.0.0/apksigner.bat" verify --print-certs $apk
if ($LASTEXITCODE -ne 0 -or ($cert -join "`n") -notmatch '1b44b5384101836bff52967d99f6711ea77bc532d5f6ec01bc82d7d2e5680438') { throw 'APK signature differs from installed release' }
$badging=& "$root/.tools/android-sdk/build-tools/35.0.0/aapt.exe" dump badging $apk
if ($LASTEXITCODE -ne 0 -or ($badging -join "`n") -notmatch "name='local.pocketdrop.android' versionCode='$code' versionName='$([regex]::Escape($version))'" -or ($badging -join "`n") -match 'application-debuggable') { throw 'APK metadata mismatch' }
$output=Join-Path $root "artifacts/android-release-$version"
New-Item -ItemType Directory -Force -Path $output | Out-Null
$name="PocketDrop-$version-Android.apk"
Copy-Item -LiteralPath $apk -Destination (Join-Path $output $name) -Force
$hash=(Get-FileHash -LiteralPath $apk -Algorithm SHA256).Hash.ToLowerInvariant()
$notesPath=Join-Path $root "docs/RELEASE_ANDROID_$version.md"
if (-not (Test-Path -LiteralPath $notesPath)) { throw "Missing Android release notes: $notesPath" }
$manifest=@{versionCode=$code;versionName=$version;minSdk=26;size=(Get-Item -LiteralPath $apk).Length;sha256=$hash;url="https://github.com/OverGreen996/PocketDrop/releases/download/android-v$version/$name";notes=[IO.File]::ReadAllText($notesPath)} | ConvertTo-Json
[IO.File]::WriteAllText((Join-Path $output 'android.json'),$manifest,[Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText((Join-Path $output 'SHA256SUMS.txt'),"$hash  $name`n",[Text.UTF8Encoding]::new($false))
Write-Output $output

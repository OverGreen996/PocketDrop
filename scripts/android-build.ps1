$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$env:JAVA_HOME = (Get-ChildItem "$root/.tools/android-build/jdk" -Directory | Select-Object -First 1).FullName
$env:ANDROID_HOME = "$root/.tools/android-sdk"
$env:ANDROID_USER_HOME = "$root/.tools/android-user"
$env:GRADLE_USER_HOME = "$root/.tools/gradle-cache"
$sdk = $env:ANDROID_HOME.Replace('\','/')
Set-Content -LiteralPath "$root/android/local.properties" -Value "sdk.dir=$sdk" -Encoding ASCII
& "$root/.tools/android-build/gradle-8.11.1/bin/gradle.bat" -p "$root/android" --console=plain assembleRelease lintRelease testReleaseUnitTest
if ($LASTEXITCODE -ne 0) { throw 'Android build failed' }

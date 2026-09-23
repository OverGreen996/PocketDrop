param([ValidateSet('dev','build','test','check')][string]$Action = 'dev')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
if (Test-Path "$root\.tools\cargo\bin\cargo.exe") {
  $env:CARGO_HOME = "$root\.tools\cargo"
  $env:RUSTUP_HOME = "$root\.tools\rustup"
  $env:PATH = "$env:CARGO_HOME\bin;$env:PATH"
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Rust MSVC is required. See README.md.' }
switch ($Action) {
 'test' { cargo test --manifest-path src-tauri/Cargo.toml --locked }
 'check' { cargo check --manifest-path src-tauri/Cargo.toml --locked }
 'build' { npm.cmd run tauri -- build --bundles nsis }
 'dev' { npm.cmd run tauri -- dev }
}
exit $LASTEXITCODE

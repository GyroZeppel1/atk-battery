# Builds atk-battery and verifies it.
# Requires Rust. For the GNU target no Visual Studio is needed - the repo's
# .cargo/config.toml switches to rust-lld with self-contained linking.
$ErrorActionPreference = "Stop"
Push-Location $PSScriptRoot
try {
    $env:CARGO_HOME  = if ($env:CARGO_HOME)  { $env:CARGO_HOME }  else { Join-Path $env:USERPROFILE ".cargo" }
    $env:RUSTUP_HOME = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE ".rustup" }
    $cargo = Join-Path $env:CARGO_HOME "bin\cargo.exe"
    if (-not (Test-Path $cargo)) { $cargo = "cargo" }

    Write-Host "== tests ==" -ForegroundColor Cyan
    & $cargo test --release
    if ($LASTEXITCODE -ne 0) { throw "tests failed" }

    Write-Host "`n== release build ==" -ForegroundColor Cyan
    & $cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "build failed" }

    $exe = Join-Path $PSScriptRoot "target\release\atk-battery.exe"
    $kb  = [math]::Round((Get-Item $exe).Length / 1KB, 1)
    Write-Host "`nbuilt: $exe ($kb KB)" -ForegroundColor Green

}
finally {
    Pop-Location
}

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargo) {
    $userCargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
    if (Test-Path $userCargo) { $cargo = Get-Item $userCargo } else { throw 'Rust ontbreekt. Installeer Rust via https://rustup.rs en Visual Studio Build Tools (Desktop development with C++), en start dit script opnieuw.' }
}
$cargoExe = if ($cargo.Source) { $cargo.Source } else { $cargo.FullName }
Push-Location $projectRoot
try {
    # Gebruik de statisch gelinkte Microsoft C-runtime: geen aparte VC++ runtime installer.
    $env:RUSTFLAGS = '-C target-feature=+crt-static'
    if (-not (Test-Path Cargo.lock)) {
        & $cargoExe generate-lockfile
        if ($LASTEXITCODE -ne 0) { throw 'Dependency-resolutie mislukt.' }
    }
    & $cargoExe test --locked --release --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Rust-tests of compilatie mislukt; geen executable gepubliceerd.' }
    & $cargoExe build --locked --release --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Build mislukt.' }
    $output = Join-Path $projectRoot 'output\Windows-Rust'
    New-Item -ItemType Directory -Force $output | Out-Null
    Copy-Item 'target\x86_64-pc-windows-msvc\release\licentiekluis-native.exe' (Join-Path $output 'LicentieKluis-Rust.exe') -Force
    Copy-Item README.html $output -Force
    $size = (Get-Item (Join-Path $output 'LicentieKluis-Rust.exe')).Length
    Write-Host ('Gebouwd: {0:N2} MB. Bestand: {1}' -f ($size / 1MB), (Join-Path $output 'LicentieKluis-Rust.exe'))
    Write-Host 'Deze Rust-client heeft geen .NET of WebView2 nodig. Test het programma eerst met voorbeeldgegevens.'
} finally { Pop-Location }

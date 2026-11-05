$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
dotnet run --project (Join-Path $root 'Tests\LicenseVault.Tests.csproj') -c Release
if ($LASTEXITCODE -ne 0) { throw 'Tests mislukt.' }
dotnet publish (Join-Path $root 'Server\LicenseVault.Server.csproj') -c Release --self-contained false -p:UseAppHost=false -o (Join-Path $root 'ServerBin')
if ($LASTEXITCODE -ne 0) { throw 'Serverpublicatie mislukt.' }
Write-Host 'ServerBin bijgewerkt; gebruik het LXC-updatescript.'

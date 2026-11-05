param([Parameter(Mandatory=$true)][string]$CertificatePath)
$ErrorActionPreference = 'Stop'
$cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2((Resolve-Path $CertificatePath).Path)
Write-Host ('Certificaat: ' + $cert.Subject)
Write-Host ('SHA-1 vingerafdruk: ' + $cert.Thumbprint)
Write-Host 'Vergelijk dit met openssl x509 -in /root/licentiekluis-ca.crt -noout -fingerprint -sha1 op je LXC.'
if ((Read-Host 'Klopt de vingerafdruk en vertrouw je deze eigen server-CA? Typ JA') -cne 'JA') { throw 'Niet geimporteerd.' }
Import-Certificate -FilePath (Resolve-Path $CertificatePath).Path -CertStoreLocation Cert:\CurrentUser\Root | Out-Null
Write-Host 'Server-CA vertrouwd voor je huidige Windowsaccount.'

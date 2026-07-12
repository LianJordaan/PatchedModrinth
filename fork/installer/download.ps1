param([Parameter(Mandatory = $true)][string]$OutFile)

# Downloads the latest ByteLauncher.exe from GitHub Releases and verifies it
# against the release asset's published SHA-256 digest. Used by the online
# installer; the offline installer bundles the exe instead.

$ErrorActionPreference = 'Stop'
# Windows PowerShell 5.1 may not negotiate TLS 1.2 by default, which api.github.com requires.
try { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 } catch {}
$headers = @{ 'User-Agent' = 'ByteLauncher-Installer' }

$release = Invoke-RestMethod -Headers $headers 'https://api.github.com/repos/LianJordaan/ByteLauncher/releases/latest'

$asset = $release.assets |
    Where-Object { $_.name -match '\.exe$' -and $_.name -notmatch 'Setup|installer' } |
    Select-Object -First 1
if (-not $asset) {
    Write-Error 'No ByteLauncher.exe asset found on the latest release'
    exit 3
}

Invoke-WebRequest -Headers $headers -Uri $asset.browser_download_url -OutFile $OutFile

if ($asset.digest) {
    $want = ($asset.digest -replace '^sha256:', '')
    $got = (Get-FileHash -Algorithm SHA256 -Path $OutFile).Hash
    if ($got -ne $want) {
        Remove-Item -Force $OutFile
        Write-Error 'Downloaded file failed SHA-256 verification'
        exit 4
    }
}

exit 0

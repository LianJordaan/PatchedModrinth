param([Parameter(Mandatory = $true)][string]$OutFile)

# Downloads the latest ByteLauncher.exe from GitHub's stable non-API release CDN
# (releases/latest/download — NOT the rate-limited api.github.com). Streams in
# chunks and prints progress lines so the installer's details view shows the
# download actually happening. Verifies a published SHA-256 sidecar when present.

$ErrorActionPreference = 'Stop'
# Windows PowerShell 5.1 may not negotiate TLS 1.2 by default, which GitHub requires.
try { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 } catch {}

# Write straight to stdout (and flush) so nsExec::ExecToLog streams each line to
# the installer's details window in real time — Write-Host is not captured.
function Write-Line([string]$msg) {
    [Console]::Out.WriteLine($msg)
    [Console]::Out.Flush()
}

$base = 'https://github.com/LianJordaan/ByteLauncher/releases/latest/download'

Write-Line 'Connecting to GitHub...'
$req = [System.Net.HttpWebRequest]::Create("$base/ByteLauncher.exe")
$req.UserAgent = 'ByteLauncher-Installer'
$req.AllowAutoRedirect = $true
$resp = $req.GetResponse()
$total = [int64]$resp.ContentLength
$totalMB = if ($total -gt 0) { [math]::Round($total / 1MB, 1) } else { 0 }
$stream = $resp.GetResponseStream()
$fs = [System.IO.File]::Create($OutFile)
try {
    $buffer = New-Object byte[] 262144
    [int64]$done = 0
    $lastPct = -100
    while (($read = $stream.Read($buffer, 0, $buffer.Length)) -gt 0) {
        $fs.Write($buffer, 0, $read)
        $done += $read
        if ($total -gt 0) {
            $pct = [int](($done / $total) * 100)
            if ($pct -ge $lastPct + 5) {
                $lastPct = $pct
                Write-Line ("Downloading ByteLauncher...  {0}%  ({1} / {2} MB)" -f $pct, [math]::Round($done / 1MB, 1), $totalMB)
            }
        }
        elseif (($done % (2MB)) -lt $buffer.Length) {
            Write-Line ("Downloading ByteLauncher...  {0} MB" -f [math]::Round($done / 1MB, 1))
        }
    }
}
finally {
    $fs.Close(); $stream.Close(); $resp.Close()
}
Write-Line ("Download complete ({0} MB). Verifying..." -f [math]::Round($done / 1MB, 1))

$expected = $null
try {
    $shaFile = "$OutFile.sha256"
    Invoke-WebRequest -UseBasicParsing -Uri "$base/ByteLauncher.exe.sha256" -OutFile $shaFile
    $expected = ((Get-Content -Raw -Path $shaFile) -split '\s+')[0].Trim().ToLower()
}
catch {
    $expected = $null
}

if ($expected) {
    $actual = (Get-FileHash -Algorithm SHA256 -Path $OutFile).Hash.ToLower()
    if ($actual -ne $expected) {
        Remove-Item -Force $OutFile
        Write-Line 'Downloaded file failed SHA-256 verification.'
        exit 4
    }
    Write-Line 'Checksum verified.'
}

exit 0

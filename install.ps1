$ErrorActionPreference = "Stop"

$url = "https://github.com/ThriceCola/hi-my-light/releases/latest/download/hi-my-light-windows-amd64.exe"
$dest = Join-Path (Get-Location) "hi-my-light.exe"

Invoke-WebRequest -Uri $url -OutFile $dest
Start-Process -FilePath $dest

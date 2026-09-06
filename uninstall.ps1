$ErrorActionPreference = "Stop"

if (Get-Process -Name "hi-my-light" -ErrorAction SilentlyContinue) {
    Write-Host "请先关闭 Hi My Light，再重新运行卸载"
    exit 1
}

$programs = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$lnk = Join-Path $programs "HML.lnk"
$bat = Join-Path $programs "HML.bat"

$exe = $null
if (Test-Path $lnk) {
    $exe = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk).TargetPath
} elseif (Test-Path $bat) {
    if ((Get-Content $bat -Raw) -match 'start "" "(.+)"') {
        $exe = $Matches[1]
    }
}

if ($exe) {
    $dir = Split-Path $exe -Parent
    Remove-Item $exe -Force -ErrorAction SilentlyContinue
    Remove-Item (Join-Path $dir "hi-my-light.ico") -Force -ErrorAction SilentlyContinue
}

Remove-Item $lnk -Force -ErrorAction SilentlyContinue
Remove-Item $bat -Force -ErrorAction SilentlyContinue
Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "hi-my-light" -Force -ErrorAction SilentlyContinue

Write-Host "已卸载 Hi My Light"

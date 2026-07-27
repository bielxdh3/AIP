param([string]$Python = "python")

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$output = Join-Path $root "apps\desktop\src-tauri\binaries"
& $Python -m PyInstaller --onefile --noconfirm --clean --name "aip-runtime" --paths (Join-Path $root "services\runtime\src") --distpath $output --workpath (Join-Path $root ".runtime-build") --specpath (Join-Path $root ".runtime-build") (Join-Path $root "services\runtime\src\aip_runtime\__main__.py")
Move-Item -Force (Join-Path $output "aip-runtime.exe") (Join-Path $output "aip-runtime-x86_64-pc-windows-msvc.exe")

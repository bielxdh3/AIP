param([string]$Python = "python")

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$output = Join-Path $root "apps\desktop\src-tauri\binaries"
Remove-Item -Force (Join-Path $output "aip-runtime-x86_64-pc-windows-msvc.exe") -ErrorAction SilentlyContinue
& $Python -m PyInstaller --onefile --noconfirm --clean --name "aip-runtime" --paths (Join-Path $root "services\runtime\src") --distpath $output --workpath (Join-Path $root ".runtime-build") --specpath (Join-Path $root ".runtime-build") (Join-Path $root "services\runtime\aip_runtime_entry.py")
$runtime = Join-Path $output "aip-runtime.exe"
$process = [Diagnostics.Process]::new()
$process.StartInfo = [Diagnostics.ProcessStartInfo]::new($runtime, "--stdio")
$process.StartInfo.UseShellExecute = $false
$process.StartInfo.RedirectStandardInput = $true
$process.StartInfo.RedirectStandardOutput = $true
$process.StartInfo.RedirectStandardError = $true
$started = [Diagnostics.Stopwatch]::StartNew()
if (-not $process.Start()) { throw "Could not start generated runtime" }
$stderrTask = $process.StandardError.ReadToEndAsync()
$process.StandardInput.WriteLine('{"protocolVersion":1,"id":"sidecar-health","method":"runtime.health","params":{}}')
$process.StandardInput.Flush()
$healthTask = $process.StandardOutput.ReadLineAsync()
if (-not $healthTask.Wait(10000)) {
    $process.Kill()
    $process.WaitForExit()
    throw "Runtime health response timed out: $($stderrTask.Result)"
}
$line = $healthTask.Result
if (-not $line) { throw "Runtime emitted no health response: $($stderrTask.Result)" }
try { $health = $line | ConvertFrom-Json } catch { throw "Runtime emitted malformed health JSON: $line" }
if ($health.protocolVersion -ne 1 -or $health.id -ne "sidecar-health" -or $health.result.status -ne "ready" -or $health.result.protocolVersion -ne 1) { throw "Runtime health response did not match protocol or request ID" }
$process.StandardInput.WriteLine('{"protocolVersion":1,"id":"sidecar-shutdown","method":"runtime.shutdown","params":{}}')
$process.StandardInput.Flush()
if (-not $process.WaitForExit(10000)) { $process.Kill(); throw "Runtime shutdown timed out" }
$stderr = $stderrTask.Result
if ($process.ExitCode -ne 0 -or ($stderr.Trim() -and $stderr.Trim() -ne "AIP_RUNTIME_DIAGNOSTIC runtime_shutdown_requested")) { throw "Runtime smoke test failed (exit $($process.ExitCode)): $stderr" }
Write-Output "Sidecar smoke OK: $runtime; $($started.ElapsedMilliseconds)ms; $((Get-FileHash $runtime -Algorithm SHA256).Hash)"
Move-Item -Force $runtime (Join-Path $output "aip-runtime-x86_64-pc-windows-msvc.exe")

param(
  [string]$MsiPath,
  [switch]$SkipBuild,
  [switch]$KeepArtifacts
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$bundleRoot = Join-Path $root "apps\desktop\src-tauri\target\release\bundle"
$expectedMsiName = "A.I.P._0.2.3.4_x64_en-US.msi"
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("aip-installed-smoke-" + [guid]::NewGuid().ToString("N"))
$readyFile = Join-Path $tempRoot "fixture-port.txt"
$receiptFile = Join-Path $tempRoot "fixture-receipts.ndjson"
$installLog = Join-Path $tempRoot "msiexec-install.log"
$installDir = Join-Path $tempRoot "installed"
$fixture = $null
$runtime = $null
$installedMsi = $null
$installCompleted = $false
$oldOllamaHost = [Environment]::GetEnvironmentVariable("OLLAMA_HOST", "Process")
$oldOllamaExecutable = [Environment]::GetEnvironmentVariable("AIP_OLLAMA_EXECUTABLE", "Process")
$oldOllamaConfig = [Environment]::GetEnvironmentVariable("AIP_OLLAMA_CONFIG", "Process")
$oldPythonIoEncoding = [Environment]::GetEnvironmentVariable("PYTHONIOENCODING", "Process")

function Invoke-CheckedProcess {
  param([string]$FilePath, [string[]]$ArgumentList, [int[]]$AllowedExitCodes = @(0))
  $process = Start-Process -FilePath $FilePath -ArgumentList $ArgumentList -Wait -PassThru -NoNewWindow
  if ($AllowedExitCodes -notcontains $process.ExitCode) {
    throw "$FilePath failed with exit code $($process.ExitCode)"
  }
  return $process.ExitCode
}

function Resolve-Msi {
  if ($MsiPath) {
    $resolved = (Resolve-Path -LiteralPath $MsiPath).Path
    if ([IO.Path]::GetFileName($resolved) -ne $expectedMsiName) {
      throw "Installed smoke requires the exact MSI filename $expectedMsiName; got $([IO.Path]::GetFileName($resolved))"
    }
    return $resolved
  }
  $expected = Join-Path $bundleRoot "msi\$expectedMsiName"
  if ($SkipBuild) {
    if (Test-Path -LiteralPath $expected) { return (Resolve-Path -LiteralPath $expected).Path }
    throw "No $expectedMsiName found and -SkipBuild was supplied"
  }

  & (Join-Path $root "scripts\build-runtime.ps1")
  if ($LASTEXITCODE -ne 0) { throw "Runtime packaging failed" }
  & pnpm --filter @aip/desktop tauri build
  if ($LASTEXITCODE -ne 0) { throw "Tauri packaging failed" }
  $source = Get-ChildItem -LiteralPath (Join-Path $bundleRoot "msi") -Filter "*.msi" -File |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
  if (-not $source) { throw "Tauri packaging did not produce an MSI" }
  if ($source.FullName -ne $expected) {
    Copy-Item -LiteralPath $source.FullName -Destination $expected -Force
  }
  return (Resolve-Path -LiteralPath $expected).Path
}

function Read-JsonLine {
  param([Diagnostics.Process]$Process, [int]$TimeoutMs = 15000)
  $task = $Process.StandardOutput.ReadLineAsync()
  if (-not $task.Wait($TimeoutMs)) { throw "Installed runtime did not emit a protocol line within ${TimeoutMs}ms" }
  $line = $task.Result
  if ([string]::IsNullOrWhiteSpace($line)) { throw "Installed runtime stdout closed before the expected protocol line" }
  try { return ($line | ConvertFrom-Json -Depth 12) } catch { throw "Installed runtime emitted malformed JSON: $line" }
}

function Send-Json {
  param([Diagnostics.Process]$Process, [hashtable]$Value)
  $Process.StandardInput.WriteLine(($Value | ConvertTo-Json -Compress -Depth 12))
  $Process.StandardInput.Flush()
}

function Wait-Generation {
  param(
    [Diagnostics.Process]$Process,
    [string]$RequestId,
    [string]$ExpectedOutput,
    [int]$MinimumChunks = 1
  )
  $accepted = $false
  $started = $false
  $chunks = [Collections.Generic.List[string]]::new()
  $sequences = [Collections.Generic.List[int]]::new()
  $terminal = $null
  $deadline = [Diagnostics.Stopwatch]::StartNew()
  while ($deadline.Elapsed.TotalSeconds -lt 30) {
    $message = Read-JsonLine -Process $Process -TimeoutMs 5000
    if ($message.id -eq $RequestId -and $message.result.status -eq "accepted") {
      $accepted = $true
      continue
    }
    if ($message.requestId -ne $RequestId) { continue }
    if ($message.event -eq "generation.started") { $started = $true }
    if ($message.event -eq "generation.chunk") {
      [void]$chunks.Add([string]$message.content)
      [void]$sequences.Add([int]$message.sequence)
    }
    if ($message.event -in @("generation.complete", "generation.failed", "generation.cancelled")) {
      $terminal = [string]$message.event
      break
    }
  }
  if (-not $accepted -or -not $started -or $terminal -ne "generation.complete") {
    throw "Installed generation $RequestId did not complete (accepted=$accepted started=$started terminal=$terminal)"
  }
  if ($chunks.Count -lt $MinimumChunks) {
    throw "Installed generation $RequestId returned only $($chunks.Count) chunks; expected at least $MinimumChunks"
  }
  if ($sequences.Count -gt 0) {
    for ($index = 0; $index -lt $sequences.Count; $index++) {
      if ($sequences[$index] -ne ($index + 1)) { throw "Installed generation $RequestId emitted a non-contiguous chunk sequence" }
    }
  }
  $output = $chunks -join ""
  if ($output -ne $ExpectedOutput) { throw "Installed generation $RequestId returned an unexpected fixture output" }
  return [pscustomobject]@{ RequestId = $RequestId; Output = $output; ChunkCount = $chunks.Count }
}

function Read-TraceEvents {
  param([string]$Stderr)
  $prefix = "AIP_RUNTIME_TRACE "
  $events = [Collections.Generic.List[object]]::new()
  foreach ($line in ($Stderr -split "\r?\n")) {
    if (-not $line.StartsWith($prefix, [StringComparison]::Ordinal)) { continue }
    try {
      $event = $line.Substring($prefix.Length) | ConvertFrom-Json -Depth 12
    } catch {
      throw "Installed runtime emitted malformed trace JSON"
    }
    if (-not $event.requestId -or -not $event.event) {
      throw "Installed runtime emitted an uncorrelated trace event"
    }
    [void]$events.Add($event)
  }
  return @($events)
}

function Get-FixtureExpectedOutput {
  param([string]$Marker)
  if ($Marker -eq "marker-one") { return "AIP-STREAM-TEST-OK" }
  if ($Marker -in @("marker-two", "marker-three")) {
    $suffix = (0..127 | ForEach-Object { "{0:D3}," -f $_ }) -join ""
    return "fixture-response:$Marker|$suffix"
  }
  return "fixture-response:$Marker"
}

try {
  New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null
  $installedMsi = Resolve-Msi
  if ([IO.Path]::GetFileName($installedMsi) -ne $expectedMsiName) { throw "MSI identity mismatch" }
  $msiHash = (Get-FileHash -LiteralPath $installedMsi -Algorithm SHA256).Hash
  if (-not $msiHash) { throw "MSI SHA256 was not available" }

  $fixture = Start-Process -FilePath "python" -ArgumentList @(
    "-u", (Join-Path $root "scripts\fake-ollama-server.py"),
    "--ready-file", $readyFile,
    "--receipt", $receiptFile
  ) -PassThru -WindowStyle Hidden
  $fixtureDeadline = [Diagnostics.Stopwatch]::StartNew()
  while (-not (Test-Path -LiteralPath $readyFile) -and $fixtureDeadline.Elapsed.TotalSeconds -lt 10) {
    Start-Sleep -Milliseconds 50
  }
  if (-not (Test-Path -LiteralPath $readyFile)) { throw "Fake Ollama fixture did not become ready" }
  $port = [int](Get-Content -LiteralPath $readyFile -Raw)
  $env:OLLAMA_HOST = "127.0.0.1:$port"
  $env:PYTHONIOENCODING = "utf-8"
  Remove-Item Env:AIP_OLLAMA_EXECUTABLE -ErrorAction SilentlyContinue
  Remove-Item Env:AIP_OLLAMA_CONFIG -ErrorAction SilentlyContinue

  $installResult = Invoke-CheckedProcess "msiexec.exe" @("/i", $installedMsi, "INSTALLDIR=$installDir", "/qn", "/norestart", "/l*v", $installLog) @(0, 3010)
  $installCompleted = $true
  $candidatePaths = @(
    (Join-Path $installDir "aip-desktop.exe"),
    (Join-Path $installDir "aip-runtime.exe"),
    (Join-Path $env:ProgramFiles "A.I.P.\aip-desktop.exe"),
    (Join-Path $env:ProgramFiles "A.I.P.\aip-runtime.exe"),
    (Join-Path $env:LOCALAPPDATA "Programs\A.I.P.\aip-desktop.exe"),
    (Join-Path $env:LOCALAPPDATA "Programs\A.I.P.\aip-runtime.exe")
  )
  $installedDesktop = $candidatePaths | Where-Object { $_ -like "*aip-desktop.exe" -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
  $installedRuntime = $candidatePaths | Where-Object { $_ -like "*aip-runtime.exe" -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
  if (-not $installedDesktop -or -not $installedRuntime) {
    $searchRoots = @($env:ProgramFiles, $env:LOCALAPPDATA) | Where-Object { $_ -and (Test-Path -LiteralPath $_) }
    $installedDesktop = $searchRoots | ForEach-Object { Get-ChildItem -LiteralPath $_ -Filter "aip-desktop.exe" -File -Recurse -ErrorAction SilentlyContinue } | Select-Object -First 1 -ExpandProperty FullName
    $installedRuntime = $searchRoots | ForEach-Object { Get-ChildItem -LiteralPath $_ -Filter "aip-runtime.exe" -File -Recurse -ErrorAction SilentlyContinue } | Select-Object -First 1 -ExpandProperty FullName
  }
  if (-not $installedDesktop -or -not $installedRuntime) { throw "The MSI did not install both aip-desktop.exe and aip-runtime.exe" }
  $repoFullPath = (Resolve-Path -LiteralPath $root).Path.TrimEnd("\") + "\"
  foreach ($path in @($installedDesktop, $installedRuntime)) {
    $fullPath = (Resolve-Path -LiteralPath $path).Path
    if ($fullPath.StartsWith($repoFullPath, [StringComparison]::OrdinalIgnoreCase)) { throw "Installed smoke resolved a repository binary instead of an installed binary: $fullPath" }
  }

  $identity = (& $installedDesktop --print-build-identity 2>&1 | Out-String).Trim()
  if ($LASTEXITCODE -ne 0 -or $identity -ne "0.2.3.4") { throw "Installed desktop reported build identity '$identity' instead of 0.2.3.4" }

  $startInfo = [Diagnostics.ProcessStartInfo]::new()
  $startInfo.FileName = $installedRuntime
  $startInfo.Arguments = "--stdio"
  $startInfo.UseShellExecute = $false
  $startInfo.CreateNoWindow = $true
  $startInfo.RedirectStandardInput = $true
  $startInfo.RedirectStandardOutput = $true
  $startInfo.RedirectStandardError = $true
  $runtime = [Diagnostics.Process]::new()
  $runtime.StartInfo = $startInfo
  if (-not $runtime.Start()) { throw "Installed runtime failed to start" }
  $stderrTask = $runtime.StandardError.ReadToEndAsync()

  Send-Json $runtime @{ protocolVersion = 1; id = "smoke-health"; method = "runtime.health"; params = @{} }
  $health = Read-JsonLine $runtime
  if ($health.id -ne "smoke-health" -or $health.result.status -ne "ready") { throw "Installed runtime health handshake failed" }
  Send-Json $runtime @{ protocolVersion = 1; id = "smoke-discover"; method = "provider.discover"; params = @{} }
  $discover = Read-JsonLine $runtime
  if ($discover.id -ne "smoke-discover" -or $discover.result.models[0].providerModelId -ne "fixture:latest") { throw "Installed provider discovery did not reach the fixture" }

  $outputs = @()
  $requests = @(
    @("generation-one", "marker-one", 1),
    @("generation-two", "marker-two", 100),
    @("generation-three", "marker-three", 100)
  )
  foreach ($item in $requests) {
    $requestId = $item[0]
    $marker = $item[1]
    Send-Json $runtime @{
      protocolVersion = 1
      id = $requestId
      method = "generation.start"
      params = @{
        agentId = "smoke-agent"
        conversationId = "smoke-conversation"
        assistantMessageId = "smoke-$requestId"
        model = "fixture:latest"
        keepAliveMinutes = 0
        messages = @(@{ role = "user"; content = "installed $marker" })
      }
    }
    $expected = Get-FixtureExpectedOutput $marker
    $result = Wait-Generation $runtime $requestId $expected ([int]$item[2])
    $outputs += $result
    Send-Json $runtime @{ protocolVersion = 1; id = "health-$requestId"; method = "runtime.health"; params = @{} }
    $health = Read-JsonLine $runtime
    if ($health.id -ne "health-$requestId" -or $health.result.status -ne "ready") {
      throw "Installed runtime liveness check failed after $requestId"
    }
  }
  $receipts = @(Get-Content -LiteralPath $receiptFile | ForEach-Object { $_ | ConvertFrom-Json })
  if ($receipts.Count -ne 3 -or $receipts.marker -notcontains "marker-one" -or $receipts.marker -notcontains "marker-two" -or $receipts.marker -notcontains "marker-three") {
    throw "Fake Ollama did not receive exactly the three distinct generation requests"
  }
  if (($receipts | Where-Object marker -eq "marker-two").chunkCount -lt 100 -or ($receipts | Where-Object marker -eq "marker-three").chunkCount -lt 100) {
    throw "Fake Ollama did not emit the required 100-plus chunk streams"
  }

  Send-Json $runtime @{ protocolVersion = 1; id = "smoke-shutdown"; method = "runtime.shutdown"; params = @{} }
  if (-not $runtime.WaitForExit(10000)) { throw "Installed runtime shutdown timed out" }
  if ($runtime.ExitCode -ne 0) { throw "Installed runtime exited with code $($runtime.ExitCode)" }
  $stderr = $stderrTask.Result
  $traceEvents = @(Read-TraceEvents $stderr)
  $requiredTraceEvents = @("generation.accepted", "ollama.request.started", "provider.stream.started", "ollama.connected", "ollama.first_chunk", "provider.stream.completed", "runtime.terminal.emitted", "ollama.request.completed")
  $accountingTraceEvents = @("provider.stream.completed", "runtime.terminal.emitted", "ollama.request.completed")
  foreach ($result in $outputs) {
    $requestTrace = @($traceEvents | Where-Object { $_.requestId -eq $result.RequestId })
    if ($requestTrace.Count -eq 0) {
      throw "Installed runtime emitted no trace events for $($result.RequestId)"
    }
    foreach ($traceEvent in $requiredTraceEvents) {
      if (@($requestTrace | Where-Object { $_.event -eq $traceEvent }).Count -eq 0) {
        throw "Installed runtime trace for $($result.RequestId) is missing $traceEvent"
      }
    }
    $expectedBytes = [Text.Encoding]::UTF8.GetByteCount([string]$result.Output)
    $expectedCharacters = ([string]$result.Output).Length
    foreach ($traceEvent in $accountingTraceEvents) {
      $accounted = @($requestTrace | Where-Object { $_.event -eq $traceEvent }) | Select-Object -Last 1
      $counters = $accounted.counters
      $expectedTerminalEvents = if ($traceEvent -eq "provider.stream.completed") { 0 } else { 1 }
      if ($null -eq $counters) {
        throw "Installed runtime trace for $($result.RequestId) is missing counters on $traceEvent"
      }
      foreach ($counterName in @("provider_chunks", "provider_bytes", "provider_characters", "runtime_chunks", "runtime_bytes", "runtime_characters", "runtime_terminal_events")) {
        if ($null -eq $counters.$counterName) {
          throw "Installed runtime trace for $($result.RequestId) is missing $counterName on $traceEvent"
        }
      }
      if ([int64]$counters.provider_chunks -ne [int64]$result.ChunkCount -or
        [int64]$counters.runtime_chunks -ne [int64]$result.ChunkCount -or
        [int64]$counters.provider_bytes -ne [int64]$expectedBytes -or
        [int64]$counters.runtime_bytes -ne [int64]$expectedBytes -or
        [int64]$counters.provider_characters -ne [int64]$expectedCharacters -or
        [int64]$counters.runtime_characters -ne [int64]$expectedCharacters -or
        [int64]$counters.runtime_terminal_events -ne $expectedTerminalEvents) {
        throw "Installed runtime trace counters for $($result.RequestId) do not match the reconstructed response"
      }
    }
  }
  Write-Output "Installed MSI smoke OK: $installedMsi (SHA256 $msiHash)"
  Write-Output "Installed binaries: $installedDesktop; $installedRuntime"
  Write-Output "Fixture receipts: $($receipts.Count); chunks: $(($outputs | ForEach-Object ChunkCount) -join ', ')"
}
finally {
  if ($runtime -and -not $runtime.HasExited) {
    try { $runtime.Kill() } catch { }
    try { $runtime.WaitForExit(2000) } catch { }
  }
  if ($fixture -and -not $fixture.HasExited) {
    try { $fixture.Kill() } catch { }
    try { $fixture.WaitForExit(2000) } catch { }
  }
  [Environment]::SetEnvironmentVariable("OLLAMA_HOST", $oldOllamaHost, "Process")
  [Environment]::SetEnvironmentVariable("AIP_OLLAMA_EXECUTABLE", $oldOllamaExecutable, "Process")
  [Environment]::SetEnvironmentVariable("AIP_OLLAMA_CONFIG", $oldOllamaConfig, "Process")
  [Environment]::SetEnvironmentVariable("PYTHONIOENCODING", $oldPythonIoEncoding, "Process")
  if ($installCompleted -and $installedMsi -and -not $KeepArtifacts -and (Test-Path -LiteralPath $installedMsi)) {
    try { Invoke-CheckedProcess "msiexec.exe" @("/x", $installedMsi, "/qn", "/norestart") @(0, 3010) | Out-Null } catch { Write-Warning "MSI cleanup failed: $_" }
  }
  if (-not $KeepArtifacts -and (Test-Path -LiteralPath $tempRoot)) {
    Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
  } elseif (Test-Path -LiteralPath $tempRoot) {
    Write-Output "Smoke artifacts retained at $tempRoot"
  }
}

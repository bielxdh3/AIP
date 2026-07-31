import { execFileSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { performance } from "node:perf_hooks";

const endpoint = process.env.AIP_OLLAMA_URL ?? "http://127.0.0.1:11434";
const defaultPrompt =
  "Responda em uma frase, em português: memória curta é temporária e memória longa é persistente.";

function argument(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

function powershellJson(command, fallback) {
  try {
    const output = execFileSync(
      "powershell",
      ["-NoProfile", "-Command", command],
      {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
      },
    ).trim();
    return output ? JSON.parse(output) : fallback;
  } catch {
    return fallback;
  }
}

function numeric(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function processSnapshot() {
  const processes = powershellJson(
    "Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -like 'ollama*' } | Select-Object ProcessName,Id,WorkingSet64,PeakWorkingSet64,CPU | ConvertTo-Json -Compress",
    [],
  );
  const list = Array.isArray(processes)
    ? processes
    : processes
      ? [processes]
      : [];
  return {
    processCount: list.length,
    workingSetBytes: list.reduce(
      (sum, process) => sum + (numeric(process.WorkingSet64) ?? 0),
      0,
    ),
    peakWorkingSetBytes: list.reduce(
      (sum, process) => sum + (numeric(process.PeakWorkingSet64) ?? 0),
      0,
    ),
    cpuSeconds: list.reduce(
      (sum, process) => sum + (numeric(process.CPU) ?? 0),
      0,
    ),
  };
}

function hardwareProfile() {
  const cpu = powershellJson(
    "Get-CimInstance Win32_Processor | Select-Object Name,NumberOfCores,NumberOfLogicalProcessors | ConvertTo-Json -Compress",
    null,
  );
  const memory = powershellJson(
    "Get-CimInstance Win32_OperatingSystem | Select-Object TotalVisibleMemorySize,FreePhysicalMemory | ConvertTo-Json -Compress",
    null,
  );
  const graphics = powershellJson(
    "Get-CimInstance Win32_VideoController | Select-Object Name,AdapterRAM,DriverVersion | ConvertTo-Json -Compress",
    null,
  );
  return {
    cpu: cpu
      ? {
          name: String(cpu.Name ?? "unknown").trim(),
          cores: numeric(cpu.NumberOfCores),
          logicalProcessors: numeric(cpu.NumberOfLogicalProcessors),
        }
      : null,
    memory: memory
      ? {
          totalBytes: numeric(memory.TotalVisibleMemorySize)
            ? memory.TotalVisibleMemorySize * 1024
            : null,
          freeBytesAtStart: numeric(memory.FreePhysicalMemory)
            ? memory.FreePhysicalMemory * 1024
            : null,
        }
      : null,
    graphics: (Array.isArray(graphics)
      ? graphics
      : graphics
        ? [graphics]
        : []
    ).map((device) => ({
      name: String(device.Name ?? "unknown").trim(),
      adapterRamBytes: numeric(device.AdapterRAM),
      driverVersion: device.DriverVersion ?? null,
    })),
    vram: {
      status: "not_available",
      bytes: null,
      note: "No supported dedicated-GPU telemetry was available to this preliminary harness.",
    },
  };
}

async function request(path, body) {
  const response = await fetch(`${endpoint}${path}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) throw new Error(`http_${response.status}`);
  return response;
}

async function unload(model) {
  try {
    await request("/api/generate", {
      model,
      prompt: "",
      stream: false,
      keep_alive: 0,
    });
  } catch {
    // A failed unload must not hide a benchmark result.
  }
}

function bytesFromOllamaSize(value) {
  const match = /^(\d+(?:\.\d+)?)\s*(B|KB|MB|GB|TB)$/i.exec(value ?? "");
  if (!match) return null;
  const units = { B: 0, KB: 1, MB: 2, GB: 3, TB: 4 };
  const exponent = units[match[2].toUpperCase()];
  return exponent === undefined
    ? null
    : Math.round(Number(match[1]) * 1024 ** exponent);
}

function ollamaRuntime(model) {
  try {
    const lines = execFileSync("ollama", ["ps"], { encoding: "utf8" })
      .trim()
      .split(/\r?\n/)
      .filter(Boolean);
    const line = lines
      .slice(1)
      .find((candidate) => candidate.startsWith(model));
    if (!line)
      return {
        status: "not_reported",
        processor: null,
        loadedModelBytes: null,
      };
    const columns = line.trim().split(/\s{2,}/);
    return {
      status: "reported",
      processor: columns[3] ?? null,
      loadedModelBytes: bytesFromOllamaSize(columns[2]),
    };
  } catch {
    return { status: "unavailable", processor: null, loadedModelBytes: null };
  }
}

async function benchmark(model, prompt) {
  await unload(model);
  const before = processSnapshot();
  const started = performance.now();
  let firstContentMs = null;
  let terminal = null;

  try {
    const response = await request("/api/chat", {
      model,
      messages: [{ role: "user", content: prompt }],
      stream: true,
      keep_alive: "10m",
      think: false,
      options: { temperature: 0, num_predict: 64 },
    });
    if (!response.body) throw new Error("missing_stream");
    const reader = response.body
      .pipeThrough(new TextDecoderStream())
      .getReader();
    let buffered = "";
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      buffered += value;
      const lines = buffered.split("\n");
      buffered = lines.pop() ?? "";
      for (const line of lines) {
        if (!line.trim()) continue;
        const event = JSON.parse(line);
        if (firstContentMs === null && event.message?.content) {
          firstContentMs = performance.now() - started;
        }
        if (event.done) terminal = event;
      }
    }
    if (buffered.trim()) {
      const event = JSON.parse(buffered);
      if (firstContentMs === null && event.message?.content)
        firstContentMs = performance.now() - started;
      if (event.done) terminal = event;
    }
    if (!terminal) throw new Error("missing_terminal_event");
    const elapsedMs = performance.now() - started;
    const after = processSnapshot();
    const evalDurationNs = numeric(terminal.eval_duration);
    const evalCount = numeric(terminal.eval_count);
    return {
      model,
      status: "completed",
      elapsedMs: Math.round(elapsedMs),
      firstContentMs:
        firstContentMs === null ? null : Math.round(firstContentMs),
      loadMs:
        numeric(terminal.load_duration) === null
          ? null
          : Math.round(terminal.load_duration / 1_000_000),
      promptTokens: numeric(terminal.prompt_eval_count),
      generatedTokens: evalCount,
      generationMs:
        evalDurationNs === null ? null : Math.round(evalDurationNs / 1_000_000),
      tokensPerSecond:
        evalCount === null || evalDurationNs === null || evalDurationNs <= 0
          ? null
          : Number((evalCount / (evalDurationNs / 1_000_000_000)).toFixed(2)),
      runtime: ollamaRuntime(model),
      cpuSecondsDelta: Number(
        Math.max(0, after.cpuSeconds - before.cpuSeconds).toFixed(3),
      ),
      ram: {
        ollamaWorkingSetBytesBefore: before.workingSetBytes,
        ollamaWorkingSetBytesAfter: after.workingSetBytes,
        ollamaPeakWorkingSetBytesAfter: after.peakWorkingSetBytes,
      },
      vram: { status: "not_available", bytes: null },
    };
  } catch (error) {
    return {
      model,
      status: "failed",
      errorClass: error instanceof Error ? error.message : "unknown_error",
      runtime: ollamaRuntime(model),
      vram: { status: "not_available", bytes: null },
    };
  } finally {
    await unload(model);
  }
}

const tagsResponse = await fetch(`${endpoint}/api/tags`);
if (!tagsResponse.ok)
  throw new Error(
    `Unable to discover local Ollama models: http_${tagsResponse.status}`,
  );
const tags = await tagsResponse.json();
const requestedModels = [];
for (let index = 2; index < process.argv.length; index += 1) {
  const value = process.argv[index];
  if (value === "--output") {
    index += 1;
  } else if (value && !value.startsWith("--")) {
    requestedModels.push(value);
  }
}
const models =
  requestedModels.length > 0
    ? requestedModels
    : tags.models.map((model) => model.name);
const output = resolve(
  argument("--output") ??
    `benchmarks/preliminary-${new Date().toISOString().slice(0, 10)}.json`,
);
const result = {
  schemaVersion: 1,
  kind: "preliminary_local_hardware",
  createdAt: new Date().toISOString(),
  endpoint: "local_ollama",
  promptPolicy:
    "One fixed Portuguese prompt, reasoning disabled when supported, and a 64-token output cap; response content is never recorded.",
  hardware: hardwareProfile(),
  models: [],
};

for (const model of models) {
  result.models.push(await benchmark(model, defaultPrompt));
}

await mkdir(dirname(output), { recursive: true });
await writeFile(output, `${JSON.stringify(result, null, 2)}\n`, "utf8");
console.log(
  JSON.stringify({
    output,
    models: result.models.map(({ model, status }) => ({ model, status })),
  }),
);

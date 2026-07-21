import { execFileSync } from "node:child_process";
import { readFileSync, statSync } from "node:fs";
import { basename } from "node:path";

const listed = execFileSync(
  "git",
  ["ls-files", "--cached", "--others", "--exclude-standard", "-z"],
  { encoding: "utf8" },
);

const files = listed.split("\0").filter(Boolean);
const findings = [];
const forbiddenNames = [
  /^\.env(?:\..+)?$/i,
  /\.(?:pem|p12|pfx|sqlite|sqlite3|db|dump|gguf|safetensors)$/i,
];
const contentRules = [
  ["private key", /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/],
  ["GitHub token", /(?:ghp|github_pat)_[A-Za-z0-9_]{20,}/],
  ["AWS access key", /\b(?:AKIA|ASIA)[A-Z0-9]{16}\b/],
  ["Google API key", /\bAIza[0-9A-Za-z_-]{30,}\b/],
  ["Slack token", /\bxox[baprs]-[0-9A-Za-z-]{10,}\b/],
  ["live secret key", /\bsk_(?:live|test)_[0-9A-Za-z]{16,}\b/],
  [
    "credential assignment",
    /\b(?:API_KEY|AUTH_TOKEN|ACCESS_TOKEN|CLIENT_SECRET)\s*[:=]\s*["'][^"']{8,}["']/i,
  ],
  [
    "personal Windows path",
    /[A-Za-z]:\\Users\\(?!<user>|USERNAME|Public)[^\\\s]+\\/i,
  ],
];

for (const file of files) {
  if (forbiddenNames.some((pattern) => pattern.test(basename(file)))) {
    findings.push(`${file}: forbidden file type`);
    continue;
  }

  const stats = statSync(file);
  if (!stats.isFile() || stats.size > 2_000_000) {
    continue;
  }

  const buffer = readFileSync(file);
  if (buffer.includes(0)) {
    continue;
  }

  const content = buffer.toString("utf8");
  for (const [name, pattern] of contentRules) {
    if (pattern.test(content)) {
      findings.push(`${file}: ${name}`);
    }
  }
}

if (findings.length > 0) {
  console.error("Secret/privacy scan failed:");
  for (const finding of findings) {
    console.error(`- ${finding}`);
  }
  process.exit(1);
}

console.log(
  `Secret/privacy scan passed (${files.length} repository files checked).`,
);

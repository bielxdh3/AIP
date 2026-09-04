import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPOSITORY_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const ACTIVE_DEVELOPMENT_VERSION = "0.2.1";
const SEMVER = /^\d+\.\d+\.\d+$/;

function jsonVersion(path, readFile) {
  const manifest = JSON.parse(readFile(path));
  if (typeof manifest.version !== "string") {
    throw new Error(`${path} must declare a string version`);
  }
  return manifest.version;
}

function tomlVersion(path, readFile) {
  const match = readFile(path).match(/^\s*version\s*=\s*"([^"]+)"\s*$/m);
  if (!match) {
    throw new Error(`${path} does not declare a package version`);
  }
  return match[1];
}

function pythonVersion(path, readFile) {
  const match = readFile(path).match(/^\s*__version__\s*=\s*"([^"]+)"\s*$/m);
  if (!match) {
    throw new Error(`${path} does not declare a runtime version`);
  }
  return match[1];
}

export function readManifestVersions(readFile) {
  return {
    "package.json": jsonVersion("package.json", readFile),
    "apps/desktop/package.json": jsonVersion(
      "apps/desktop/package.json",
      readFile,
    ),
    "packages/contracts/package.json": jsonVersion(
      "packages/contracts/package.json",
      readFile,
    ),
    "services/runtime/pyproject.toml": tomlVersion(
      "services/runtime/pyproject.toml",
      readFile,
    ),
    "services/runtime/src/aip_runtime/__init__.py": pythonVersion(
      "services/runtime/src/aip_runtime/__init__.py",
      readFile,
    ),
    "apps/desktop/src-tauri/Cargo.toml": tomlVersion(
      "apps/desktop/src-tauri/Cargo.toml",
      readFile,
    ),
    "apps/desktop/src-tauri/tauri.conf.json": jsonVersion(
      "apps/desktop/src-tauri/tauri.conf.json",
      readFile,
    ),
  };
}

export function validateManifestVersions(readFile) {
  const versions = readManifestVersions(readFile);
  const canonicalVersion = versions["package.json"];

  if (!SEMVER.test(canonicalVersion)) {
    throw new Error(
      `package.json version is not valid SemVer: ${canonicalVersion}`,
    );
  }
  if (canonicalVersion !== ACTIVE_DEVELOPMENT_VERSION) {
    throw new Error(
      `Phase H active development version must remain ${ACTIVE_DEVELOPMENT_VERSION}; package.json=${canonicalVersion}`,
    );
  }

  const mismatches = Object.entries(versions).filter(
    ([, version]) => version !== canonicalVersion,
  );
  if (mismatches.length > 0) {
    const details = mismatches
      .map(([path, version]) => `${path}=${version}`)
      .join(", ");
    throw new Error(
      `Phase H manifest version mismatch; package.json=${canonicalVersion}, ${details}`,
    );
  }

  return { canonicalVersion, versions };
}

export function validateWorkspaceVersions(repositoryRoot = REPOSITORY_ROOT) {
  return validateManifestVersions((path) =>
    readFileSync(join(repositoryRoot, path), "utf8"),
  );
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const { canonicalVersion } = validateWorkspaceVersions();
  console.log(`Phase H manifest version synchronized: ${canonicalVersion}`);
  console.log(
    "pnpm-lock.yaml dependency integrity remains validated by pnpm install --frozen-lockfile",
  );
}

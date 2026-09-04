import assert from "node:assert/strict";
import test from "node:test";

import { validateManifestVersions } from "./validate-phase-h-version.mjs";

const files = {
  "package.json": JSON.stringify({ version: "0.2.1" }),
  "apps/desktop/package.json": JSON.stringify({ version: "0.2.1" }),
  "packages/contracts/package.json": JSON.stringify({ version: "0.2.1" }),
  "services/runtime/pyproject.toml": '[project]\nversion = "0.2.1"\n',
  "services/runtime/src/aip_runtime/__init__.py": '__version__ = "0.2.1"\n',
  "apps/desktop/src-tauri/Cargo.toml": '[package]\nversion = "0.2.1"\n',
  "apps/desktop/src-tauri/tauri.conf.json": JSON.stringify({
    version: "0.2.1",
  }),
};

const readFixture = (path) => {
  assert.ok(path in files, `unexpected file read: ${path}`);
  return files[path];
};

test("accepts synchronized active manifest versions without reading pnpm-lock.yaml", () => {
  const result = validateManifestVersions(readFixture);
  assert.equal(result.canonicalVersion, "0.2.1");
});

test("rejects drift in an authoritative manifest", () => {
  const driftedFiles = {
    ...files,
    "apps/desktop/package.json": JSON.stringify({ version: "0.2.0" }),
  };

  assert.throws(
    () => validateManifestVersions((path) => driftedFiles[path]),
    /apps\/desktop\/package\.json=0\.2\.0/,
  );
});

test("rejects changing the active development version", () => {
  const changedFiles = {
    ...files,
    "package.json": JSON.stringify({ version: "0.2.0" }),
  };

  assert.throws(
    () => validateManifestVersions((path) => changedFiles[path]),
    /active development version must remain 0\.2\.1/,
  );
});

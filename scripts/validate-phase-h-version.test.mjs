import assert from "node:assert/strict";
import test from "node:test";

import { validateManifestVersions } from "./validate-phase-h-version.mjs";

const files = {
  "package.json": JSON.stringify({ version: "0.2.4" }),
  "apps/desktop/package.json": JSON.stringify({ version: "0.2.4" }),
  "packages/contracts/package.json": JSON.stringify({ version: "0.2.4" }),
  "services/runtime/pyproject.toml": '[project]\nversion = "0.2.4"\n',
  "services/runtime/src/aip_runtime/__init__.py": '__version__ = "0.2.4"\n',
  "apps/desktop/src-tauri/Cargo.toml": '[package]\nversion = "0.2.4"\n',
  "apps/desktop/src-tauri/tauri.conf.json": JSON.stringify({
    version: "0.2.4",
  }),
  "build-revision.txt": "0.2.4.1\n",
};

const readFixture = (path) => {
  assert.ok(path in files, `unexpected file read: ${path}`);
  return files[path];
};

test("accepts synchronized active manifest versions without reading pnpm-lock.yaml", () => {
  const result = validateManifestVersions(readFixture);
  assert.equal(result.canonicalVersion, "0.2.4");
  assert.equal(result.buildRevision, "0.2.4.1");
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
    /active development version must remain 0\.2\.4/,
  );
});

test("rejects a build identity that does not extend the active version", () => {
  const changedFiles = {
    ...files,
    "build-revision.txt": "0.2.4.2\n",
  };

  assert.throws(
    () => validateManifestVersions((path) => changedFiles[path]),
    /build identity must remain 0\.2\.4\.1/,
  );
});

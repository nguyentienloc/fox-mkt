#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const SEMVER = /^\d+\.\d+\.\d+$/;

const files = [
  { path: "package.json", pattern: /("version":\s*")([^"]+)(")/ },
  { path: "src-tauri/tauri.conf.json", pattern: /("version":\s*")([^"]+)(")/ },
  { path: "src-tauri/Cargo.toml", pattern: /^(version = ")([^"]+)(")/m },
  {
    path: "src-tauri/Cargo.lock",
    pattern: /(name = "foxia-mkt"\r?\nversion = ")([^"]+)(")/,
  },
];

function currentVersion() {
  return JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version;
}

function nextVersion(current, arg) {
  if (SEMVER.test(arg)) return arg;
  const [major, minor, patch] = current.split(".").map(Number);
  if (arg === "patch") return `${major}.${minor}.${patch + 1}`;
  if (arg === "minor") return `${major}.${minor + 1}.0`;
  if (arg === "major") return `${major + 1}.0.0`;
  throw new Error(
    `Invalid version "${arg}". Use x.y.z, "patch", "minor" or "major".`,
  );
}

const arg = process.argv[2];
const current = currentVersion();

if (!arg) {
  console.log(current);
  process.exit(0);
}

const next = nextVersion(current, arg);
if (process.argv.includes("--dry-run")) {
  console.log(next);
  process.exit(0);
}

for (const { path, pattern } of files) {
  const fullPath = join(root, path);
  const content = readFileSync(fullPath, "utf8");
  if (!pattern.test(content)) {
    throw new Error(`Could not find the version field in ${path}`);
  }
  writeFileSync(fullPath, content.replace(pattern, `$1${next}$3`));
  console.log(`  ${path}: ${current} -> ${next}`);
}
console.log(next);

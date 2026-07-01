import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { execSync } from "node:child_process";

const ROOT = new URL("..", import.meta.url).pathname;

const version = process.argv[2];
if (!version) {
  console.error("Usage: bun scripts/release.js <version|patch|minor|major>");
  process.exit(1);
}

function readJSON(path) {
  return JSON.parse(readFileSync(path, "utf-8"));
}

function writeJSON(path, obj) {
  writeFileSync(path, JSON.stringify(obj, null, 2) + "\n");
}

function bump(current, kind) {
  const parts = current.split(".").map(Number);
  switch (kind) {
    case "major":
      parts[0]++;
      parts[1] = 0;
      parts[2] = 0;
      break;
    case "minor":
      parts[1]++;
      parts[2] = 0;
      break;
    case "patch":
      parts[2]++;
      break;
    default:
      return kind;
  }
  return parts.join(".");
}

// Update package.json
const packagePath = join(ROOT, "package.json");
const pkg = readJSON(packagePath);
const old = pkg.version;
const next = bump(old, version);
pkg.version = next;
writeJSON(packagePath, pkg);

// Update Cargo.toml
const cargoPath = join(ROOT, "src-tauri", "Cargo.toml");
let cargo = readFileSync(cargoPath, "utf-8");
cargo = cargo.replace(/^version = ".*"/m, `version = "${next}"`);
writeFileSync(cargoPath, cargo);

console.log(`Bumping: ${old} → ${next}`);

// Refresh lock files
execSync("bun install", { cwd: ROOT, stdio: "ignore" });
execSync("cargo generate-lockfile", {
  cwd: join(ROOT, "src-tauri"),
  stdio: "ignore",
});

// Git commit and tag
execSync(
  "git add package.json bun.lock src-tauri/Cargo.toml src-tauri/Cargo.lock",
  { cwd: ROOT },
);
execSync(`git commit -m "chore: bump to ${next}"`, { cwd: ROOT });
execSync(`git tag v${next}`, { cwd: ROOT });

console.log(`Committed and tagged: v${next}`);
console.log(`Push with: git push && git push --tags`);

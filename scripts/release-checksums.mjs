import { createHash } from "node:crypto";
import { copyFile, mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = join(root, "dist-release");

const sources = [
  join(root, "src-tauri", "target", "release", "bundle", "nsis"),
  join(root, "src-tauri", "target", "release", "bundle", "msi"),
  join(root, "dist-portable"),
];

await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });

const artifacts = [];
for (const directory of sources) {
  if (!existsSync(directory)) continue;
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (!entry.isFile()) continue;
    if (!/\.(exe|msi|zip)$/i.test(entry.name)) continue;
    artifacts.push(join(directory, entry.name));
  }
}

if (artifacts.length === 0) {
  console.error("未找到安装包或便携版 zip，请先运行 npm run tauri:build 与 npm run package:portable。");
  process.exit(1);
}

const lines = [];
for (const artifact of artifacts) {
  const name = artifact.split(/[\\/]/).pop();
  const digest = createHash("sha256").update(await readFile(artifact)).digest("hex");
  await copyFile(artifact, join(output, name));
  lines.push(`${digest}  ${name}`);
  console.log(`${name}  ${digest}`);
}

await writeFile(join(output, "SHA256SUMS.txt"), `${lines.join("\n")}\n`, "utf8");
console.log(`\n发布产物已汇总到：${output}`);

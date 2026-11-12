import { copyFile, mkdir, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const exe = join(root, "src-tauri", "target", "release", "pixtrace.exe");
const outputRoot = join(root, "dist-portable");
const appDir = join(outputRoot, "PixTrace");
const zip = join(outputRoot, "PixTrace-portable-x64.zip");

if (!existsSync(exe)) {
  console.error("未找到 release 可执行文件，请先运行：npm run tauri:build -- --no-bundle");
  process.exit(1);
}

await rm(outputRoot, { recursive: true, force: true });
await mkdir(appDir, { recursive: true });
await copyFile(exe, join(appDir, "PixTrace.exe"));
await writeFile(join(appDir, "portable.flag"), "PixTrace portable build\n");
await copyFile(join(root, "README.md"), join(appDir, "README.txt"));

if (process.platform === "win32") {
  execFileSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-Command",
      `Compress-Archive -Path '${appDir}' -DestinationPath '${zip}' -Force`,
    ],
    { stdio: "inherit" },
  );
  console.log(`便携版已生成：${zip}`);
} else {
  console.log(`便携版目录已生成：${appDir}`);
}

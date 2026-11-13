import { createHash } from "node:crypto";
import { copyFile, cp, mkdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const exe = join(root, "src-tauri", "target", "release", "pixtrace.exe");
const outputRoot = join(root, "dist-portable");

const args = process.argv.slice(2);
const webviewFlag = args.indexOf("--webview2");
const fixedRuntime =
  webviewFlag >= 0
    ? resolve(args[webviewFlag + 1] ?? "")
    : process.env.PIXTRACE_WEBVIEW2_DIR
      ? resolve(process.env.PIXTRACE_WEBVIEW2_DIR)
      : null;

if (!existsSync(exe)) {
  console.error("未找到 release 可执行文件，请先运行：npm run tauri:build -- --no-bundle");
  process.exit(1);
}

if (fixedRuntime && !existsSync(join(fixedRuntime, "msedgewebview2.exe"))) {
  console.error(
    `指定的 WebView2 固定运行时目录无效：${fixedRuntime}\n` +
      "目录中必须包含 msedgewebview2.exe。",
  );
  process.exit(1);
}

const packageJson = JSON.parse(await readFile(join(root, "package.json"), "utf8"));
const version = packageJson.version;
const suffix = fixedRuntime ? "-full" : "";
const appDir = join(outputRoot, "PixTrace");
const zip = join(outputRoot, `PixTrace-${version}-portable${suffix}-x64.zip`);

const launcher = [
  "@echo off",
  "chcp 65001 >nul 2>nul",
  "setlocal",
  'cd /d "%~dp0"',
  "",
  'set "WEBVIEW2_FOUND="',
  'if exist "%~dp0webview2-runtime\\msedgewebview2.exe" (',
  '  set "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=%~dp0webview2-runtime"',
  '  set "WEBVIEW2_FOUND=1"',
  ")",
  "if defined WEBVIEW2_FOUND goto run",
  "",
  'set "WEBVIEW2_GUID={F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"',
  'reg query "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\%WEBVIEW2_GUID%" /v pv >nul 2>nul && set "WEBVIEW2_FOUND=1"',
  "if defined WEBVIEW2_FOUND goto run",
  'reg query "HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\%WEBVIEW2_GUID%" /v pv >nul 2>nul && set "WEBVIEW2_FOUND=1"',
  "if defined WEBVIEW2_FOUND goto run",
  'reg query "HKCU\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\%WEBVIEW2_GUID%" /v pv >nul 2>nul && set "WEBVIEW2_FOUND=1"',
  "if defined WEBVIEW2_FOUND goto run",
  'if exist "%ProgramFiles(x86)%\\Microsoft\\EdgeWebView\\Application\\msedgewebview2.exe" set "WEBVIEW2_FOUND=1"',
  "if defined WEBVIEW2_FOUND goto run",
  'if exist "%ProgramFiles%\\Microsoft\\EdgeWebView\\Application\\msedgewebview2.exe" set "WEBVIEW2_FOUND=1"',
  "if defined WEBVIEW2_FOUND goto run",
  "",
  "echo.",
  "echo 未检测到 Microsoft Edge WebView2 运行时，PixTrace 无法启动。",
  "echo 请安装 WebView2 Runtime 后重试：",
  "echo   https://developer.microsoft.com/microsoft-edge/webview2/",
  "echo 或使用自带运行时的完整版便携包（文件名含 -full）。",
  "echo.",
  "pause",
  "exit /b 1",
  "",
  ":run",
  'start "" "%~dp0PixTrace.exe" %*',
  "",
].join("\r\n");

await rm(outputRoot, { recursive: true, force: true });
await mkdir(appDir, { recursive: true });
await copyFile(exe, join(appDir, "PixTrace.exe"));
await writeFile(join(appDir, "portable.flag"), "PixTrace portable build\n");
await writeFile(join(appDir, "启动 PixTrace.cmd"), launcher, "utf8");
await copyFile(join(root, "README.md"), join(appDir, "README.txt"));
if (existsSync(join(root, "docs", "USER_GUIDE.md"))) {
  await copyFile(join(root, "docs", "USER_GUIDE.md"), join(appDir, "用户指南.md"));
}

if (fixedRuntime) {
  console.log(`复制 WebView2 固定运行时：${fixedRuntime}`);
  await cp(fixedRuntime, join(appDir, "webview2-runtime"), { recursive: true });
}

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
  const digest = createHash("sha256")
    .update(await readFile(zip))
    .digest("hex");
  const checksum = `${zip}.sha256`;
  const name = zip.split(/[\\/]/).pop();
  await writeFile(checksum, `${digest}  ${name}\n`, "utf8");
  const size = (await stat(zip)).size;
  console.log(`便携版已生成：${zip}`);
  console.log(`大小：${(size / 1024 / 1024).toFixed(1)} MB`);
  console.log(`SHA-256：${digest}`);
  console.log(`校验文件：${checksum}`);
} else {
  console.log(`便携版目录已生成：${appDir}`);
}

# PixTrace（图迹）

本地 ComfyUI 生成图像浏览器。当前进度：**M0 工程骨架**。

## M0 已实现

- Tauri 2 + Vue 3 + TypeScript + Vite 工程
- 安装版 / 便携版运行模式判定与数据目录抽象
- 工作目录递归扫描，索引写入 `.pixtrace/index.db`（schema v1，含 FTS5 表）
- `pixtrace://` 自定义协议：协议自检图 + 按 `image_id` 读取工作目录内图片
- 三栏应用外壳、欢迎页、目录信息、样例图预览、协议自检
- Rust 单元/集成测试、GitHub Actions CI

## 开发

要求：Node.js 20+、Rust 1.77+、Windows 10/11（首版目标平台）。

```bash
npm install
npm run tauri:dev
```

仅前端调试：

```bash
npm run dev
```

## 检查与测试

```bash
npm run typecheck
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## 构建

安装版（NSIS）：

```bash
npm run tauri:build
```

便携版（先构建可执行文件，再组装 zip）：

```bash
npm run tauri:build -- --no-bundle
node scripts/package-portable.mjs
```

便携版目录中的 `portable.flag` 决定运行模式；存在时设置、日志与 WebView2 用户数据写入程序目录 `data/`，否则写入 `%APPDATA%\PixTrace`。可用 `--portable` / `--installed` 或环境变量 `PIXTRACE_PORTABLE=1` 强制覆盖。

## 目录

- `src/`：Vue 3 前端
- `src-tauri/`：Tauri 2 / Rust 后端
- `docs/PROJECT_PLAN.md`：项目计划（功能、架构、里程碑）
- `scripts/`：资源生成与便携版打包脚本

## 索引位置

打开工作目录后，PixTrace 在目录内创建 `.pixtrace/index.db`。删除该目录只会丢失索引与缩略图缓存，不影响原始图像；评级等用户数据的备份策略在后续里程碑实现。

# PixTrace（图迹）

本地 ComfyUI 生成图像浏览器。当前进度：**M1 浏览与解析**。

## M1 已实现

- PNG `tEXt` / `zTXt` / `iTXt` 文本块与 `IHDR` 尺寸解析，带大小上限与截断保护
- ComfyUI API `prompt`、UI `workflow`、A1111 `parameters` 三类元数据回退解析
- 正负提示词提取（采样器反向遍历、正向闭包剔除负向节点），模型 / LoRA / 采样器 / 步数 / CFG / 种子 / 分辨率
- 后台索引队列：批量事务、进度事件、取消、单文件失败隔离
- 增量对账：新增、修改、删除、恢复；`parser_version` 变更触发重解析
- 按需生成 WebP 缩略图并缓存到 `.pixtrace/cache/thumbs`，`pixtrace://thumb/<id>` 提供
- 行虚拟化网格、分页加载、详情覆盖层、滚轮缩放 / 拖拽平移 / 适应窗口 / 1:1
- 详情检查器：提示词、参数、工作流 JSON 树、文件信息；底部胶片条
- 本地合成 10,003 张图像验证：打开后约 0.6 秒出现首批缩略图，后台索引约 3 秒内完成

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

生成用于手动验收的样例工作目录（含 SD1.5 / SDXL / Flux 提示词、无元数据、损坏 PNG 与超大 JSON）：

```bash
node scripts/make-sample-workspace.mjs "C:/temp/pixtrace-sample" 36
# 追加 small / tiny 参数可生成大量小图，用于虚拟网格与索引压力测试
node scripts/make-sample-workspace.mjs "C:/temp/pixtrace-10k" 10000 tiny
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

如果直接使用 Cargo 构建可执行文件，必须启用 `custom-protocol` 特性，否则应用会尝试连接 Vite 开发服务器：

```bash
cd src-tauri
cargo build --release --features custom-protocol
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

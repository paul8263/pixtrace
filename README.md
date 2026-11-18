# PixTrace（图迹）

本地 AI 生成图像浏览器。技术栈：Tauri 2 + Vue 3 + TypeScript + Vite + Rust + SQLite/FTS5。

## 功能特性

### 浏览与详情

- 递归扫描工作目录，支持 PNG / JPEG / WebP；网格行虚拟化与分页加载，索引期间可继续浏览与检索。
- 网格缩略图显示比例可调：右下角拖动杆或 `Ctrl` + 滚轮，双击恢复默认并自动保存。
- 详情页支持滚轮缩放、拖拽平移、适应窗口与 1:1。
- 底部胶片条可在当前结果集中切换图片，支持滚轮横向浏览；可拖动上边缘调整高度，缩略图同步缩放，双击恢复默认并自动保存。
- 详情检查器展示正负提示词、参数、工作流与文件信息。
- 提示词按分词切分并高亮：悬停强调词块，点击加入正向搜索 chip，`Ctrl` + 点击加入负向 chip。
- 超大提示词与 JSON 分块展示：提示词前 20,000 字符、原始文本前 200,000 字符、JSON 树按深度与节点数截断；切换图片会取消未完成的加载。

### 元数据与工作流

- 解析 PNG `tEXt` / `zTXt` / `iTXt` 文本块与 `IHDR` 尺寸，带大小上限与截断保护。
- 支持 ComfyUI API `prompt`、UI `workflow`、A1111 `parameters` 三类元数据回退解析。
- 提取正负提示词、模型、LoRA、采样器、步数、CFG、种子与分辨率；正负分支通过采样器反向遍历与正向闭包剔除。
- 工作流以节点连线图展示：节点按类别着色，端口之间用彩色贝塞尔曲线连接；支持滚轮缩放、拖拽平移、适应窗口与 1:1。
- 可切换 JSON 树查看层级结构，或原始文本查看未解析的 JSON。
- `pixtrace://` 自定义协议按 `image_id` 安全读取工作目录内图片与缩略图，并提供协议自检图。

### 检索与筛选

- 中英文分词：英文按词切分并保留下划线，中文 / 日文 / 韩文使用 `jieba-rs`。
- SQLite FTS5 倒排索引与 BM25 相关度排序；索引与查询共用同一套分词。
- 检索语法：空格分隔为 AND，`"短语"` 精确匹配，`-词` 排除，例如 `masterpiece "red dress" -lowres`。
- 词块搜索框：空格提交词块，悬停显示删除按钮，点击词块或空输入框按退格可内联编辑，粘贴检索式自动拆分。
- 支持按模型、采样器、是否包含工作流筛选，以及未评级 / 至少 N 星 / 恰好 N 星评级筛选。
- 排序支持相关度、最新、路径与评级。
- 搜索框 280ms 防抖，回车立即检索，`Esc` 或清除按钮重置，`Ctrl+F` 聚焦搜索框。
- 索引期间可浏览与检索，索引完成后自动刷新结果；筛选选项随索引更新。

### 评级、分组与对比

- 1–5 星评级：网格悬停星级、详情星级或数字键 `1`–`5`；点击当前星级清除评级。
- 评级乐观更新，写入失败时回滚并在图库顶部提示。
- 评级按相对路径关联，文件删除后保留、恢复后重新生效；每次评级及打开目录时导出 `.pixtrace/ratings.json`，索引库删除后可自动恢复。
- 同提示词分组：按规范化正面提示词哈希聚合，支持按数量 / 最近排序、组内网格浏览、直接对比组内前 4 张。
- 网格可勾选 2–4 张图像进入全屏对比；对比视图支持同步缩放与平移、同步开关、重置视图。
- 参数差异表可拖动调整高度，高亮提示词、模型、采样器、步数、CFG、种子与 LoRA 差异。

### 索引、监听与缓存

- 工作目录内 SQLite 索引 `.pixtrace/index.db`，包含 FTS5 表。
- 后台索引队列：批量事务、进度事件、取消与单文件失败隔离。
- 增量对账：新增、修改、删除、恢复；`parser_version` 变更触发重新解析。
- 文件监听默认开启，递归监听工作目录，约 2 秒防抖后增量扫描；过滤 `.pixtrace` 与不支持的扩展名。
- 支持手动“重新扫描目录”和“重建提示词索引”。
- 按需生成 WebP 缩略图并缓存到 `.pixtrace/cache/thumbs`；默认 2GB 上限，按最近使用时间 LRU 清理。
- 设置页可查看缓存占用、修改上限、清理缓存。
- 工作目录锁 `.pixtrace/workspace.lock`，同一目录禁止多实例同时写入；网络路径与只读目录给出降级提示，便携模式不可写时回退到临时目录。

### 文件管理与设置

- 网格支持全选 / 全不选 / 反选，作用于当前搜索结果的全部图片；支持批量删除，对比仅在选中 2–4 张时可用。
- 网格与对比视图均可右键删除图片；删除确认默认“移到回收站”，也可选择“永久删除”。
- 对比视图删除后自动移出对比；双图对比删除一幅后自动进入剩余图片详情。
- 设置面板：工作目录信息、扫描 / 重建、文件监听开关、缓存统计与清理、数据与日志位置。
- 状态栏“关于”显示应用图标、名称、版本、作者与联系方式。
- 三栏应用外壳、欢迎页、目录信息与协议自检。

### 分发与发布

- 安装版 / 便携版运行模式判定与数据目录抽象。
- NSIS / MSI 安装包与便携版 zip；便携启动脚本检测 WebView2，完整版可内置固定运行时。
- 发布产物 SHA-256 校验、Windows Authenticode 签名脚本与 GitHub Actions 构建产物上传。
- 用户指南 `docs/USER_GUIDE.md`。

### 工程质量与性能

- Rust 单元 / 集成测试覆盖元数据解析、索引、检索、评级、缓存与工作目录锁。
- GitHub Actions CI 自动执行前端构建、Rust 检查与测试，并上传构建产物。
- 本地合成 10,003 张图像：打开后约 0.6 秒出现首批缩略图，后台索引约 3 秒内完成。
- 10 万张 tiny PNG：目录扫描 0.7s、后台索引约 39s（100,003 张，0 失败）、搜索 2–33ms、进程内存约 76MB、索引约 129MB。
- 5 万行检索（含评级排序）release 模式性能回归：P95 约 102ms。

## 开发

要求：Node.js 20+、Rust 1.77+、Windows 10/11（首版目标平台）。

```bash
npm install
npm run tauri:dev
```

生成用于手动验收的样例工作目录（含 SD1.5 / SDXL / Flux / 中文提示词、SDXL 工作流、同一提示词多张图像、无元数据、损坏 PNG 与超大 JSON）：

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
# 5 万行检索延迟回归（默认忽略，release 模式运行）
cargo test --release --test search_test -- --ignored --nocapture
```

## 构建

安装版（NSIS + MSI）：

```bash
npm run tauri:build
```

MSI 由 Tauri 自动下载 WiX 3 工具链；网络受限时可只构建 NSIS：

```bash
npm run tauri:build -- --bundles nsis
```

如果直接使用 Cargo 构建可执行文件，必须启用 `custom-protocol` 特性，否则应用会尝试连接 Vite 开发服务器：

```bash
cd src-tauri
cargo build --release --features custom-protocol
```

便携版（先构建可执行文件，再组装 zip）：

```bash
npm run tauri:build -- --no-bundle
npm run package:portable
```

便携包会包含 `PixTrace.exe`、`portable.flag`、`启动 PixTrace.cmd`、`README.txt` 与用户指南。启动脚本会检测 WebView2，缺失时给出安装提示。若已有 WebView2 固定运行时目录，可生成自带运行时的完整版：

```bash
npm run package:portable -- --webview2 "C:\path\to\Microsoft.WebView2.FixedVersionRuntime.x.x.x.x-x64"
# 或设置 PIXTRACE_WEBVIEW2_DIR 环境变量
```

汇总安装包、便携包并生成 SHA-256：

```bash
npm run release:checksums
```

产物输出到 `dist-release/`，包含 `SHA256SUMS.txt`。

便携版目录中的 `portable.flag` 决定运行模式；存在时设置、日志与 WebView2 用户数据写入程序目录 `data/`，否则写入 `%APPDATA%\PixTrace`。可用 `--portable` / `--installed` 或环境变量 `PIXTRACE_PORTABLE=1` 强制覆盖。

### 代码签名

使用 Authenticode 证书签名发布产物：

```powershell
$env:PIXTRACE_SIGN_PFX = "C:\certs\pixtrace.pfx"
$env:PIXTRACE_SIGN_PASSWORD = "***"
.\scripts\sign-windows.ps1 .\dist-release\PixTrace_0.1.0_x64-setup.exe
```

也可以使用证书指纹：

```powershell
$env:PIXTRACE_SIGN_THUMBPRINT = "ABCDEF..."
.\scripts\sign-windows.ps1 .\dist-release\*.msi
```

签名后重新运行 `npm run release:checksums`，确保校验文件对应最终产物。

## 目录

- `src/`：Vue 3 前端
- `src-tauri/`：Tauri 2 / Rust 后端
- `docs/PROJECT_PLAN.md`：项目计划（功能、架构、里程碑）
- `docs/USER_GUIDE.md`：用户指南（打开目录、检索、缓存、分发与常见问题）
- `scripts/`：资源生成、便携版打包、校验与签名脚本

## 索引位置

打开工作目录后，PixTrace 在目录内创建 `.pixtrace/index.db`。删除该目录只会丢失索引与缩略图缓存，不影响原始图像；评分快照保存在 `.pixtrace/ratings.json`，重建索引后会自动恢复。

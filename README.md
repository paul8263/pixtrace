# PixTrace（图迹）

本地 ComfyUI 生成图像浏览器。当前进度：**M3 评级与对比**。

## M3 已实现

- 1–5 星评级：网格悬停星级、详情星级、数字键 `1`–`5`；点击当前星级清除评级
- 评级乐观更新，写入失败时回滚并在图库顶部提示
- 评级筛选：未评级、至少 N 星、恰好 N 星；侧栏实时显示各档数量；工具栏支持按评级排序
- 评级按相对路径关联，文件删除后保留，文件恢复后重新生效
- 每次评级及打开目录时导出 `.pixtrace/ratings.json`；索引库删除后重新打开目录会自动恢复评分
- 同提示词分组：按规范化正面提示词哈希聚合，支持按数量 / 最近排序、组内网格浏览、直接对比组内前 4 张
- 网格多选 2–4 张图像，底部选择操作条进入全屏对比
- 对比视图：同步缩放与平移、同步开关、重置视图；参数差异表高亮提示词、模型、采样器、步数、CFG、种子与 LoRA 差异
- 5 万行检索（含评级排序）性能回归测试：release 模式 P95 约 102ms

## M2 已实现

- 中英文分词：英文按词切分并保留下划线，中文/日文/韩文使用 `jieba-rs`，索引与查询共用同一套分词
- SQLite FTS5 倒排索引，支持 BM25 相关度排序
- 检索语法：空格分隔为 AND、`"短语"` 精确短语、`-词` 排除；示例 `masterpiece "red dress" -lowres`
- 模型 / 采样器筛选、仅含工作流筛选，可与关键词组合
- 排序切换：相关度 / 最新 / 路径
- 搜索框 280ms 防抖、回车立即检索、Esc 或清除按钮重置；`Ctrl+F` 聚焦搜索框
- 索引期间可浏览与检索，索引完成后自动刷新结果；筛选选项随索引更新
- 5 万行检索性能回归测试（release 模式）：P95 < 150ms

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

打开工作目录后，PixTrace 在目录内创建 `.pixtrace/index.db`。删除该目录只会丢失索引与缩略图缓存，不影响原始图像；评分快照保存在 `.pixtrace/ratings.json`，重建索引后会自动恢复。评级等用户数据的完整备份策略在后续里程碑继续完善。

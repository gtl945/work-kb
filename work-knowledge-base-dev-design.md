# 个人工作知识库 — 完整开发文档

> 产品需求文档：`work-knowledge-base-prd.md`
> 本文档覆盖架构设计、里程碑历史、现状实现方案、数据库/接口/解析矩阵、构建打包，随开发进度持续更新。

---

## 一、产品定位

将散落的工作痕迹（Word/PPT/Excel/PDF/TXT/图片等）结构化为可搜索、有时间标签的知识库条目，支持文件级回链溯源，按日/周/季/年多粒度导出报告。

**核心价值**：拖入成品文件 → 本地解析提炼 → 结构化入库 → 搜索筛选 → 多视图导出，全程本地处理，隐私不外传。

**约束**：桌面客户端（Tauri），MVP 只支持文件级回链，云同步/位置级定位延后。OCR 已通过 PaddleOCR 实现（本地运行，不上传数据）。

> **需求变更记录 — OCR 从排除到纳入**
>
> 原 PRD 基于简洁性和隐私考量排除 OCR。实际使用中，用户存在大量以图片形式保存的工作材料（新闻截图、奖项照片、专利证书图片等），这些材料无法通过文本解析提取内容，导致知识库覆盖不全。经复盘确认后纳入 PaddleOCR（本地运行，不上传数据），确保图片材料也能提炼为可搜索条目。HEIC 格式因场景极少已移除。

---

## 二、架构总览

单机桌面应用，三进程协作：Tauri 主进程（Rust 核心）+ Vue 前端（WebView）+ Python 解析 sidecar。

```
work-kb/
├── src/                          # Vue3 前端
│   ├── api.ts                    # Tauri Command 封装
│   ├── router/index.ts           # 路由（导入/知识库/导出/数据库）
│   ├── App.vue                   # 布局框架
│   ├── main.ts                   # 入口
│   └── views/
│       ├── ImportView.vue        # 文件导入与条目提炼
│       ├── KnowledgeBaseView.vue # 知识库搜索与数据管理
│       ├── ExportView.vue        # 报告导出
│       └── DatabaseView.vue      # 数据库状态与备份恢复
├── src-tauri/
│   ├── Cargo.toml                # Rust 依赖
│   ├── tauri.conf.json           # Tauri 配置
│   ├── capabilities/default.json # 权限配置
│   ├── icons/                    # 应用图标
│   ├── migrations/001_init.sql   # 建表 SQL
│   ├── sidecar/
│   │   ├── parse_server.py       # Python 解析服务
│   │   └── requirements.txt       # Python 依赖
│   └── src/
│       ├── main.rs               # 入口
│       ├── lib.rs                # Tauri Command 注册
│       ├── db.rs                 # SQLite + FTS5 操作
│       ├── models.rs             # 数据结构体定义（StatsResult/FileInfo 等）
│       ├── parser.rs             # 解析调度 + Rust 原生解析器 + SUPPORTED_EXTS 常量
│       ├── extractor.rs          # 标题切块 + 规则字段抽取
│       ├── searcher.rs           # 全文检索 + 分词 + 拼音
│       └── exporter.rs           # Markdown 报告生成
├── .github/workflows/build.yml   # GitHub Actions 云编译
├── build.bat                     # 本地一键打包脚本
├── check.sh                      # Linux cargo check 脚本
└── upload-github.ps1             # GitHub API 上传脚本（无需 git）
```

**进程边界与通信：**
- 前端 ↔ Rust：`invoke()` 调用 Tauri Command，参数/返回均为 JSON。
- Rust ↔ Python sidecar：主进程首次解析时 spawn 常驻 sidecar，经 stdin/stdout 走 JSON-RPC，避免反复启动开销。
- 所有解析路径（Rust 原生 + Python sidecar）统一产出 `ParseResult`，下游切块/提炼对来源无感。

---

## 三、里程碑历史与现状

| 里程碑 | 范围 | 状态 | 关键产出 |
|--------|------|------|----------|
| M1 骨架 | Tauri+Vue+Element 脚手架、路由、SQLite 初始化与建表 | ✅ 完成 | 可运行空壳 + 数据库 |
| M2 解析层 | 混塔解析（Rust PDF + Python sidecar Office）+ 统一 ParseResult | ✅ 完成 | 支持 PDF/DOCX/XLSX/PPTX |
| M3 提炼入库 | 标题切块 + 日期/量化值规则抽取 + 入库确认 UI | ✅ 完成 | 条目可入库，支持批量导入 |
| M4 知识库 | FTS5 + jieba 分词 + 拼音 + ngram 容错 + 筛选 + 回链 | ✅ 完成 | 可搜索可溯源 |
| M5 导出 | 多视图切片（日/周/季/年）+ Markdown 输出 | ✅ 完成 | 可出报告，支持复制/保存 |
| M6 打磨 | 删除条目 + 统计概览 + 项目筛选 + 空状态 + 错误提示 | ✅ 完成 | 可交付 v1 |
| 格式扩展 | TXT/MD/CSV/HTML 原生解析 + DOC/XLS/PPT/RTF/WPS sidecar | ✅ 完成 | 支持 16 种文件格式 |
| 云编译 | GitHub Actions 自动打包 + 无 git 上传方案 | ✅ 完成 | 云端生成 EXE |
| 编译修复 | 图标缺失 + jieba cut_for_search 参数 + &mut self | ✅ 完成 | cargo check 通过 |
| 数据库状态页 | 数据库信息展示 + 源文件清单 + 数据备份导出/导入恢复 | ✅ 完成 | 新增 DatabaseView 页面，数据持久化方案确认 |
| 文件夹批量导入 | 递归扫描文件夹 + 文件哈希查重 + 条目级查重 + 批量解析进度 | ✅ 完成 | 支持文件夹导入，三层查重 |
| 解析兜底 | 解析失败/零草稿时自动生成 fallback 草稿，文件名做标题 | ✅ 完成 | 确保所有文件可搜索 |
| 图片格式支持 | JPG/PNG/BMP/GIF/TIFF/WEBP + SVG，PaddleOCR 文字识别 + EXIF 元数据 | ✅ 完成 | 支持 25 种格式，图片可 OCR 搜索 |
| 解析质量优化 | DOCX 表格内联+非标准标题识别、XLSX 合并单元格、PPTX 幻灯片表格+文档属性、类型智能分类、量化值范围/约数、日期格式扩展、超长拆分、CSV 分隔符检测 | ✅ 完成 | 解析提取能力全面提升 |
| 代码复盘 | SQL注入修复+字节长度bug+架构调整（models.rs拆分+SUPPORTED_EXTS统一+SQL常量提取+封装修复） | ✅ 完成 | 代码质量与安全性提升 |

---

## 四、模块职责

| 模块 | 文件 | 进程 | 职责 |
|------|------|------|------|
| 数据结构 | models.rs | Rust | 所有 Tauri Command 返回的结构体定义（StatsResult/FileInfo/DbInfo 等 8 个），独立于 db.rs 业务逻辑 |
| 文件管理 | db.rs | Rust | 登记本地文件（路径+SHA-256 哈希），文件夹递归扫描，CRUD |
| 解析调度 | parser.rs | Rust | 按扩展名路由：原生格式→Rust；Office 系→Python sidecar；CSV 分隔符自动检测；SUPPORTED_EXTS 常量（单一数据源） |
| 切块提炼 | extractor.rs | Rust | 按标题层级切块 + 超长 section 拆分(>500字符)；正则抽日期（多格式）/量化值（范围/约数/普通）；关键词类型分类（亮点>成果>问题>完成） |
| 存储检索 | db.rs + searcher.rs | Rust | SQLite CRUD + FTS5 索引同步 + 全文检索 + 条目查重 |
| 导出 | exporter.rs | Rust | 按时间切片生成日/周/季/年报 Markdown |
| 统计 | db.rs | Rust | 条目统计（总数/分类/项目/文件数）+ 数据库信息 |
| 数据持久化 | db.rs + lib.rs | Rust | WAL checkpoint 导出备份 + ATTACH 导入恢复 |
| Office+图片解析 | parse_server.py | Python | DOCX/XLSX/PPTX + DOC/XLS/PPT/RTF/WPS/ET/DPS + 图片 OCR + EXIF |
| 前端 | src/views/*.vue | Vue | 导入/知识库/导出/数据库四个页面 |

---

## 五、文件解析矩阵（25 种格式）

### Rust 原生解析器（无需 Python）

| 格式 | 函数 | 实现方式 |
|------|------|----------|
| PDF | `parse_pdf` | pdf-extract 提取纯文本，整篇一条 section |
| TXT | `parse_txt` | 按空行分段，UTF-8 优先/GBK 回退（encoding_rs） |
| MD | `parse_md` | 按 `#` 标题层级切分，正文归入对应标题 |
| CSV | `parse_csv` | 自动检测分隔符（逗号/Tab/分号），引号转义，首行表头，输出 TSV |
| HTML | `parse_html` | 去 script/style，h1-h6 转 Markdown 标题，去标签，解码实体 |
| SVG | `parse_svg` | 提取 `<text>`/`<tspan>`/`<title>`/`<desc>` 标签内容 |

### Python sidecar 解析器

| 格式 | 函数 | 依赖库 | 实现方式 |
|------|------|--------|----------|
| DOCX | `parse_docx` | python-docx | 标题层级 + 段落 + **表格内联**（按文档流序）+ **非标准标题识别**（加粗+大字号）+ **文档属性**（创建/修改日期） |
| XLSX | `parse_xlsx` | openpyxl | 每 sheet 一条 section，TSV 行输出 + **合并单元格填充** |
| PPTX | `parse_pptx` | python-pptx | 每页一条 section，标题+正文 + **幻灯片表格** + **演讲者备注** + **文档属性** |
| DOC | `parse_doc` | olefile | antiword 优先 → olefile 提取 WordDocument 流 → 二进制兜底 |
| XLS | `parse_xls` | xlrd | 每 sheet 一条 section，TSV 行输出 |
| PPT | `parse_ppt` | olefile | LibreOffice 转 pptx → olefile 兜底 |
| RTF | `parse_rtf` | striprtf | rtf_to_text 转纯文本 |
| WPS | `parse_wps` | - | 先尝试 DOCX 解析 → 回退 DOC 处理 |
| ET | `parse_et` | - | 先尝试 XLSX 解析 → 回退 XLS 处理 |
| DPS | `parse_dps` | - | 先尝试 PPTX 解析 → 回退 PPT 处理 |
| JPG/JPEG | `parse_image` | PaddleOCR + Pillow | OCR 文字识别 + EXIF 元数据（拍摄日期/设备/型号） |
| PNG | `parse_image` | PaddleOCR + Pillow | 同上 |
| BMP | `parse_image` | PaddleOCR + Pillow | 同上 |
| GIF | `parse_image` | PaddleOCR + Pillow | 同上（首帧） |
| TIF/TIFF | `parse_image` | PaddleOCR + Pillow | 同上 |
| WEBP | `parse_image` | PaddleOCR + Pillow | 同上 |

### 解析路由（dispatch_parse）

```rust
match ext.as_str() {
    // Rust 原生
    "pdf" => parse_pdf(path),
    "txt" => parse_txt(path),
    "md" | "markdown" => parse_md(path),
    "csv" => parse_csv(path),
    "html" | "htm" => parse_html(path),
    "svg" => parse_svg(path),
    // Python sidecar（懒启动）
    "docx" | "xlsx" | "pptx" | "doc" | "xls" | "ppt" | "rtf" | "wps" | "et" | "dps"
    | "jpg" | "jpeg" | "png" | "bmp" | "gif" | "tif" | "tiff" | "webp" => {
        sidecar.parse(path, &ext)
    }
    other => Err(ParseError::Unsupported(other))
}
```

### 统一 ParseResult 结构

```ts
interface ParseResult {
  sourcePath: string;
  docTitle: string;
  sections: {
    heading: string;   // 标题文本，无标题时为空
    level: number;     // 标题层级 1..N；无标题记 0
    body: string;      // 该标题下正文
    page?: number;     // 页码（PPT 用）
  }[];
}
```

### 解析质量优化详情

#### 类型智能分类（extractor.rs::classify_item_type）

按关键词优先级自动判断条目类型，用户可在草稿区修改：

| 优先级 | 类型 | 关键词 |
|--------|------|--------|
| 1 | 亮点 | 获奖/专利/发表/荣誉/表彰/证书/第一/首创/突破/优秀/标兵 |
| 2 | 成果 | 完成/实现/达成/交付/上线/验收/签署/获批/通过 |
| 3 | 问题 | 问题/困难/挑战/瓶颈/障碍/延期/失败/风险/不足 |
| 4 | 完成 | 默认（以上关键词均未命中） |

#### 量化值三级提取（extractor.rs::extract_quant）

| 级别 | 模式 | 示例 |
|------|------|------|
| 1 范围 | `\d+[-~到]\d+[单位]` | 3-5个、3~5人、3到5项 |
| 2 约数 | `(约\|大约\|近\|超\|逾)\d+[单位]` | 约20%、大约15个、近30人 |
| 3 普通 | `\d+[单位]` | 30%、15个 |

单位集：`% ‰ 倍 个 件 次 条 项 篇 份 人 天 日 周 月 年 小时 分钟 秒 元 万 亿 分 度 名 页 章`

#### 日期格式支持（extractor.rs::extract_body_date）

| 格式 | 示例 |
|------|------|
| 年-月-日 | 2026-08-29 |
| 年/月/日 | 2026/8/29 |
| 年月日 | 2026年8月29日 |
| 年.月.日 | 2026.08.29 |
| 年.月 | 2026.08（无日，输出 2026-08） |
| 文件名 | 20260829、2026_08_29 |

#### 超长 section 拆分（extractor.rs::maybe_split_chunks）

- 触发条件：body > 500 字符
- 一级拆分：按 `\n\n`（双换行/段落）切分
- 二级拆分：段落仍超长时按 `\n`（单行）切分
- 拆分后所有子块共享原标题

#### CSV 分隔符自动检测（parser.rs::detect_delimiter）

- 采样前 5 行
- 统计逗号 `,` / Tab `\t` / 分号 `;` 出现次数
- 选频率最高的作为分隔符

#### DOCX 非标准标题识别（parse_server.py::_is_heading_by_format）

- 触发条件：段落无 "Heading" 样式名
- 判定规则：所有 run 均为 bold 且至少一个 run 的 font.size ≥ 14pt
- 命中后默认标为 Heading2 级别

#### DOCX 表格内联（parse_server.py::_iter_block_items）

- 使用 python-docx 内部 XML 遍历（`doc.element.body.iterchildren()`）
- 按 `CT_P` / `CT_Tbl` 类型分别 yield Paragraph / Table 对象
- 表格在文档流中的位置与原文一致，不再堆在末尾

#### XLSX 合并单元格（parse_server.py::parse_xlsx）

- 改为 `read_only=False`（支持 `merged_cells.ranges`）
- 遍历所有合并区域，取左上角值填充到区域内所有单元格
- 输出时每个单元格均有值（不再为空）

#### PPTX 幻灯片表格（parse_server.py::parse_pptx）

- 遍历 `slide.shapes`，检查 `shape.has_table`
- 提取表格行/列转 Markdown 格式（`| cell1 | cell2 |`）
- 与文本和演讲者备注一起输出

#### 文档属性提取（DOCX + PPTX）

- 提取 `core_properties.created` 和 `core_properties.modified`
- 格式化为 `[创建日期] YYYY-MM-DD` 和 `[修改日期] YYYY-MM-DD`
- DOCX 属性在首个 section 的 body 开头输出
- PPTX 属性在第一页 slide 的 body 开头输出

---

## 六、数据库设计（SQLite + FTS5）

### 基础表

```sql
-- 文件登记
CREATE TABLE files (
  id INTEGER PRIMARY KEY, path TEXT NOT NULL, filename TEXT NOT NULL,
  ext TEXT NOT NULL, size INTEGER, hash TEXT,              -- hash: SHA-256 内容查重
  imported_at INTEGER NOT NULL, archived INTEGER DEFAULT 0,
  archived_path TEXT, user_id INTEGER DEFAULT 0, sync_state INTEGER DEFAULT 0
);

-- 项目
CREATE TABLE projects (
  id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL, user_id INTEGER DEFAULT 0, sync_state INTEGER DEFAULT 0
);

-- 标签
CREATE TABLE tags (
  id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL, user_id INTEGER DEFAULT 0, sync_state INTEGER DEFAULT 0
);

-- 条目主表
CREATE TABLE items (
  id INTEGER PRIMARY KEY, title TEXT NOT NULL, type TEXT NOT NULL,
  occur_date TEXT, project_id INTEGER REFERENCES projects(id),
  points_text TEXT NOT NULL, quant_value TEXT,
  source_file_id INTEGER REFERENCES files(id),
  evidence_type TEXT, status TEXT NOT NULL,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
  user_id INTEGER DEFAULT 0, sync_state INTEGER DEFAULT 0
);

-- 条目-标签多对多
CREATE TABLE item_tags (
  item_id INTEGER REFERENCES items(id) ON DELETE CASCADE,
  tag_id INTEGER REFERENCES tags(id),
  PRIMARY KEY (item_id, tag_id)
);
```

### 全文索引（FTS5）

```sql
CREATE VIRTUAL TABLE items_fts USING fts5(
  title_seg,       -- jieba 分词后的标题
  points_seg,      -- jieba 分词后的要点正文
  pinyin_full,     -- 全拼，如 "zhuanli"
  pinyin_initial,  -- 首字母，如 "zl"
  ngram,           -- bigram 容错
  tokenize='unicode61'
);
```

**写入约定**：`items_fts.rowid` = `items.id`，便于 join。入库时 jieba 分词 + pinyin 生成 + bigram，同事务写入主表和 FTS。

**查询构造**：输入经分词→拼音→bigram 后拼 MATCH 串，跨列 OR，多词 AND，叠加类型/日期/项目筛选。

### 三层查重策略

| 层级 | 时机 | 方式 | 字段 | 行为 |
|------|------|------|------|------|
| L1 路径查重 | 文件夹扫描时 | `SELECT id FROM files WHERE path = ?` | `files.path` | 标注"已登记"，前端跳过 |
| L2 哈希查重 | 文件登记时 | 计算 SHA-256 → `SELECT id FROM files WHERE hash = ?` | `files.hash` | 返回 `is_new=false`，前端标记"内容重复" |
| L3 条目查重 | 解析出草稿后 | `SELECT COUNT(*) FROM items WHERE title=? AND occur_date=?` | `items.title + items.occur_date` | 草稿标注"疑似重复"，用户决定跳过或保留 |

**性能**：L1/L3 为 SQL 索引查询（O(1)）；L2 需读取文件计算哈希（8KB 分块 SHA-256，100MB 文件 <1s），仅对 L1 未命中的文件执行。

---

## 七、Tauri Command 接口

| Command | 入参 | 返回 | 说明 |
|---------|------|------|------|
| `ping` | - | `string` | 版本号 |
| `db_status` | - | `string` | 库内表名（连接状态） |
| `import_files` | `{paths: string[]}` | `RegisterResult[]` | 登记文件（含路径+哈希查重），返回登记结果 |
| `parse_file` | `{fileId: number}` | `DraftItem[]` | 解析+切块+提炼，返回草稿条目；解析失败/零草稿时返回 fallback 草稿（文件名做标题，isFallback=true） |
| `confirm_items` | `{items: DraftItem[]}` | `number[]` | 确认入库，返回条目 id |
| `search` | `{query, filters}` | `SearchResult[]` | FTS5 全文检索+多维筛选 |
| `open_source_file` | `{itemId: number}` | `boolean` | 系统默认程序打开源文件 |
| `export_view` | `{granularity, dateFrom, dateTo}` | `{markdown, fileList, itemCount}` | 多粒度报告 |
| `save_file` | `{path, content}` | `void` | 保存 Markdown 到文件 |
| `delete_item` | `{itemId: number}` | `void` | 删除条目及 FTS/标签关联（事务） |
| `get_stats` | - | `StatsResult` | 统计数据 |
| `get_projects` | - | `ProjectInfo[]` | 项目列表（带条目数） |
| `get_tags` | - | `TagInfo[]` | 标签列表（带条目数） |
| `get_db_info` | - | `DbInfo` | 数据库路径/大小/各表行数 |
| `get_file_list` | - | `SourceFileInfo[]` | 已登记源文件列表（带关联条目数） |
| `export_data` | `{targetPath: string}` | `void` | WAL checkpoint 后复制 db 文件到指定路径（备份） |
| `import_data` | `{sourcePath: string}` | `void` | ATTACH 源 db → 事务清空并复制全部表数据（恢复） |
| `scan_folder` | `{folderPath: string}` | `ScannedFile[]` | 递归扫描文件夹，返回支持格式的文件列表（标注已登记状态） |
| `check_item_duplicate` | `{title, occurDate}` | `boolean` | 检查条目是否疑似重复（标题+日期相同） |

### 关键数据结构

```ts
// 文件登记结果（含查重信息）
interface RegisterResult {
  fileId: number;
  isNew: boolean;              // true=新建，false=重复
  duplicateReason: string | null;  // "路径已存在" | "内容已存在" | null
}

// 草稿条目（解析产出，用户确认前可编辑）
interface DraftItem {
  title: string;
  itemType: string;
  occurDate: string | null;
  project: string | null;
  pointsText: string;
  quantValue: string | null;
  sourceFileId: number;
  evidenceType: string | null;
  tags: string[];
  isFallback?: boolean;         // true=解析失败自动生成（文件名做标题）
}

// 文件夹扫描结果
interface ScannedFile {
  path: string;
  filename: string;
  ext: string;
  size: number | null;
  alreadyRegistered: boolean;  // 路径查重命中
  fileId: number | null;       // 已登记的 file_id
}

// 数据库信息
interface DbInfo {
  dbPath: string;
  dbSize: number;               // bytes
  tables: TableInfo[];
}
interface TableInfo { name: string; rowCount: number; }

// 已登记源文件信息
interface SourceFileInfo {
  id: number; filename: string; path: string; ext: string;
  size: number | null; importedAt: number; itemCount: number;
}
```

### Python sidecar 协议（JSON-RPC over stdio）

```jsonc
// 请求
{"id":1,"method":"parse","params":{"path":"D:/xx.docx","kind":"docx"}}
// 响应
{"id":1,"ok":true,"result":{ /* ParseResult */ }}
```

---

## 八、搜索引擎设计

| 能力 | 实现 |
|------|------|
| 中文分词 | jieba-rs 搜索模式（cut_for_search, hmm=true） |
| 拼音 | pinyin crate 生成全拼 + 首字母 |
| 容错 | bigram n-gram 倒排索引 |
| 前缀匹配 | FTS5 前缀 `zhuanli*` |
| 多词 AND | 多个 MATCH 子句 AND 串联 |
| 多维筛选 | 类型/日期范围/证据类型/项目 |
| 无关键词浏览 | 空查询时返回全部（LIMIT 200） |

---

## 九、导出系统

按日/周/季/年粒度分组条目，生成结构化 Markdown 报告：
- 标题 + 时间范围
- 概览统计（总数/分类计数）
- 源文件列表
- 按日期分组、按类型（完成/成果/问题/亮点）子分组
- 每条条目：标题 + 日期 + 要点正文 + 量化值

---

## 十、前端页面

### ImportView.vue — 文件导入与提炼
- 双模式选择：选择文件（多选 25 种格式）/ 选择文件夹（递归扫描子目录）
- 文件夹扫描：递归遍历，自动过滤不支持的格式，标注已登记文件
- 三层查重：路径查重（扫描时）→ SHA-256 哈希查重（登记时）→ 条目级查重（解析后）
- 批量解析：逐个解析（非并发），进度条更新，跳过重复文件
- 草稿编辑区：类型/标题/日期/项目/标签/证据类型/量化值
- 疑似重复条目标注（L3 条目查重），支持一键跳过重复
- 解析失败·自动生成标注（fallback 草稿，文件名做标题）
- 全部确认入库按钮
- 空状态引导（无文件→选择按钮；无草稿→提示原因）

### KnowledgeBaseView.vue — 知识库搜索与数据管理
- 6 张统计卡片（总数/完成/成果/亮点/项目数/源文件数）
- 搜索栏（关键词 + 项目下拉筛选）
- 结果列表（类型标签 + 标题 + 日期 + 要点 + 量化值 + 源文件名）
- 每条目删除按钮（确认弹窗 + 本地状态同步）
- 空状态优化（空知识库→引导导入；无结果→提示调整条件）

### ExportView.vue — 报告导出
- 粒度选择（日/周/季/年）
- 日期范围选择
- 实时 Markdown 预览
- 条目数展示
- 复制到剪贴板 / 保存为 .md 文件
- 空数据提示

### DatabaseView.vue — 数据库状态与备份恢复
- 数据库文件路径和大小展示
- 数据持久化说明（本地 AppData 存储，更新应用不丢数据）
- 各表行数统计（files/projects/tags/items/item_tags/items_fts）
- 已登记源文件清单（文件名/格式/大小/关联条目数/导入时间/路径）
- 导出备份（WAL checkpoint 后复制 db 文件到用户选定路径）
- 恢复数据（ATTACH 源 db → 事务清空并复制全部表数据，含 FTS5 重建）

---

## 十一、数据持久化方案

### 存储位置

数据库文件存储在 Tauri 的 `app_data_dir()` 路径下：

| OS | 路径 |
|----|------|
| Windows | `C:\Users\<用户名>\AppData\Roaming\work-kb\workkb.db` |
| macOS | `~/Library/Application Support/work-kb/workkb.db` |
| Linux | `~/.local/share/workkb/workkb.db` |

### 跨版本持久化

**结论：重新打包安装新版本后，旧数据自动保留。**

- `app_data_dir()` 位于用户目录，不在应用安装目录内
- 卸载/安装应用不会触碰 AppData 目录
- `init_schema()` 使用 `CREATE TABLE IF NOT EXISTS`，幂等不破坏数据
- WAL 模式下还会有 `workkb.db-wal` 和 `workkb.db-shm` 辅助文件

### 风险与应对

| 风险 | 应对 |
|------|------|
| 应用 identifier 变更导致 AppData 路径变化 | 保持 `tauri.conf.json` 的 `identifier` 不变 |
| 系统清理工具误删 AppData | 提供导出备份功能，用户可主动备份 |
| 跨电脑迁移 | 导出 db 文件 → 新电脑导入恢复 |
| WAL 文件被手动删除 | 导出前执行 `PRAGMA wal_checkpoint(TRUNCATE)` 刷盘 |

### 备份/恢复实现

- **导出**：`PRAGMA wal_checkpoint(TRUNCATE)` 刷盘 → `std::fs::copy` 复制 db 文件
- **导入**：`ATTACH DATABASE '源路径' AS src` → 事务内 DELETE 全部表 → INSERT FROM src（含 FTS5 重建）→ DETACH
- 导入路径经单引号转义（`replace('\' => "''")`），防 SQL 注入

---

## 十二、依赖清单

### Rust（Cargo.toml）

| crate | 版本 | 用途 |
|-------|------|------|
| tauri | 2 | 应用框架 |
| rusqlite | 0.32 (bundled) | SQLite + FTS5 |
| serde / serde_json | 1 | 序列化 |
| pdf-extract | 0.7 | PDF 文本提取 |
| regex | 1 | 日期/量化值正则 |
| jieba-rs | 0.7 | 中文分词 |
| pinyin | 0.10 | 拼音生成 |
| encoding_rs | 0.8 | GBK 编码回退 |
| sha2 | 0.10 | SHA-256 文件哈希（查重） |
| tauri-plugin-dialog | 2 | 系统文件选择器 |
| open | 5 | 调用系统默认程序打开文件 |
| chrono | 0.4 | 日期计算（周/季分组） |

### Python（requirements.txt）

| 包 | 用途 |
|----|------|
| python-docx | DOCX 解析 |
| openpyxl | XLSX 解析 |
| python-pptx | PPTX 解析 |
| xlrd | XLS 旧格式解析 |
| striprtf | RTF 转纯文本 |
| olefile | DOC/PPT OLE 流提取 |
| Pillow | 图片打开 + EXIF 元数据提取 |
| paddleocr | 图片 OCR 文字识别（中文优化） |
| paddlepaddle | PaddleOCR 后端引擎（CPU 版） |

### 前端（package.json）

| 包 | 用途 |
|----|------|
| vue | 3 | 框架 |
| element-plus | UI 组件 |
| @tauri-apps/api | Tauri 前端 SDK |
| @tauri-apps/plugin-dialog | 文件选择器 |
| vite | 构建工具 |
| typescript | 类型检查 |

---

## 十三、构建与打包

### 方式一：GitHub Actions 云编译（推荐）

无需本地安装 Rust/MSVC，在 GitHub 云端 Windows 环境编译。

**工作流文件**：`.github/workflows/build.yml`

**流程**：
1. setup Node.js 22 + Python 3.11 + Rust stable
2. pip install 依赖 + PyInstaller
3. PyInstaller 打包 sidecar → `parse_server.exe`
4. npm install + `npm run tauri build`
5. 复制 sidecar 到 release 目录
6. 上传 Artifacts（work-kb.exe + sidecar + NSIS/MSI 安装包）

**触发方式**：
- 手动：Actions → Build Windows EXE → Run workflow
- 自动：push tag `v*` 时触发

**产物**：
- `work-kb.exe` — 主程序
- `sidecar/parse_server.exe` — Python 解析引擎
- `bundle/nsis/*.exe` — NSIS 安装包
- `bundle/msi/*.msi` — MSI 安装包

### 方式二：本地打包（build.bat）

需要安装：Node.js + Python + Rust + MSVC Build Tools

**流程**：
1. pip install 依赖 + PyInstaller
2. PyInstaller 打包 sidecar
3. npm install
4. `npm run tauri build`
5. 复制 sidecar 到 release 目录

### 终端用户要求

- Windows 10/11
- WebView2 Runtime（Win11 自带，Win10 可能需安装）
- 无需 Python / Rust / Node.js

### sidecar 路径解析（parser.rs）

优先级：
1. 环境变量 `WORKKB_SIDECAR_PATH`
2. exe 同目录 `sidecar/parse_server.exe`（打包模式）
3. 开发路径 `src-tauri/sidecar/parse_server.py`（开发模式）

打包模式直接运行 `.exe`（内含 Python 运行时），开发模式通过 `python` 运行 `.py`。

---

## 十四、编码修复记录

| 问题 | 原因 | 修复 |
|------|------|------|
| 图标文件缺失 | tauri::generate_context! 编译期校验图标 | 用 `tauri icon` 生成全部 5 个图标 |
| jieba cut_for_search 少参数 | jieba-rs 0.7.4 签名变更 | 添加 `hmm: true` 参数 |
| delete_item 不可变引用 | Connection::transaction() 需 &mut self | `&self` → `&mut self`，调用方 `let mut guard` |
| parser.rs clean_html 未用变量 | `lower` 赋值后未用 | 删除冗余赋值 |
| import_data SQL 路径注入 | ATTACH DATABASE 路径含单引号会断 SQL | 路径单引号转义 + null 字节拒绝 |
| get_db_info 表名注入 | format! 拼接表名到 COUNT 查询 | 添加 `is_valid_identifier` 校验 |
| maybe_split_chunks 字节长度 bug | `body.len()` 返回字节数，中文 UTF-8 占 3 字节 | 改用 `chars().count()` 按字符数判断 |
| SUPPORTED_EXTS 三处重复 | db.rs / parser.rs / api.ts 各维护一份 | 提取为 parser.rs `pub const`，db.rs 引用，api.ts 注释同步 |
| quant 正则单元列表三重重复 | 3 个正则函数各硬编码同一份 30+ 单位后缀 | 提取为 `QUANT_UNITS` 常量，format! 拼接 |
| db.rs God Object（666 行） | 8 个结构体 + 15 个方法 + 辅助函数全挤一个文件 | 提取结构体到 models.rs，db.rs 降至 ~580 行 |
| scan_dir_recursive 破坏封装 | 直接访问 db.conn 私有字段 | 新增 `find_file_by_path` 方法封装路径查询 |
| search_items SQL 双重重复 | FTS 和非 FTS 两条 SQL 各写一份 SELECT/JOIN | 提取 `SEARCH_COLS` / `SEARCH_JOINS` 常量，format! 拼接 |

---

## 十五、代码复盘与架构调整（2026-09-05）

### 复盘范围

对全部 6 个 Rust 模块（lib.rs / db.rs / parser.rs / extractor.rs / searcher.rs / exporter.rs）+ SQL 建库脚本 + 前端 api.ts 进行完整代码审查，识别安全问题、正确性 Bug、架构问题和代码重复。

### 安全修复

| 问题 | 风险 | 修复方案 |
|------|------|----------|
| `import_data` SQL 注入 | ATTACH DATABASE 用 `format!` 拼接路径，单引号转义不充分 + null 字节可截断 | 添加 null 字节拒绝 + 保留单引号转义 |
| `get_db_info` 表名注入 | `format!` 拼接 sqlite_master 返回的表名到 COUNT 查询 | 新增 `is_valid_identifier` 校验（仅允许字母/数字/下划线/中文） |

### 正确性修复

| 问题 | 影响 | 修复方案 |
|------|------|----------|
| `maybe_split_chunks` 字节长度 vs 字符长度 | `body.len()` 返回字节数，中文 UTF-8 占 3 字节，500 字节实际仅 ~166 个汉字，远低于预期拆分阈值 | 改用 `chars().count()` 按字符数判断 |

### 架构调整

| 问题 | 原设计 | 调整后 |
|------|--------|--------|
| `SUPPORTED_EXTS` 三处重复 | db.rs / parser.rs / api.ts 各维护一份扩展名列表 | 提取为 parser.rs `pub const SUPPORTED_EXTS`，db.rs `use crate::parser::SUPPORTED_EXTS`，api.ts 注释指向 Rust 源 |
| `db.rs` God Object | 666 行，8 个结构体 + 15 个方法 + 辅助函数 | 提取 8 个结构体到 `models.rs`，db.rs 降至 ~580 行 |
| `scan_dir_recursive` 破坏封装 | 直接访问 `db.conn` 私有字段 | 新增 `Database::find_file_by_path` 方法，封装路径查询 |
| `search_items` SQL 双重重复 | FTS 和非 FTS 两条 SQL 各写一份 SELECT 列 + JOIN 子句 | 提取 `SEARCH_COLS` / `SEARCH_JOINS` 常量，`format!` 拼接，三处查询共用 |
| `extractor.rs` quant 正则三重重复 | 3 个正则函数各硬编码同一份 30+ 单位后缀 | 提取 `QUANT_UNITS` 常量，`format!` 拼接 |

### 模块职责变化

新增 `models.rs` 模块（Rust），承载所有 Tauri Command 返回的结构体定义：

| 结构体 | 用途 |
|--------|------|
| `StatsResult` | 知识库统计概览 |
| `ProjectInfo` / `TagInfo` | 筛选下拉列表 |
| `TableInfo` / `DbInfo` | 数据库状态页 |
| `FileInfo` | 源文件清单 |
| `ScannedFile` | 文件夹扫描结果 |
| `RegisterResult` | 文件登记查重结果 |

`db.rs` 从 `models.rs` 引用这些结构体，自身只保留 `Database` 句柄 + 连接管理 + 查询逻辑 + 辅助函数。

### 未调整的已知问题（列入未来优化）

| 问题 | 说明 | 优先级 |
|------|------|--------|
| 错误类型不统一 | parser.rs 有 ParseError 枚举，db.rs 全用 Box<dyn Error>，lib.rs 全转 String，丢失上下文 | P3 |
| Sidecar stderr 未读取 | 子进程 stderr 管道可能满阻塞进程 | P3 |
| 搜索硬编码 limit=200 | 无分页，大量结果时前端需自行处理 | P3 |
| SidecarClient 无重连 | sidecar 进程异常退出后不自动重启 | P3 |

---

## 十六、未来方向

| 方向 | 说明 | 优先级 |
|------|------|--------|
| PDF 标题识别 | 按字体大小/加粗启发式识别标题层级（需替换 pdf-extract） | P2 |
| 位置级回链 | 从文件级精确到页/段落 | P2 |
| LLM 提炼 | 替换/增强规则抽取，支持语义理解 | P2 |
| 项目/标签自动建议 | 从标题/正文提取关键词匹配已有项目名 | P2 |
| 云同步 | user_id/sync_state 字段已预留 | P3 |
| LibreOffice 集成 | 打包 LibreOffice portable，完善 PPT 旧格式 | P3 |
| 编辑距离容错 | 补充 bigram 对短词容错弱的短板 | P3 |

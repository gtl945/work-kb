-- 个人工作知识库 v1 建库脚本（幂等）
-- 规范化：项目/标签独立成表；预留 user_id/sync_state 以备未来云同步。

-- 文件登记（v1 只存路径引用，预留指纹与归档）
CREATE TABLE IF NOT EXISTS files (
  id            INTEGER PRIMARY KEY,
  path          TEXT NOT NULL,
  filename      TEXT NOT NULL,
  ext           TEXT NOT NULL,
  size          INTEGER,
  hash          TEXT,                  -- 预留：内容指纹去重
  imported_at   INTEGER NOT NULL,
  archived      INTEGER DEFAULT 0,     -- 预留：是否已拷贝入库
  archived_path TEXT,                  -- 预留：归档副本路径
  user_id       INTEGER DEFAULT 0,     -- 预留：云同步
  sync_state    INTEGER DEFAULT 0      -- 预留：0=本地,1=待同步,2=已同步
);

-- 项目
CREATE TABLE IF NOT EXISTS projects (
  id         INTEGER PRIMARY KEY,
  name       TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL,
  user_id    INTEGER DEFAULT 0,
  sync_state INTEGER DEFAULT 0
);

-- 标签
CREATE TABLE IF NOT EXISTS tags (
  id         INTEGER PRIMARY KEY,
  name       TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL,
  user_id    INTEGER DEFAULT 0,
  sync_state INTEGER DEFAULT 0
);

-- 条目主表
CREATE TABLE IF NOT EXISTS items (
  id               INTEGER PRIMARY KEY,
  title            TEXT NOT NULL,
  type             TEXT NOT NULL,            -- 完成/成果/问题/亮点
  occur_date       TEXT,                    -- ISO 日期 yyyy-MM-dd
  project_id       INTEGER REFERENCES projects(id),
  points_text      TEXT NOT NULL,           -- 要点正文（原文，未分词）
  quant_value      TEXT,                    -- 量化值，如 效率+30%
  source_file_id   INTEGER REFERENCES files(id),
  evidence_type    TEXT,                    -- 专利/论文/奖项/证书（仅亮点）
  evidence_file_id INTEGER REFERENCES files(id), -- 亮点佐证文件（v1 单文件）
  status           TEXT NOT NULL,           -- 草稿/已确认
  created_at       INTEGER NOT NULL,
  updated_at       INTEGER NOT NULL,
  user_id          INTEGER DEFAULT 0,
  sync_state       INTEGER DEFAULT 0
);

-- 条目-标签 多对多
CREATE TABLE IF NOT EXISTS item_tags (
  item_id INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  tag_id  INTEGER NOT NULL REFERENCES tags(id),
  PRIMARY KEY (item_id, tag_id)
);

-- 筛选用索引
CREATE INDEX IF NOT EXISTS idx_items_type_date ON items(type, occur_date);
CREATE INDEX IF NOT EXISTS idx_items_project     ON items(project_id);
CREATE INDEX IF NOT EXISTS idx_items_evidence    ON items(evidence_type);
CREATE INDEX IF NOT EXISTS idx_items_occur        ON items(occur_date);

-- 全文索引（app 层 jieba 预分词后写入；rowid 显式对齐 items.id）
CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(
  title_seg,       -- jieba 分词后的标题（空格连接）
  points_seg,     -- jieba 分词后的要点正文
  pinyin_full,     -- 全拼，如 zhuanli
  pinyin_initial,  -- 首字母，如 zl
  ngram,           -- bigram 串，用于容错
  tokenize='unicode61'
);

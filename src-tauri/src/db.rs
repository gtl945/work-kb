use crate::extractor::DraftItem;
use crate::searcher::{self, SearchFilters, SearchResult};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// 知识库统计数据（M6）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsResult {
    pub total: i64,
    pub completed: i64,
    pub achievements: i64,
    pub problems: i64,
    pub highlights: i64,
    pub file_count: i64,
    pub project_count: i64,
    pub tag_count: i64,
}

/// 项目信息（M6 筛选用）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub id: i64,
    pub name: String,
    pub item_count: i64,
}

/// 标签信息（M6 筛选用）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagInfo {
    pub id: i64,
    pub name: String,
    pub item_count: i64,
}

/// 数据库信息（数据库状态页）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableInfo {
    pub name: String,
    pub row_count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbInfo {
    pub db_path: String,
    pub db_size: i64,
    pub tables: Vec<TableInfo>,
}

/// 已登记源文件信息（数据库状态页）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub id: i64,
    pub filename: String,
    pub path: String,
    pub ext: String,
    pub size: Option<i64>,
    pub imported_at: i64,
    pub item_count: i64,
}

/// 文件夹扫描结果（文件夹批量导入）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedFile {
    pub path: String,
    pub filename: String,
    pub ext: String,
    pub size: Option<i64>,
    pub already_registered: bool,
    pub file_id: Option<i64>,
}

/// 文件登记结果（含查重信息）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResult {
    pub file_id: i64,
    pub is_new: bool,
    pub duplicate_reason: Option<String>,
}

/// 本地 SQLite 数据库句柄。
pub struct Database {
    conn: Connection,
    db_path: std::path::PathBuf,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(path)?;
        // WAL：写入不阻塞读取；外外键级联生效。
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(Self { conn, db_path: path.to_path_buf() })
    }

    /// 执行建库脚本（幂等，可重复执行）。
    pub fn init_schema(&self) -> Result<(), Box<dyn std::error::Error>> {
        let sql = include_str!("../migrations/001_init.sql");
        self.conn.execute_batch(sql)?;
        Ok(())
    }

    /// 列出所有表名（用于连接状态自检）。
    pub fn table_names(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let names: Vec<String> = rows.filter_map(|r| r.ok()).collect();
        Ok(names.join(", "))
    }

    /// 登记一个本地文件，计算 SHA-256 哈希并查重。
    /// 路径重复 → 返回已有 id；哈希重复 → 返回已有 id；否则新建。
    pub fn register_file(&self, path: &str) -> Result<RegisterResult, Box<dyn std::error::Error>> {
        let p = Path::new(path);
        let filename = p
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .to_string();
        let ext = p
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        let size = fs::metadata(p).ok().map(|m| m.len() as i64);

        // 路径查重
        let mut stmt = self.conn.prepare("SELECT id FROM files WHERE path = ?1")?;
        if let Ok(existing_id) = stmt.query_row(params![path], |row| row.get::<_, i64>(0)) {
            return Ok(RegisterResult {
                file_id: existing_id,
                is_new: false,
                duplicate_reason: Some("路径已存在".to_string()),
            });
        }

        // 哈希查重
        let hash = compute_file_hash(path);
        if let Some(ref h) = hash {
            let mut stmt2 = self.conn.prepare("SELECT id FROM files WHERE hash = ?1")?;
            if let Ok(existing_id) = stmt2.query_row(params![h], |row| row.get::<_, i64>(0)) {
                return Ok(RegisterResult {
                    file_id: existing_id,
                    is_new: false,
                    duplicate_reason: Some("内容已存在".to_string()),
                });
            }
        }

        self.conn.execute(
            "INSERT INTO files (path, filename, ext, size, hash, imported_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![path, filename, ext, size, hash, now_unix()],
        )?;
        Ok(RegisterResult {
            file_id: self.conn.last_insert_rowid(),
            is_new: true,
            duplicate_reason: None,
        })
    }

    /// 按 id 取文件路径。
    pub fn get_file_path(&self, file_id: i64) -> Result<String, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare("SELECT path FROM files WHERE id = ?1")?;
        let path: String = stmt.query_row(params![file_id], |row| row.get(0))?;
        Ok(path)
    }

    /// 项目不存在则新建，返回 id。
    pub fn ensure_project(&self, name: &str) -> Result<i64, Box<dyn std::error::Error>> {
        self.conn.execute(
            "INSERT OR IGNORE INTO projects (name, created_at) VALUES (?1, ?2)",
            params![name, now_unix()],
        )?;
        let mut stmt = self.conn.prepare("SELECT id FROM projects WHERE name = ?1")?;
        Ok(stmt.query_row(params![name], |r| r.get::<_, i64>(0))?)
    }

    /// 标签不存在则新建，返回 id。
    pub fn ensure_tag(&self, name: &str) -> Result<i64, Box<dyn std::error::Error>> {
        self.conn.execute(
            "INSERT OR IGNORE INTO tags (name, created_at) VALUES (?1, ?2)",
            params![name, now_unix()],
        )?;
        let mut stmt = self.conn.prepare("SELECT id FROM tags WHERE name = ?1")?;
        Ok(stmt.query_row(params![name], |r| r.get::<_, i64>(0))?)
    }

    /// 确认入库：写 items + item_tags + items_fts（M3+M4）。
    pub fn insert_item(&self, draft: &DraftItem) -> Result<i64, Box<dyn std::error::Error>> {
        let project_id = match &draft.project {
            Some(n) if !n.trim().is_empty() => Some(self.ensure_project(n.trim())?),
            _ => None,
        };
        let now = now_unix();
        self.conn.execute(
            "INSERT INTO items
             (title, type, occur_date, project_id, points_text, quant_value,
              source_file_id, evidence_type, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                draft.title,
                draft.item_type,
                draft.occur_date,
                project_id,
                draft.points_text,
                draft.quant_value,
                draft.source_file_id,
                draft.evidence_type,
                "已确认",
                now,
                now,
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        for t in &draft.tags {
            let name = t.trim();
            if name.is_empty() {
                continue;
            }
            let tid = self.ensure_tag(name)?;
            self.conn.execute(
                "INSERT OR IGNORE INTO item_tags (item_id, tag_id) VALUES (?1, ?2)",
                params![id, tid],
            )?;
        }
        // FTS5 全文索引：jieba 分词 + 拼音 + bigram，rowid 对齐 items.id
        let fts = searcher::build_fts_fields(&draft.title, &draft.points_text);
        self.conn.execute(
            "INSERT INTO items_fts (rowid, title_seg, points_seg, pinyin_full, pinyin_initial, ngram)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, &fts.title_seg, &fts.points_seg, &fts.pinyin_full, &fts.pinyin_initial, &fts.ngram],
        )?;
        Ok(id)
    }

    /// 搜索条目：FTS5 MATCH + 筛选条件，返回带项目名/源文件/标签的结果。
    pub fn search_items(
        &self,
        query: &str,
        filters: &SearchFilters,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let match_str = searcher::build_match_str(query);

        let results = if let Some(ms) = &match_str {
            let sql = "SELECT i.id, i.title, i.type, i.occur_date, i.points_text, i.quant_value,
                              i.source_file_id, i.evidence_type,
                              p.name as project_name,
                              f.path as source_file_path, f.filename as source_file_name,
                              GROUP_CONCAT(t.name, ',') as tag_names
                       FROM items_fts ft
                       JOIN items i ON i.id = ft.rowid
                       LEFT JOIN projects p ON p.id = i.project_id
                       LEFT JOIN files f ON f.id = i.source_file_id
                       LEFT JOIN item_tags it ON it.item_id = i.id
                       LEFT JOIN tags t ON t.id = it.tag_id
                       WHERE items_fts MATCH ?1
                         AND (?2 IS NULL OR i.type = ?2)
                         AND (?3 IS NULL OR i.occur_date >= ?3)
                         AND (?4 IS NULL OR i.occur_date <= ?4)
                         AND (?5 IS NULL OR i.evidence_type = ?5)
                         AND (?6 IS NULL OR i.project_id = ?6)
                       GROUP BY i.id
                       ORDER BY i.occur_date DESC
                       LIMIT ?7 OFFSET ?8";
            let mut stmt = self.conn.prepare(sql)?;
            let rows = stmt.query_map(
                params![ms, &filters.item_type, &filters.date_from, &filters.date_to,
                        &filters.evidence_type, &filters.project_id, limit, offset],
                map_search_row,
            )?;
            rows.filter_map(|r| r.ok()).collect()
        } else {
            let sql = "SELECT i.id, i.title, i.type, i.occur_date, i.points_text, i.quant_value,
                              i.source_file_id, i.evidence_type,
                              p.name as project_name,
                              f.path as source_file_path, f.filename as source_file_name,
                              GROUP_CONCAT(t.name, ',') as tag_names
                       FROM items i
                       LEFT JOIN projects p ON p.id = i.project_id
                       LEFT JOIN files f ON f.id = i.source_file_id
                       LEFT JOIN item_tags it ON it.item_id = i.id
                       LEFT JOIN tags t ON t.id = it.tag_id
                       WHERE (?1 IS NULL OR i.type = ?1)
                         AND (?2 IS NULL OR i.occur_date >= ?2)
                         AND (?3 IS NULL OR i.occur_date <= ?3)
                         AND (?4 IS NULL OR i.evidence_type = ?4)
                         AND (?5 IS NULL OR i.project_id = ?5)
                       GROUP BY i.id
                       ORDER BY i.occur_date DESC
                       LIMIT ?6 OFFSET ?7";
            let mut stmt = self.conn.prepare(sql)?;
            let rows = stmt.query_map(
                params![&filters.item_type, &filters.date_from, &filters.date_to,
                        &filters.evidence_type, &filters.project_id, limit, offset],
                map_search_row,
            )?;
            rows.filter_map(|r| r.ok()).collect()
        };

        Ok(results)
    }

    /// 按条目 id 取源文件路径（回链打开用）。
    pub fn get_item_source_path(
        &self,
        item_id: i64,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.path FROM items i
             LEFT JOIN files f ON f.id = i.source_file_id
             WHERE i.id = ?1",
        )?;
        let result = stmt.query_row(params![item_id], |row| {
            let path: Option<String> = row.get(0)?;
            Ok(path)
        });
        match result {
            Ok(path) => Ok(path),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 按日期范围查全部条目（导出用，无分页无 FTS5）。
    pub fn query_items_by_date_range(
        &self,
        date_from: Option<&str>,
        date_to: Option<&str>,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let sql = "SELECT i.id, i.title, i.type, i.occur_date, i.points_text, i.quant_value,
                          i.source_file_id, i.evidence_type,
                          p.name as project_name,
                          f.path as source_file_path, f.filename as source_file_name,
                          GROUP_CONCAT(t.name, ',') as tag_names
                   FROM items i
                   LEFT JOIN projects p ON p.id = i.project_id
                   LEFT JOIN files f ON f.id = i.source_file_id
                   LEFT JOIN item_tags it ON it.item_id = i.id
                   LEFT JOIN tags t ON t.id = it.tag_id
                   WHERE (?1 IS NULL OR i.occur_date >= ?1)
                     AND (?2 IS NULL OR i.occur_date <= ?2)
                   GROUP BY i.id
                   ORDER BY i.occur_date DESC";
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![date_from, date_to], map_search_row)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 删除条目及其关联数据（M6）。
    pub fn delete_item(&mut self, item_id: i64) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM items_fts WHERE rowid = ?1", params![item_id])?;
        tx.execute("DELETE FROM item_tags WHERE item_id = ?1", params![item_id])?;
        tx.execute("DELETE FROM items WHERE id = ?1", params![item_id])?;
        tx.commit()?;
        Ok(())
    }

    /// 获取知识库统计数据（M6）。
    pub fn get_stats(&self) -> Result<StatsResult, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                (SELECT COUNT(*) FROM items) AS total,
                (SELECT COUNT(*) FROM items WHERE type = '完成') AS completed,
                (SELECT COUNT(*) FROM items WHERE type = '成果') AS achievements,
                (SELECT COUNT(*) FROM items WHERE type = '问题') AS problems,
                (SELECT COUNT(*) FROM items WHERE type = '亮点') AS highlights,
                (SELECT COUNT(DISTINCT source_file_id) FROM items WHERE source_file_id IS NOT NULL) AS file_count,
                (SELECT COUNT(DISTINCT project_id) FROM items WHERE project_id IS NOT NULL) AS project_count,
                (SELECT COUNT(DISTINCT tag_id) FROM item_tags) AS tag_count",
        )?;
        let stats = stmt.query_row([], |row| {
            Ok(StatsResult {
                total: row.get(0)?,
                completed: row.get(1)?,
                achievements: row.get(2)?,
                problems: row.get(3)?,
                highlights: row.get(4)?,
                file_count: row.get(5)?,
                project_count: row.get(6)?,
                tag_count: row.get(7)?,
            })
        })?;
        Ok(stats)
    }

    /// 获取所有项目列表（M6 筛选用）。
    pub fn get_projects(&self) -> Result<Vec<ProjectInfo>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.name,
                    (SELECT COUNT(*) FROM items WHERE project_id = p.id) AS item_count
             FROM projects p
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProjectInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                item_count: row.get(2)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 获取所有标签列表（M6 筛选用）。
    pub fn get_tags(&self) -> Result<Vec<TagInfo>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, COUNT(it.item_id) AS item_count
             FROM tags t
             LEFT JOIN item_tags it ON t.id = it.tag_id
             GROUP BY t.id
             ORDER BY t.name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TagInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                item_count: row.get(2)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 获取数据库信息（路径、大小、各表行数）。
    pub fn get_db_info(&self) -> Result<DbInfo, Box<dyn std::error::Error>> {
        let db_path = self.db_path.display().to_string();
        let db_size = fs::metadata(&self.db_path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);

        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master
             WHERE type='table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )?;
        let table_names: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        let mut tables = Vec::with_capacity(table_names.len());
        for name in &table_names {
            let count: i64 = self.conn.query_row(
                &format!("SELECT COUNT(*) FROM \"{}\"", name),
                [],
                |row| row.get(0),
            )?;
            tables.push(TableInfo {
                name: name.clone(),
                row_count: count,
            });
        }

        Ok(DbInfo {
            db_path,
            db_size,
            tables,
        })
    }

    /// 获取所有已登记源文件列表（含关联条目数）。
    pub fn get_file_list(&self) -> Result<Vec<FileInfo>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.filename, f.path, f.ext, f.size, f.imported_at,
                    (SELECT COUNT(*) FROM items WHERE source_file_id = f.id) AS item_count
             FROM files f
             ORDER BY f.imported_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FileInfo {
                id: row.get(0)?,
                filename: row.get(1)?,
                path: row.get(2)?,
                ext: row.get(3)?,
                size: row.get(4)?,
                imported_at: row.get(5)?,
                item_count: row.get(6)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 导出数据库到指定路径（备份）。
    pub fn export_data(&self, target_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.conn
            .execute("PRAGMA wal_checkpoint(TRUNCATE)", [])?;
        std::fs::copy(&self.db_path, target_path)?;
        Ok(())
    }

    /// 从指定路径导入数据库（恢复），覆盖现有全部数据。
    pub fn import_data(&mut self, source_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let escaped = source_path.replace('\'', "''");
        self.conn.execute(
            &format!("ATTACH DATABASE '{}' AS src", escaped),
            [],
        )?;

        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM items_fts", [])?;
        tx.execute("DELETE FROM item_tags", [])?;
        tx.execute("DELETE FROM items", [])?;
        tx.execute("DELETE FROM tags", [])?;
        tx.execute("DELETE FROM projects", [])?;
        tx.execute("DELETE FROM files", [])?;

        tx.execute("INSERT INTO files SELECT * FROM src.files", [])?;
        tx.execute("INSERT INTO projects SELECT * FROM src.projects", [])?;
        tx.execute("INSERT INTO tags SELECT * FROM src.tags", [])?;
        tx.execute("INSERT INTO items SELECT * FROM src.items", [])?;
        tx.execute("INSERT INTO item_tags SELECT * FROM src.item_tags", [])?;
        tx.execute(
            "INSERT INTO items_fts(rowid, title_seg, points_seg, pinyin_full, pinyin_initial, ngram)
             SELECT rowid, title_seg, points_seg, pinyin_full, pinyin_initial, ngram
             FROM src.items_fts",
            [],
        )?;

        tx.commit()?;
        self.conn.execute("DETACH DATABASE src", [])?;
        Ok(())
    }

    /// 递归扫描文件夹，返回所有支持的文件，标注是否已登记（按路径查重）。
    pub fn scan_folder(&self, folder_path: &str) -> Result<Vec<ScannedFile>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        scan_dir_recursive(folder_path, &mut results, self)?;
        Ok(results)
    }

    /// 检查条目是否疑似重复（标题 + 日期相同）。
    pub fn check_item_duplicate(
        &self,
        title: &str,
        occur_date: Option<&str>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let count: i64 = match occur_date {
            Some(date) => self.conn.query_row(
                "SELECT COUNT(*) FROM items WHERE title = ?1 AND occur_date = ?2",
                params![title, date],
                |row| row.get(0),
            )?,
            None => self.conn.query_row(
                "SELECT COUNT(*) FROM items WHERE title = ?1 AND occur_date IS NULL",
                params![title],
                |row| row.get(0),
            )?,
        };
        Ok(count > 0)
    }
}

const SUPPORTED_EXTS: &[&str] = &[
    "pdf", "docx", "xlsx", "pptx", "doc", "xls", "ppt",
    "txt", "csv", "md", "html", "htm", "rtf", "wps", "et", "dps",
];

fn scan_dir_recursive(
    dir: &str,
    results: &mut Vec<ScannedFile>,
    db: &Database,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_dir() {
            scan_dir_recursive(&path.display().to_string(), results, db)?;
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        if !SUPPORTED_EXTS.contains(&ext.as_str()) {
            continue;
        }

        let path_str = path.display().to_string();
        let filename = entry.file_name().to_string_lossy().to_string();
        let size = entry.metadata().ok().map(|m| m.len() as i64);

        // 按路径查重
        let mut stmt = db.conn.prepare("SELECT id FROM files WHERE path = ?1")?;
        let existing_id = stmt
            .query_row(params![&path_str], |row| row.get::<_, i64>(0))
            .ok();

        results.push(ScannedFile {
            path: path_str,
            filename,
            ext,
            size,
            already_registered: existing_id.is_some(),
            file_id: existing_id,
        });
    }
    Ok(())
}

fn compute_file_hash(path: &str) -> Option<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => return None,
        };
        hasher.update(&buf[..n]);
    }
    Some(format!("{:x}", hasher.finalize()))
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 将 SQL 查询行映射为 SearchResult。
fn map_search_row(row: &rusqlite::Row) -> rusqlite::Result<SearchResult> {
    let tag_names: Option<String> = row.get(11)?;
    let tags: Vec<String> = tag_names
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    Ok(SearchResult {
        id: row.get(0)?,
        title: row.get(1)?,
        item_type: row.get(2)?,
        occur_date: row.get(3)?,
        points_text: row.get(4)?,
        quant_value: row.get(5)?,
        source_file_id: row.get(6)?,
        evidence_type: row.get(7)?,
        project_name: row.get(8)?,
        source_file_path: row.get(9)?,
        source_file_name: row.get(10)?,
        tags,
    })
}

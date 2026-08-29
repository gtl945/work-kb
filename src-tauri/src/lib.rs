mod db;
mod exporter;
mod extractor;
mod parser;
mod searcher;

use std::sync::Mutex;
use tauri::Manager;

use db::{ProjectInfo, StatsResult, TagInfo};
use exporter::{ExportParams, ExportResult};
use extractor::DraftItem;
use parser::SidecarClient;
use searcher::{SearchFilters, SearchResult};

/// 应用全局共享：SQLite 连接。
struct DbState(Mutex<db::Database>);
/// Python 解析 sidecar（懒启动）。
struct SidecarState(Mutex<Option<SidecarClient>>);

#[tauri::command]
fn ping() -> &'static str {
    "work-kb v0.1.0"
}

/// 返回库内已建表名，供前端侧栏展示连接状态。
#[tauri::command]
fn db_status(state: tauri::State<DbState>) -> Result<String, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    guard.table_names().map_err(|e| e.to_string())
}

/// 登记本地文件，返回各文件 id（解析前的文件入库）。
#[tauri::command]
fn import_files(paths: Vec<String>, state: tauri::State<DbState>) -> Result<Vec<i64>, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let mut ids = Vec::with_capacity(paths.len());
    for p in paths {
        let id = guard.register_file(&p).map_err(|e| e.to_string())?;
        ids.push(id);
    }
    Ok(ids)
}

/// 解析文件并切块 + 规则抽字段，返回条目草稿（用户确认前可编辑）。
/// 注意：当前为同步命令，大文件解析期间 UI 会短暂冻结，M6 改为异步线程池。
#[tauri::command]
fn parse_file(
    file_id: i64,
    db_state: tauri::State<DbState>,
    sidecar_state: tauri::State<SidecarState>,
) -> Result<Vec<DraftItem>, String> {
    let path = {
        let guard = db_state.0.lock().map_err(|e| e.to_string())?;
        guard.get_file_path(file_id).map_err(|e| e.to_string())?
    };
    let mut guard = sidecar_state.0.lock().map_err(|e| e.to_string())?;
    let pr = parser::dispatch_parse(std::path::Path::new(&path), &mut *guard)
        .map_err(|e| e.to_string())?;
    Ok(extractor::chunk_and_extract(&pr, file_id, &path))
}

/// 确认入库：写 items + item_tags + FTS5 索引。返回入库的条目 id。
#[tauri::command]
fn confirm_items(items: Vec<DraftItem>, state: tauri::State<DbState>) -> Result<Vec<i64>, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let mut ids = Vec::with_capacity(items.len());
    for it in items {
        let id = guard.insert_item(&it).map_err(|e| e.to_string())?;
        ids.push(id);
    }
    Ok(ids)
}

/// 搜索知识库：FTS5 全文检索 + 多维筛选。空查询时浏览全部。
#[tauri::command]
fn search(
    query: String,
    filters: SearchFilters,
    state: tauri::State<DbState>,
) -> Result<Vec<SearchResult>, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    guard
        .search_items(&query, &filters, 200, 0)
        .map_err(|e| e.to_string())
}

/// 用系统默认程序打开条目关联的源文件（回链）。
#[tauri::command]
fn open_source_file(item_id: i64, state: tauri::State<DbState>) -> Result<bool, String> {
    let path = {
        let guard = state.0.lock().map_err(|e| e.to_string())?;
        guard
            .get_item_source_path(item_id)
            .map_err(|e| e.to_string())?
    };
    match path {
        Some(p) => {
            open::that(&p).map_err(|e| e.to_string())?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// 导出多视图报告：按日/周/季/年粒度切片生成 Markdown。
#[tauri::command]
fn export_view(
    params: ExportParams,
    state: tauri::State<DbState>,
) -> Result<ExportResult, String> {
    let items = {
        let guard = state.0.lock().map_err(|e| e.to_string())?;
        guard
            .query_items_by_date_range(
                params.date_from.as_deref(),
                params.date_to.as_deref(),
            )
            .map_err(|e| e.to_string())?
    };
    Ok(exporter::generate_markdown(&items, &params))
}

/// 将 Markdown 保存到本地文件。
#[tauri::command]
fn save_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// 删除条目（M6）。
#[tauri::command]
fn delete_item(item_id: i64, state: tauri::State<DbState>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    guard.delete_item(item_id).map_err(|e| e.to_string())
}

/// 获取知识库统计数据（M6）。
#[tauri::command]
fn get_stats(state: tauri::State<DbState>) -> Result<StatsResult, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    guard.get_stats().map_err(|e| e.to_string())
}

/// 获取所有项目（M6 筛选用）。
#[tauri::command]
fn get_projects(state: tauri::State<DbState>) -> Result<Vec<ProjectInfo>, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    guard.get_projects().map_err(|e| e.to_string())
}

/// 获取所有标签（M6 筛选用）。
#[tauri::command]
fn get_tags(state: tauri::State<DbState>) -> Result<Vec<TagInfo>, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    guard.get_tags().map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db_path = dir.join("workkb.db");
            let database = db::Database::open(&db_path)?;
            database.init_schema()?;
            app.manage(DbState(Mutex::new(database)));
            app.manage(SidecarState(Mutex::new(None)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            db_status,
            import_files,
            parse_file,
            confirm_items,
            search,
            open_source_file,
            export_view,
            save_file,
            delete_item,
            get_stats,
            get_projects,
            get_tags
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

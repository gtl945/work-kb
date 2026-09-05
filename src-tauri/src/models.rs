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

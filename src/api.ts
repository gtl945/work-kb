import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

/// 唤起系统文件选择器，返回用户选中的本地文件路径列表（支持多选）。
export async function pickFiles(): Promise<string[] | null> {
  const selected = await open({
    multiple: true,
    filters: [{ name: "办公文档", extensions: [
      "docx", "xlsx", "pptx", "pdf",
      "doc", "xls", "ppt",
      "txt", "csv", "md", "html", "htm",
      "rtf", "wps", "et", "dps"
   ] }],
  });
  if (!selected) return null;
  if (Array.isArray(selected)) return selected;
  return [selected];
}

export async function pickFolder(): Promise<string | null> {
  const selected = await open({ directory: true });
  if (!selected || Array.isArray(selected)) return null;
  return selected;
}

export async function ping(): Promise<string> {
  return invoke<string>("ping");
}

export async function dbStatus(): Promise<string> {
  return invoke<string>("db_status");
}

export interface RegisterResult {
  fileId: number;
  isNew: boolean;
  duplicateReason: string | null;
}

export async function importFiles(paths: string[]): Promise<RegisterResult[]> {
  return invoke<RegisterResult[]>("import_files", { paths });
}

export interface Section {
  heading: string;
  level: number;
  body: string;
  page: number | null;
}

export interface ParseResult {
  sourcePath: string;
  docTitle: string;
  sections: Section[];
}

export interface DraftItem {
  title: string;
  itemType: string;
  occurDate: string | null;
  project: string | null;
  pointsText: string;
  quantValue: string | null;
  sourceFileId: number;
  evidenceType: string | null;
  tags: string[];
  isFallback?: boolean;
}

export async function parseFile(fileId: number): Promise<DraftItem[]> {
  return invoke<DraftItem[]>("parse_file", { fileId });
}

export async function confirmItems(items: DraftItem[]): Promise<number[]> {
  return invoke<number[]>("confirm_items", { items });
}

export interface SearchFilters {
  itemType?: string | null;
  dateFrom?: string | null;
  dateTo?: string | null;
  evidenceType?: string | null;
  projectId?: number | null;
}

export interface SearchResult {
  id: number;
  title: string;
  itemType: string;
  occurDate: string | null;
  projectName: string | null;
  pointsText: string;
  quantValue: string | null;
  sourceFileId: number | null;
  sourceFilePath: string | null;
  sourceFileName: string | null;
  evidenceType: string | null;
  tags: string[];
}

export async function searchItems(
  query: string,
  filters: SearchFilters
): Promise<SearchResult[]> {
  return invoke<SearchResult[]>("search", { query, filters });
}

export async function openSourceFile(itemId: number): Promise<boolean> {
  return invoke<boolean>("open_source_file", { itemId });
}

export type ExportGranularity = "daily" | "weekly" | "quarterly" | "yearly";

export interface ExportParams {
  granularity: ExportGranularity;
  dateFrom?: string | null;
  dateTo?: string | null;
}

export interface ExportResult {
  markdown: string;
  fileList: string[];
  itemCount: number;
}

export async function exportView(params: ExportParams): Promise<ExportResult> {
  return invoke<ExportResult>("export_view", { params });
}

export async function saveFile(path: string, content: string): Promise<void> {
  await invoke<void>("save_file", { path, content });
}

export async function pickSavePath(defaultName: string): Promise<string | null> {
  const path = await save({
    defaultPath: defaultName,
    filters: [{ name: "Markdown", extensions: ["md"] }],
  });
  return path ?? null;
}

// ---- M6: 数据管理 / 统计 ----

export interface StatsResult {
  total: number;
  completed: number;
  achievements: number;
  problems: number;
  highlights: number;
  fileCount: number;
  projectCount: number;
  tagCount: number;
}

export interface ProjectInfo {
  id: number;
  name: string;
  itemCount: number;
}

export interface TagInfo {
  id: number;
  name: string;
  itemCount: number;
}

export async function deleteItem(itemId: number): Promise<void> {
  await invoke<void>("delete_item", { itemId });
}

export async function getStats(): Promise<StatsResult> {
  return invoke<StatsResult>("get_stats");
}

export async function getProjects(): Promise<ProjectInfo[]> {
  return invoke<ProjectInfo[]>("get_projects");
}

export async function getTags(): Promise<TagInfo[]> {
  return invoke<TagInfo[]>("get_tags");
}

// ---- 数据库状态页 ----

export interface TableInfo {
  name: string;
  rowCount: number;
}

export interface DbInfo {
  dbPath: string;
  dbSize: number;
  tables: TableInfo[];
}

export interface SourceFileInfo {
  id: number;
  filename: string;
  path: string;
  ext: string;
  size: number | null;
  importedAt: number;
  itemCount: number;
}

export async function getDbInfo(): Promise<DbInfo> {
  return invoke<DbInfo>("get_db_info");
}

export async function getFileList(): Promise<SourceFileInfo[]> {
  return invoke<SourceFileInfo[]>("get_file_list");
}

export async function exportData(targetPath: string): Promise<void> {
  await invoke<void>("export_data", { targetPath });
}

export async function importData(sourcePath: string): Promise<void> {
  await invoke<void>("import_data", { sourcePath });
}

export async function pickDbSavePath(): Promise<string | null> {
  const path = await save({
    defaultPath: "workkb-backup.db",
    filters: [{ name: "SQLite Database", extensions: ["db"] }],
  });
  return path ?? null;
}

export async function pickDbOpenPath(): Promise<string | null> {
  const selected = await open({
    multiple: false,
    filters: [{ name: "SQLite Database", extensions: ["db"] }],
  });
  if (!selected) return null;
  return Array.isArray(selected) ? selected[0] : selected;
}

// ---- 文件夹批量导入 + 查重 ----

export interface ScannedFile {
  path: string;
  filename: string;
  ext: string;
  size: number | null;
  alreadyRegistered: boolean;
  fileId: number | null;
}

export async function scanFolder(folderPath: string): Promise<ScannedFile[]> {
  return invoke<ScannedFile[]>("scan_folder", { folderPath });
}

export async function checkItemDuplicate(
  title: string,
  occurDate: string | null
): Promise<boolean> {
  return invoke<boolean>("check_item_duplicate", { title, occurDate });
}

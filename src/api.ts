import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

/// 唤起系统文件选择器，返回用户选中的本地文件路径列表（支持多选）。
export async function pickFiles(): Promise<string[] | null> {
  const selected = await open({
    multiple: true,
    filters: [{ name: "办公文档", extensions: ["docx", "xlsx", "pptx", "pdf"] }],
  });
  if (!selected) return null;
  if (Array.isArray(selected)) return selected;
  return [selected];
}

export async function ping(): Promise<string> {
  return invoke<string>("ping");
}

export async function dbStatus(): Promise<string> {
  return invoke<string>("db_status");
}

export async function importFiles(paths: string[]): Promise<number[]> {
  return invoke<number[]>("import_files", { paths });
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

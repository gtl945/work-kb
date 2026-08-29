use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

use crate::parser::ParseResult;

/// 解析后的条目草稿：用户在确认入库前可编辑各字段。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftItem {
    pub title: String,
    #[serde(default)]
    pub item_type: String,
    #[serde(default)]
    pub occur_date: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    pub points_text: String,
    #[serde(default)]
    pub quant_value: Option<String>,
    pub source_file_id: i64,
    #[serde(default)]
    pub evidence_type: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn body_date_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(\d{4})[-/年](\d{1,2})[-/月](\d{1,2})").unwrap())
}

fn fname_date_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(\d{4})[-_]?(\d{2})[-_]?(\d{2})").unwrap())
}

fn quant_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"\d+(?:\.\d+)?\s*(?:%|‰|倍|个|件|次|条|项|篇|份|人|天|日|周|月|年|小时|分钟|秒|元|万|亿|分|度|名|页|章)")
            .unwrap()
    })
}

fn valid_ymd(y: u32, m: u32, d: u32) -> bool {
    (1..=12).contains(&m) && (1..=31).contains(&d) && y > 1900
}

fn fmt_date(y: u32, m: u32, d: u32) -> String {
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// 从正文抽日期（2026-08-29 / 2026/8/29 / 2026年8月29日）。
fn extract_body_date(text: &str) -> Option<String> {
    let c = body_date_regex().captures(text)?;
    let y = c.get(1)?.as_str().parse::<u32>().ok()?;
    let m = c.get(2)?.as_str().parse::<u32>().ok()?;
    let d = c.get(3)?.as_str().parse::<u32>().ok()?;
    if !valid_ymd(y, m, d) {
        return None;
    }
    Some(fmt_date(y, m, d))
}

/// 从文件名抽日期（20260829 / 2026_08_29 / 2026-08-29）。
fn extract_fname_date(path: &str) -> Option<String> {
    let stem = Path::new(path).file_stem()?.to_str()?;
    let c = fname_date_regex().captures(stem)?;
    let y = c.get(1)?.as_str().parse::<u32>().ok()?;
    let m = c.get(2)?.as_str().parse::<u32>().ok()?;
    let d = c.get(3)?.as_str().parse::<u32>().ok()?;
    if !valid_ymd(y, m, d) {
        return None;
    }
    Some(fmt_date(y, m, d))
}

/// 从正文抽"数字+单位"量化值（如 30% / 2小时）。
fn extract_quant(text: &str) -> Option<String> {
    Some(quant_regex().find(text)?.as_str().trim().to_string())
}

/// 切块 + 规则字段抽取：
/// - 标题层级切块；全无标题则整篇一条（PDF/无标题文档）。
/// - 抽时间（正文优先，回退文件名）、量化值；类型默认"完成"，项目/标签由用户补。
pub fn chunk_and_extract(
    parse_result: &ParseResult,
    source_file_id: i64,
    file_path: &str,
) -> Vec<DraftItem> {
    let sections = &parse_result.sections;
    let doc_title = &parse_result.doc_title;

    let chunks: Vec<(String, String)> = if sections.iter().all(|s| s.heading.is_empty()) {
        let body = sections
            .iter()
            .map(|s| s.body.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        vec![(doc_title.clone(), body)]
    } else {
        sections
            .iter()
            .map(|s| {
                let title = if s.heading.is_empty() {
                    doc_title.clone()
                } else {
                    s.heading.clone()
                };
                (title, s.body.clone())
            })
            .collect()
    };

    chunks
        .into_iter()
        .map(|(title, body)| {
            let occur_date =
                extract_body_date(&body).or_else(|| extract_fname_date(file_path));
            let quant_value = extract_quant(&body);
            DraftItem {
                title,
                item_type: "完成".to_string(),
                occur_date,
                project: None,
                points_text: body,
                quant_value,
                source_file_id,
                evidence_type: None,
                tags: Vec::new(),
            }
        })
        .collect()
}

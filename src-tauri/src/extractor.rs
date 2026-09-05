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
    #[serde(default)]
    pub is_fallback: bool,
}

fn body_date_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(\d{4})[-/年.](\d{1,2})(?:[-/月.](\d{1,2}))?").unwrap()
    })
}

fn fname_date_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(\d{4})[-_]?(\d{2})[-_]?(\d{2})").unwrap())
}

/// 量化值单位后缀（统一定义，避免三处正则各写一份）。
const QUANT_UNITS: &str =
    r"(?:%|‰|倍|个|件|次|条|项|篇|份|人|天|日|周|月|年|小时|分钟|秒|元|万|亿|分|度|名|页|章)";

fn quant_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(&format!(r"\d+(?:\.\d+)?\s*{}", QUANT_UNITS)).unwrap()
    })
}

fn quant_range_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(&format!(
            r"\d+(?:\.\d+)?\s*[-~到]\d+(?:\.\d+)?\s*{}",
            QUANT_UNITS
        ))
        .unwrap()
    })
}

fn quant_approx_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(&format!(
            r"(?:约|大约|近|超|逾)\s*\d+(?:\.\d+)?\s*{}",
            QUANT_UNITS
        ))
        .unwrap()
    })
}

fn valid_ymd(y: u32, m: u32, d: u32) -> bool {
    (1..=12).contains(&m) && (1..=31).contains(&d) && y > 1900
}

fn fmt_date(y: u32, m: u32, d: u32) -> String {
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// 从正文抽日期（2026-08-29 / 2026/8/29 / 2026年8月29日 / 2026.08.29 / 2026.08）。
fn extract_body_date(text: &str) -> Option<String> {
    let c = body_date_regex().captures(text)?;
    let y = c.get(1)?.as_str().parse::<u32>().ok()?;
    let m = c.get(2)?.as_str().parse::<u32>().ok()?;
    if !(1..=12).contains(&m) || y <= 1900 {
        return None;
    }
    if let Some(d_match) = c.get(3) {
        let d = d_match.as_str().parse::<u32>().ok()?;
        if !(1..=31).contains(&d) {
            return None;
        }
        Some(fmt_date(y, m, d))
    } else {
        Some(format!("{:04}-{:02}", y, m))
    }
}

/// 从文件名抽日期（20260829 / 2026_08_29 / 2026-08-29）。
pub fn extract_fname_date(path: &str) -> Option<String> {
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

/// 从正文抽量化值：优先范围（3-5个）→ 约数（约20%）→ 普通（30%）。
fn extract_quant(text: &str) -> Option<String> {
    if let Some(m) = quant_range_regex().find(text) {
        return Some(m.as_str().trim().to_string());
    }
    if let Some(m) = quant_approx_regex().find(text) {
        return Some(m.as_str().trim().to_string());
    }
    Some(quant_regex().find(text)?.as_str().trim().to_string())
}

/// 关键词规则分类条目类型：亮点 > 成果 > 问题 > 完成。
fn classify_item_type(title: &str, body: &str) -> &'static str {
    let text = format!("{} {}", title, body);
    let highlight_kw = ["获奖", "专利", "发表", "荣誉", "表彰", "证书", "第一", "首创", "突破", "优秀", "标兵"];
    if highlight_kw.iter().any(|k| text.contains(k)) {
        return "亮点";
    }
    let achievement_kw = ["完成", "实现", "达成", "交付", "上线", "验收", "签署", "获批", "通过"];
    if achievement_kw.iter().any(|k| text.contains(k)) {
        return "成果";
    }
    let problem_kw = ["问题", "困难", "挑战", "瓶颈", "障碍", "延期", "失败", "风险", "不足"];
    if problem_kw.iter().any(|k| text.contains(k)) {
        return "问题";
    }
    "完成"
}

/// 超长 section 拆分：body > 500 字时按段落/换行二次切分。
fn maybe_split_chunks(chunks: Vec<(String, String)>) -> Vec<(String, String)> {
    const MAX_LEN: usize = 500;
    let char_len = |s: &str| s.chars().count();
    let mut result = Vec::new();
    for (title, body) in chunks {
        if char_len(&body) <= MAX_LEN {
            result.push((title, body));
            continue;
        }
        let paras: Vec<&str> = body.split("\n\n").collect();
        if paras.len() > 1 {
            let mut cur = String::new();
            for para in paras {
                if !cur.is_empty() && char_len(&cur) + char_len(para) > MAX_LEN {
                    result.push((title.clone(), cur.trim().to_string()));
                    cur.clear();
                }
                cur.push_str(para);
                cur.push_str("\n\n");
            }
            if !cur.trim().is_empty() {
                result.push((title.clone(), cur.trim().to_string()));
            }
        } else {
            let lines: Vec<&str> = body.split('\n').collect();
            let mut cur = String::new();
            for line in lines {
                if !cur.is_empty() && char_len(&cur) + char_len(line) > MAX_LEN {
                    result.push((title.clone(), cur.trim().to_string()));
                    cur.clear();
                }
                cur.push_str(line);
                cur.push('\n');
            }
            if !cur.trim().is_empty() {
                result.push((title.clone(), cur.trim().to_string()));
            }
        }
    }
    result
}

/// 切块 + 规则字段抽取：
/// - 标题层级切块；全无标题则整篇一条（PDF/无标题文档）。
/// - 超长 section（>500 字）按段落/换行二次拆分。
/// - 抽时间（正文优先，回退文件名）、量化值（范围/约数/普通）；类型关键词智能分类。
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

    let chunks = maybe_split_chunks(chunks);

    chunks
        .into_iter()
        .map(|(title, body)| {
            let occur_date =
                extract_body_date(&body).or_else(|| extract_fname_date(file_path));
            let quant_value = extract_quant(&body);
            let item_type = classify_item_type(&title, &body).to_string();
            DraftItem {
                title,
                item_type,
                occur_date,
                project: None,
                points_text: body,
                quant_value,
                source_file_id,
                evidence_type: None,
                tags: Vec::new(),
                is_fallback: false,
            }
        })
        .collect()
}

/// 解析失败或零草稿时的兜底草稿：文件名做标题，确保文件可搜索。
pub fn create_fallback_draft(
    file_path: &str,
    source_file_id: i64,
    error_msg: &str,
) -> DraftItem {
    let stem = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("未知文件")
        .to_string();
    let occur_date = extract_fname_date(file_path);
    DraftItem {
        title: stem,
        item_type: "完成".to_string(),
        occur_date,
        project: None,
        points_text: format!("（解析失败：{}。标题来自文件名，请手动补充内容）", error_msg),
        quant_value: None,
        source_file_id,
        evidence_type: None,
        tags: Vec::new(),
        is_fallback: true,
    }
}

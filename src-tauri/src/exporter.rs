use crate::searcher::SearchResult;
use chrono::{Datelike, NaiveDate};
use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportGranularity {
    Daily,
    Weekly,
    Quarterly,
    Yearly,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportParams {
    pub granularity: ExportGranularity,
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub markdown: String,
    pub file_list: Vec<String>,
    pub item_count: usize,
}

const TYPE_ORDER: [&str; 4] = ["完成", "成果", "问题", "亮点"];

pub fn generate_markdown(items: &[SearchResult], params: &ExportParams) -> ExportResult {
    let mut files: Vec<String> = items
        .iter()
        .filter_map(|i| i.source_file_name.clone())
        .collect();
    files.sort();
    files.dedup();

    let mut type_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for item in items {
        *type_counts.entry(item.item_type.as_str()).or_default() += 1;
    }

    let mut groups: BTreeMap<String, Vec<&SearchResult>> = BTreeMap::new();
    for item in items {
        let key = item
            .occur_date
            .as_ref()
            .map(|d| group_key(d, &params.granularity))
            .unwrap_or_else(|| "未标注日期".to_string());
        groups.entry(key).or_default().push(item);
    }

    let mut md = String::new();
    let title = match params.granularity {
        ExportGranularity::Daily => "工作日报",
        ExportGranularity::Weekly => "工作周报",
        ExportGranularity::Quarterly => "工作季报",
        ExportGranularity::Yearly => "工作年报",
    };
    md.push_str(&format!("# {}\n\n", title));

    if let (Some(from), Some(to)) = (&params.date_from, &params.date_to) {
        md.push_str(&format!("> 时间范围: {} ~ {}\n\n", from, to));
    }

    md.push_str("## 概览\n\n");
    md.push_str(&format!("- 总计: {} 项\n", items.len()));
    let parts: Vec<String> = TYPE_ORDER
        .iter()
        .filter_map(|t| type_counts.get(t).map(|c| format!("{}: {} 项", t, c)))
        .collect();
    if !parts.is_empty() {
        md.push_str(&format!("- {}\n", parts.join(" | ")));
    }
    md.push('\n');

    if !files.is_empty() {
        md.push_str(&format!("**源文件**: {}\n\n", files.join(", ")));
    }

    for (key, group_items) in &groups {
        md.push_str(&format!("## {}\n\n", group_title(key, &params.granularity)));
        for type_name in &TYPE_ORDER {
            let typed: Vec<&&SearchResult> =
                group_items.iter().filter(|i| i.item_type == *type_name).collect();
            if typed.is_empty() {
                continue;
            }
            md.push_str(&format!("### {} ({})\n\n", type_name, typed.len()));
            for item in typed {
                md.push_str(&format_item(
                    item,
                    !matches!(params.granularity, ExportGranularity::Daily),
                ));
                md.push('\n');
            }
            md.push('\n');
        }
    }

    ExportResult {
        markdown: md,
        file_list: files,
        item_count: items.len(),
    }
}

fn group_key(date_str: &str, g: &ExportGranularity) -> String {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d");
    match date {
        Ok(d) => match g {
            ExportGranularity::Daily => date_str.to_string(),
            ExportGranularity::Weekly => {
                let w = d.iso_week();
                format!("{:04}-W{:02}", w.year(), w.week())
            }
            ExportGranularity::Quarterly => {
                let q = (d.month() - 1) / 3 + 1;
                format!("{:04}-Q{}", d.year(), q)
            }
            ExportGranularity::Yearly => format!("{:04}", d.year()),
        },
        Err(_) => "未标注日期".to_string(),
    }
}

fn group_title(key: &str, g: &ExportGranularity) -> String {
    if key == "未标注日期" {
        return key.to_string();
    }
    match g {
        ExportGranularity::Daily => key.to_string(),
        ExportGranularity::Weekly => {
            let parts: Vec<&str> = key.splitn(2, "-W").collect();
            if parts.len() == 2 {
                let year: i32 = parts[0].parse().unwrap_or(0);
                let week: u32 = parts[1].parse().unwrap_or(0);
                if year > 0 && week > 0 {
                    return format!("{}年第{}周", year, week);
                }
            }
            key.to_string()
        }
        ExportGranularity::Quarterly => {
            if key.len() >= 7 {
                let year = &key[..4];
                let q = &key[5..];
                let (m_start, m_end) = match q {
                    "Q1" => ("01", "03"),
                    "Q2" => ("04", "06"),
                    "Q3" => ("07", "09"),
                    "Q4" => ("10", "12"),
                    _ => return key.to_string(),
                };
                format!("{}年{} ({}月 ~ {}月)", year, q, m_start, m_end)
            } else {
                key.to_string()
            }
        }
        ExportGranularity::Yearly => format!("{}年", key),
    }
}

fn format_item(item: &SearchResult, show_date: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    if show_date {
        if let Some(ref date) = item.occur_date {
            let short = if date.len() >= 10 { &date[5..10] } else { date };
            parts.push(format!("[{}]", short));
        }
    }
    parts.push(item.title.clone());
    if let Some(ref p) = item.project_name {
        parts.push(format!("[项目: {}]", p));
    }
    if let Some(ref q) = item.quant_value {
        parts.push(format!("[量化: {}]", q));
    }
    if let Some(ref e) = item.evidence_type {
        parts.push(format!("[证据: {}]", e));
    }
    for tag in &item.tags {
        parts.push(format!("#{}", tag));
    }
    let header = parts.join(" ");

    let chars: Vec<char> = item.points_text.chars().collect();
    if chars.is_empty() {
        format!("- {}", header)
    } else if chars.len() > 120 {
        let body: String = chars[..120].iter().collect();
        format!("- {}\n  > {}...", header, body.trim())
    } else {
        let body: String = chars.iter().collect();
        format!("- {}\n  > {}", header, body.trim())
    }
}

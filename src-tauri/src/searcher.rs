use jieba_rs::Jieba;
use pinyin::ToPinyin;
use std::sync::OnceLock;

static JIEBA: OnceLock<Jieba> = OnceLock::new();

fn jieba() -> &'static Jieba {
    JIEBA.get_or_init(Jieba::new)
}

/// jieba 搜索模式分词，返回 token 列表。
fn segment(text: &str) -> Vec<String> {
    jieba()
        .cut_for_search(text, true)
        .iter()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// 中文 → 全拼（无声调），非中文字符跳过。
fn to_pinyin_full(text: &str) -> String {
    let mut out = String::new();
    for py_opt in text.to_pinyin() {
        if let Some(py) = py_opt {
            out.push_str(py.plain());
        }
    }
    out
}

/// 中文 → 首字母，非中文字符跳过。
fn to_pinyin_initial(text: &str) -> String {
    let mut out = String::new();
    for py_opt in text.to_pinyin() {
        if let Some(py) = py_opt {
            if let Some(c) = py.plain().chars().next() {
                out.push(c);
            }
        }
    }
    out
}

/// 字符级 bigram 生成（容错匹配用）。
fn to_bigrams(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.is_empty() {
        return vec![];
    }
    if chars.len() < 2 {
        return vec![chars.iter().collect()];
    }
    chars
        .windows(2)
        .map(|w| format!("{}{}", w[0], w[1]))
        .collect()
}

/// FTS5 索引列集合：入库时预计算写入 items_fts。
pub struct FtsFields {
    pub title_seg: String,
    pub points_seg: String,
    pub pinyin_full: String,
    pub pinyin_initial: String,
    pub ngram: String,
}

/// 为入库条目计算全部 FTS5 列。
/// 拼音按词生成、空格分隔，使前缀匹配可命中单个词的拼音。
pub fn build_fts_fields(title: &str, points: &str) -> FtsFields {
    let title_words = segment(title);
    let points_words = segment(points);

    let pinyin_words: Vec<String> = title_words
        .iter()
        .chain(points_words.iter())
        .map(|w| to_pinyin_full(w))
        .filter(|s| !s.is_empty())
        .collect();

    let initial_words: Vec<String> = title_words
        .iter()
        .chain(points_words.iter())
        .map(|w| to_pinyin_initial(w))
        .filter(|s| !s.is_empty())
        .collect();

    let combined = format!("{} {}", title, points);

    FtsFields {
        title_seg: title_words.join(" "),
        points_seg: points_words.join(" "),
        pinyin_full: pinyin_words.join(" "),
        pinyin_initial: initial_words.join(" "),
        ngram: to_bigrams(&combined).join(" "),
    }
}

/// FTS5 双引号转义。
fn escape_fts(s: &str) -> String {
    s.replace('"', "\"\"")
}

/// 从用户查询构造 FTS5 MATCH 串。
/// 多词 AND 串联，单词跨列 OR，支持关键词/拼音前缀/bigram 容错。
/// 返回 None 表示无条件（浏览全部条目）。
pub fn build_match_str(query: &str) -> Option<String> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }

    let words = segment(q);
    if words.is_empty() {
        return None;
    }

    let clauses: Vec<String> = words
        .iter()
        .filter(|w| !w.is_empty())
        .map(|word| {
            let pf = to_pinyin_full(word);
            let pi = to_pinyin_initial(word);
            let ngs = to_bigrams(word);
            let we = escape_fts(word);

            let mut parts: Vec<String> = vec![
                format!("title_seg:\"{}\"", we),
                format!("points_seg:\"{}\"", we),
            ];

            if !pf.is_empty() {
                parts.push(format!("pinyin_full:{}*", pf));
            }
            if !pi.is_empty() {
                parts.push(format!("pinyin_initial:{}*", pi));
            }
            for ng in &ngs {
                parts.push(format!("ngram:\"{}\"", escape_fts(ng)));
            }

            format!("({})", parts.join(" OR "))
        })
        .collect();

    if clauses.is_empty() {
        return None;
    }
    Some(clauses.join(" AND "))
}

/// 搜索筛选条件。
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchFilters {
    #[serde(default)]
    pub item_type: Option<String>,
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    #[serde(default)]
    pub evidence_type: Option<String>,
    #[serde(default)]
    pub project_id: Option<i64>,
}

/// 搜索结果条目。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: i64,
    pub title: String,
    pub item_type: String,
    pub occur_date: Option<String>,
    pub project_name: Option<String>,
    pub points_text: String,
    pub quant_value: Option<String>,
    pub source_file_id: Option<i64>,
    pub source_file_path: Option<String>,
    pub source_file_name: Option<String>,
    pub evidence_type: Option<String>,
    pub tags: Vec<String>,
}

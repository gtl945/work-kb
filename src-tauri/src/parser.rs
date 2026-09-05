use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// 一段文档内容（按标题切分；PDF 无标题则整篇一条，level=0）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub heading: String,
    pub level: u32,
    pub body: String,
    #[serde(default)]
    pub page: Option<u32>,
}

/// 统一解析结果：无论 PDF（Rust 原生）还是 Office（Python sidecar），都产出此结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    pub source_path: String,
    pub doc_title: String,
    pub sections: Vec<Section>,
}

#[derive(Debug)]
pub enum ParseError {
    Unsupported(String),
    Pdf(String),
    Sidecar(String),
    Io(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Unsupported(s) => write!(f, "不支持的文件类型: {}", s),
            ParseError::Pdf(s) => write!(f, "PDF 解析失败: {}", s),
            ParseError::Sidecar(s) => write!(f, "Python sidecar 错误: {}", s),
            ParseError::Io(s) => write!(f, "IO 错误: {}", s),
        }
    }
}
impl std::error::Error for ParseError {}

/// PDF：Rust 原生（pdf-extract）。无标题层级，整体作为一条 level 0。
pub fn parse_pdf(path: &Path) -> Result<ParseResult, ParseError> {
    let text =
        pdf_extract::extract_text(path).map_err(|e| ParseError::Pdf(e.to_string()))?;
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("PDF")
        .to_string();
    Ok(ParseResult {
        source_path: path.to_string_lossy().to_string(),
        doc_title: title,
        sections: vec![Section {
            heading: String::new(),
            level: 0,
            body: text,
            page: None,
        }],
    })
}

/// TXT：Rust 原生读取，按空行分段。优先 UTF-8，回退 GBK。
pub fn parse_txt(path: &Path) -> Result<ParseResult, ParseError> {
    let bytes = std::fs::read(path).map_err(|e| ParseError::Io(e.to_string()))?;
    let content = decode_text(&bytes);
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("TXT")
        .to_string();
    let sections = split_by_paragraphs(&content);
    Ok(ParseResult {
        source_path: path.to_string_lossy().to_string(),
        doc_title: title,
        sections,
    })
}

/// MD：Rust 原生，按 # 标题分层。
pub fn parse_md(path: &Path) -> Result<ParseResult, ParseError> {
    let bytes = std::fs::read(path).map_err(|e| ParseError::Io(e.to_string()))?;
    let content = decode_text(&bytes);
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("MD")
        .to_string();
    let mut sections: Vec<Section> = Vec::new();
    let mut cur_heading = String::new();
    let mut cur_level: u32 = 0;
    let mut cur_body = String::new();
    let mut started = false;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            if started {
                sections.push(Section {
                    heading: cur_heading.clone(),
                    level: cur_level,
                    body: cur_body.trim().to_string(),
                    page: None,
                });
                cur_body.clear();
            }
            let hashes = trimmed.chars().take_while(|&c| c == '#').count();
            cur_level = hashes as u32;
            cur_heading = trimmed[hashes..].trim().to_string();
            started = true;
        } else if !trimmed.is_empty() {
            cur_body.push_str(line);
            cur_body.push('\n');
            started = true;
        }
    }
    if started {
        sections.push(Section {
            heading: cur_heading,
            level: cur_level,
            body: cur_body.trim().to_string(),
            page: None,
        });
    }
    if sections.is_empty() {
        sections.push(Section {
            heading: title.clone(),
            level: 0,
            body: content,
            page: None,
        });
    }
    Ok(ParseResult {
        source_path: path.to_string_lossy().to_string(),
        doc_title: title,
        sections,
    })
}

/// CSV：Rust 原生，自动检测分隔符，按行解析（支持引号），首行为表头，输出 TSV。
pub fn parse_csv(path: &Path) -> Result<ParseResult, ParseError> {
    let bytes = std::fs::read(path).map_err(|e| ParseError::Io(e.to_string()))?;
    let content = decode_text(&bytes);
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("CSV")
        .to_string();

    let delim = detect_delimiter(&content);
    let mut rows: Vec<Vec<String>> = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        rows.push(parse_csv_line(line, delim));
    }

    let heading = if !rows.is_empty() {
        rows[0].join(" | ")
    } else {
        String::new()
    };
    let body = rows
        .iter()
        .map(|r| r.join("\t"))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(ParseResult {
        source_path: path.to_string_lossy().to_string(),
        doc_title: title,
        sections: vec![Section {
            heading,
            level: 0,
            body,
            page: None,
        }],
    })
}

/// HTML：Rust 原生，去标签提取正文，按 h1-h6 分层。
pub fn parse_html(path: &Path) -> Result<ParseResult, ParseError> {
    let bytes = std::fs::read(path).map_err(|e| ParseError::Io(e.to_string()))?;
    let content = decode_text(&bytes);
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("HTML")
        .to_string();

    let cleaned = clean_html(&content);
    let mut sections: Vec<Section> = Vec::new();
    let mut cur_heading = String::new();
    let mut cur_level: u32 = 0;
    let mut cur_body = String::new();
    let mut started = false;

    for line in cleaned.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            if started {
                sections.push(Section {
                    heading: cur_heading.clone(),
                    level: cur_level,
                    body: cur_body.trim().to_string(),
                    page: None,
                });
                cur_body.clear();
            }
            let hashes = trimmed.chars().take_while(|&c| c == '#').count();
            cur_level = hashes as u32;
            cur_heading = trimmed[hashes..].trim().to_string();
            started = true;
        } else {
            cur_body.push_str(trimmed);
            cur_body.push('\n');
            started = true;
        }
    }
    if started {
        sections.push(Section {
            heading: cur_heading,
            level: cur_level,
            body: cur_body.trim().to_string(),
            page: None,
        });
    }
    if sections.is_empty() {
        sections.push(Section {
            heading: title.clone(),
            level: 0,
            body: cleaned,
            page: None,
        });
    }
    Ok(ParseResult {
        source_path: path.to_string_lossy().to_string(),
        doc_title: title,
        sections,
    })
}

/// SVG：Rust 原生，提取 <text>/<tspan>/<title>/<desc> 中的文字。
pub fn parse_svg(path: &Path) -> Result<ParseResult, ParseError> {
    let bytes = std::fs::read(path).map_err(|e| ParseError::Io(e.to_string()))?;
    let content = decode_text(&bytes);
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("SVG")
        .to_string();

    let texts = extract_svg_texts(&content);
    let body = texts
        .iter()
        .filter(|t| !t.trim().is_empty())
        .map(|t| t.trim().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(ParseResult {
        source_path: path.to_string_lossy().to_string(),
        doc_title: title,
        sections: vec![Section {
            heading: String::new(),
            level: 0,
            body,
            page: None,
        }],
    })
}

/// 从 SVG XML 中提取文本内容。
fn extract_svg_texts(content: &str) -> Vec<String> {
    let mut results = Vec::new();
    for tag in &["text", "tspan", "title", "desc"] {
        let open = format!("<{}", tag);
        let close = format!("</{}>", tag);
        let lower = content.to_lowercase();
        let mut pos = 0;
        while let Some(start) = lower[pos..].find(&open) {
            let abs_start = pos + start;
            if let Some(end) = lower[abs_start..].find(&close) {
                let raw = &content[abs_start + open.len()..abs_start + end];
                if let Some(gt) = raw.find('>') {
                    let text = &raw[gt + 1..];
                    if !text.trim().is_empty() {
                        results.push(decode_html_entities(text.trim()));
                    }
                }
                pos = abs_start + end + close.len();
            } else {
                break;
            }
        }
    }
    results
}

/// 解码：优先 UTF-8，回退 GBK（中文常见编码）。
fn decode_text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let (cow, _, _) = encoding_rs::GBK.decode(bytes);
            cow.into_owned()
        }
    }
}

/// 按空行分段。
fn split_by_paragraphs(text: &str) -> Vec<Section> {
    let paras: Vec<&str> = text
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    if paras.is_empty() {
        return vec![Section {
            heading: String::new(),
            level: 0,
            body: text.to_string(),
            page: None,
        }];
    }
    paras
        .iter()
        .map(|p| Section {
            heading: String::new(),
            level: 0,
            body: p.to_string(),
            page: None,
        })
        .collect()
}

/// 解析单行 CSV（支持引号包裹和转义，可指定分隔符）。
fn parse_csv_line(line: &str, delim: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '"' {
            if in_quotes {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                in_quotes = true;
            }
        } else if c == delim && !in_quotes {
            fields.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(c);
        }
    }
    fields.push(current.trim().to_string());
    fields
}

/// 自动检测 CSV 分隔符：统计前 5 行的逗号/Tab/分号频率。
fn detect_delimiter(content: &str) -> char {
    let sample: String = content.lines().take(5).collect();
    let commas = sample.matches(',').count();
    let tabs = sample.matches('\t').count();
    let semicolons = sample.matches(';').count();
    if tabs >= commas && tabs >= semicolons && tabs > 0 {
        '\t'
    } else if semicolons > commas && semicolons > 0 {
        ';'
    } else {
        ','
    }
}

/// 清理 HTML：移除 script/style，转换标题为 Markdown 格式，去标签，解码实体。
fn clean_html(html: &str) -> String {
    let mut result = html.to_string();

    let remove_block = |result: &str, tag_start: &str, tag_end: &str| -> String {
        let lower = result.to_lowercase();
        let mut out = String::new();
        let mut pos = 0;
        while let Some(start) = lower[pos..].find(tag_start) {
            let abs_start = pos + start;
            out.push_str(&result[pos..abs_start]);
            if let Some(end) = lower[abs_start..].find(tag_end) {
                pos = abs_start + end + tag_end.len();
            } else {
                break;
            }
        }
        out.push_str(&result[pos..]);
        out
    };

    result = remove_block(&result, "<script", "</script>");
    result = remove_block(&result, "<style", "</style>");

    for i in 1..=6u32 {
        let prefix = "#".repeat(i as usize);
        for open in &[format!("<h{}>", i), format!("<H{}>", i)] {
            result = result.replace(open, &format!("\n{} ", prefix));
        }
        for close in &[format!("</h{}>", i), format!("</H{}>", i)] {
            result = result.replace(close, "\n");
        }
    }

    for tag in &[
        "<br>", "<br/>", "<br />", "<BR>", "<BR/>",
        "<p>", "</p>", "<P>", "</P>",
        "<div>", "</div>", "<DIV>", "</DIV>",
        "<li>", "</li>", "<LI>", "</LI>",
        "<tr>", "</tr>", "<TR>", "</TR>",
    ] {
        result = result.replace(tag, "\n");
    }

    for tag in &["</td>", "</th>", "</TD>", "</TH>"] {
        result = result.replace(tag, "\t");
    }
    for tag in &["<td>", "<th>", "<TD>", "<TH>"] {
        result = result.replace(tag, "");
    }

    let mut stripped = String::new();
    let mut in_tag = false;
    for c in result.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            stripped.push(c);
        }
    }

    stripped = decode_html_entities(&stripped);

    while stripped.contains("\n\n\n") {
        stripped = stripped.replace("\n\n\n", "\n\n");
    }
    stripped
}

/// 解码基本 HTML 实体。
fn decode_html_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&hellip;", "...")
        .replace("&mdash;", "\u{2014}")
        .replace("&ndash;", "\u{2013}")
}

/// Python sidecar 客户端：常驻进程，JSON-RPC over stdio。
pub struct SidecarClient {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    child: Child,
    next_id: u64,
}

impl SidecarClient {
    pub fn start() -> Result<Self, ParseError> {
        let script = resolve_script_path();
        let is_exe = script.ends_with(".exe");
        let mut child = if is_exe {
            Command::new(&script)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| {
                    ParseError::Sidecar(format!("启动 sidecar({}) 失败: {}", script, e))
                })?
        } else {
            let python =
                std::env::var("WORKKB_PYTHON").unwrap_or_else(|_| "python".to_string());
            Command::new(&python)
                .arg(&script)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| {
                    ParseError::Sidecar(format!("启动 python({}) 失败: {}", python, e))
                })?
        };
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ParseError::Sidecar("无法获取 stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ParseError::Sidecar("无法获取 stdout".into()))?;
        Ok(Self {
            stdin,
            stdout: BufReader::new(stdout),
            child,
            next_id: 1,
        })
    }

    pub fn parse(&mut self, path: &Path, kind: &str) -> Result<ParseResult, ParseError> {
        let id = self.next_id;
        self.next_id += 1;
        let req = json!({
            "id": id,
            "method": "parse",
            "params": { "path": path.to_string_lossy(), "kind": kind }
        });
        let line =
            serde_json::to_string(&req).map_err(|e| ParseError::Sidecar(e.to_string()))?;
        writeln!(self.stdin, "{}", line).map_err(|e| ParseError::Io(e.to_string()))?;
        self.stdin.flush().map_err(|e| ParseError::Io(e.to_string()))?;

        let mut resp_line = String::new();
        let n = self
            .stdout
            .read_line(&mut resp_line)
            .map_err(|e| ParseError::Io(e.to_string()))?;
        if n == 0 {
            return Err(ParseError::Sidecar("sidecar 进程已退出".into()));
        }
        let resp: Value = serde_json::from_str(resp_line.trim()).map_err(|e| {
            ParseError::Sidecar(format!("解析响应失败: {} | raw: {}", e, resp_line.trim()))
        })?;
        match resp.get("ok").and_then(|v| v.as_bool()) {
            Some(true) => {
                let result = resp.get("result").cloned().unwrap_or(Value::Null);
                let pr: ParseResult = serde_json::from_value(result)
                    .map_err(|e| ParseError::Sidecar(format!("反序列化 ParseResult 失败: {}", e)))?;
                Ok(pr)
            }
            _ => {
                let err = resp
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("未知错误");
                Err(ParseError::Sidecar(err.to_string()))
            }
        }
    }
}

impl Drop for SidecarClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// 解析 sidecar 脚本路径：优先环境变量；其次 exe 同目录 sidecar/ 下的 .exe（打包）或 .py（开发）。
fn resolve_script_path() -> String {
    if let Ok(p) = std::env::var("WORKKB_SIDECAR_PATH") {
        return p;
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        let sidecar_dir = exe.with_file_name("sidecar");
        candidates.push(sidecar_dir.join("parse_server.exe"));
        candidates.push(sidecar_dir.join("parse_server.py"));
        if let Some(debug_dir) = exe.parent() {
            // target/debug -> target -> src-tauri -> sidecar
            if let Some(target) = debug_dir.parent() {
                if let Some(src_tauri) = target.parent() {
                    candidates.push(src_tauri.join("sidecar").join("parse_server.py"));
                }
            }
        }
    }
    candidates.push(PathBuf::from("sidecar/parse_server.exe"));
    candidates.push(PathBuf::from("sidecar/parse_server.py"));
    for c in candidates {
        if c.exists() {
            return c.to_string_lossy().to_string();
        }
    }
    "sidecar/parse_server.py".to_string()
}

/// 支持的文件扩展名列表（单一数据源，db.rs 和 api.ts 均引用此定义）。
pub const SUPPORTED_EXTS: &[&str] = &[
    "pdf", "docx", "xlsx", "pptx", "doc", "xls", "ppt",
    "txt", "csv", "md", "html", "htm", "rtf", "wps", "et", "dps",
    "jpg", "jpeg", "png", "bmp", "gif", "tif", "tiff", "webp", "svg",
];

/// 按扩展名路由：PDF/TXT/MD/CSV/HTML→Rust 原生；Office 系→Python sidecar（懒启动）。
pub fn dispatch_parse(
    path: &Path,
    sidecar: &mut Option<SidecarClient>,
) -> Result<ParseResult, ParseError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "pdf" => parse_pdf(path),
        "txt" => parse_txt(path),
        "md" | "markdown" => parse_md(path),
        "csv" => parse_csv(path),
        "html" | "htm" => parse_html(path),
        "svg" => parse_svg(path),
        "docx" | "xlsx" | "pptx" | "doc" | "xls" | "ppt" | "rtf" | "wps" | "et" | "dps"
        | "jpg" | "jpeg" | "png" | "bmp" | "gif" | "tif" | "tiff" | "webp" => {
            if sidecar.is_none() {
                *sidecar = Some(SidecarClient::start()?);
            }
            let sc = sidecar.as_mut().expect("sidecar 已启动");
            sc.parse(path, &ext)
        }
        other => Err(ParseError::Unsupported(other.to_string())),
    }
}

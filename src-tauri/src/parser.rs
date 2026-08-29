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

/// 按扩展名路由：PDF→Rust 原生；docx/xlsx/pptx→Python sidecar（懒启动）。
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
        "docx" | "xlsx" | "pptx" => {
            if sidecar.is_none() {
                *sidecar = Some(SidecarClient::start()?);
            }
            let sc = sidecar.as_mut().expect("sidecar 已启动");
            sc.parse(path, &ext)
        }
        other => Err(ParseError::Unsupported(other.to_string())),
    }
}

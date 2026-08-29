"""个人工作知识库 - Office 解析 sidecar。

协议：JSON-RPC over stdio（每行一个请求/响应）。
请求: {"id":1,"method":"parse","params":{"path":"...","kind":"docx|xlsx|pptx"}}
响应: {"id":1,"ok":true,"result":{ParseResult}} 或 {"id":1,"ok":false,"error":"..."}

依赖：python-docx / openpyxl / python-pptx（见 requirements.txt）。
库在各自函数内懒导入，缺失某库不影响其它类型。
"""

import json
import os
import sys


def parse_docx(path):
    from docx import Document

    doc = Document(path)
    title = (doc.core_properties.title or
             os.path.splitext(os.path.basename(path))[0] or "Word")
    sections = []
    cur = {"heading": "", "level": 0, "body": "", "page": None}
    started = False
    for p in doc.paragraphs:
        style = (p.style.name or "") if p.style else ""
        text = p.text or ""
        if style.startswith("Heading") or style == "Title":
            if started:
                sections.append(cur)
            if style == "Title":
                lvl = 1
            else:
                try:
                    lvl = int(style.replace("Heading", "").strip())
                except ValueError:
                    lvl = 1
            cur = {"heading": text, "level": lvl, "body": "", "page": None}
            started = True
        else:
            if text.strip():
                cur["body"] += text + "\n"
                started = True
    if started:
        sections.append(cur)
    if not sections:
        sections = [{"heading": title, "level": 0, "body": "", "page": None}]
    return {"sourcePath": path, "docTitle": title, "sections": sections}


def parse_xlsx(path):
    from openpyxl import load_workbook

    wb = load_workbook(path, data_only=True, read_only=True)
    title = os.path.splitext(os.path.basename(path))[0] or "Excel"
    sections = []
    for ws in wb.worksheets:
        lines = []
        for row in ws.iter_rows(values_only=True):
            cells = ["" if c is None else str(c) for c in row]
            if any(c.strip() for c in cells):
                lines.append("\t".join(cells))
        sections.append({
            "heading": ws.title or "Sheet",
            "level": 0,
            "body": "\n".join(lines),
            "page": None,
        })
    return {"sourcePath": path, "docTitle": title, "sections": sections}


def parse_pptx(path):
    from pptx import Presentation

    prs = Presentation(path)
    title = (prs.core_properties.title or
             os.path.splitext(os.path.basename(path))[0] or "PPT")
    sections = []
    for i, slide in enumerate(prs.slides, 1):
        title_shape = slide.shapes.title
        heading = ""
        if title_shape is not None and title_shape.has_text_frame:
            heading = title_shape.text_frame.text or ""
        body_parts = []
        for shape in slide.shapes:
            if shape is title_shape:
                continue
            if shape.has_text_frame:
                txt = shape.text_frame.text
                if txt.strip():
                    body_parts.append(txt)
        if not heading:
            heading = f"幻灯片 {i}"
        sections.append({
            "heading": heading,
            "level": 1,
            "body": "\n".join(body_parts),
            "page": i,
        })
    return {"sourcePath": path, "docTitle": title, "sections": sections}


HANDLERS = {"docx": parse_docx, "xlsx": parse_xlsx, "pptx": parse_pptx}


def handle(req):
    rid = req.get("id")
    method = req.get("method")
    if method != "parse":
        return {"id": rid, "ok": False, "error": f"unknown method: {method}"}
    params = req.get("params", {}) or {}
    path = params.get("path", "")
    kind = params.get("kind", "")
    handler = HANDLERS.get(kind)
    if handler is None:
        return {"id": rid, "ok": False, "error": f"unsupported kind: {kind}"}
    if not os.path.exists(path):
        return {"id": rid, "ok": False, "error": f"file not found: {path}"}
    try:
        result = handler(path)
        return {"id": rid, "ok": True, "result": result}
    except Exception as e:  # noqa: BLE001 - 边界：第三方解析库异常多样
        return {"id": rid, "ok": False, "error": f"{type(e).__name__}: {e}"}


def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except Exception as e:  # noqa: BLE001
            sys.stdout.write(
                json.dumps({"id": None, "ok": False, "error": f"bad json: {e}"},
                           ensure_ascii=False) + "\n")
            sys.stdout.flush()
            continue
        resp = handle(req)
        sys.stdout.write(json.dumps(resp, ensure_ascii=False) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()

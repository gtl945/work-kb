@echo off
chcp 65001 >nul
setlocal

echo ============================================
echo   work-kb 一键打包脚本
echo ============================================
echo.

REM ---- 环境检查 ----
python --version >nul 2>&1
if errorlevel 1 (
    echo [错误] 未找到 Python，请先安装 Python 3.10+
    echo        下载地址: https://www.python.org/downloads/
    pause
    exit /b 1
)

node --version >nul 2>&1
if errorlevel 1 (
    echo [错误] 未找到 Node.js，请先安装 Node.js 18+
    echo        下载地址: https://nodejs.org/
    pause
    exit /b 1
)

cargo --version >nul 2>&1
if errorlevel 1 (
    echo [错误] 未找到 Rust/Cargo，请先安装 Rust 工具链
    echo        下载地址: https://rustup.rs/
    echo        另需安装 Visual Studio Build Tools (C++ 工作负载)
    pause
    exit /b 1
)

echo [环境] Python:    ok
echo [环境] Node.js:   ok
echo [环境] Rust:      ok
echo.

REM ---- Step 1: 安装 Python 依赖 ----
echo [1/5] 安装 Python 依赖...
pip install -r src-tauri\sidecar\requirements.txt pyinstaller -q
if errorlevel 1 (
    echo [错误] Python 依赖安装失败
    pause
    exit /b 1
)
echo       完成
echo.

REM ---- Step 2: PyInstaller 打包 sidecar ----
echo [2/5] 用 PyInstaller 打包 sidecar...
pushd src-tauri\sidecar
pyinstaller --onefile --name parse_server --noconfirm parse_server.py
if errorlevel 1 (
    echo [错误] PyInstaller 打包失败
    popd
    pause
    exit /b 1
)
copy dist\parse_server.exe . >nul
rmdir /s /q dist 2>nul
rmdir /s /q build 2>nul
del parse_server.spec 2>nul
popd
echo       完成 - sidecar\parse_server.exe
echo.

REM ---- Step 3: 安装前端依赖 ----
echo [3/5] 安装前端依赖...
call npm install --silent
echo       完成
echo.

REM ---- Step 4: 构建 Tauri 应用 ----
echo [4/5] 构建 Tauri 应用 (release 模式，请耐心等待)...
call npm run tauri build
if errorlevel 1 (
    echo [错误] Tauri 构建失败
    pause
    exit /b 1
)
echo       完成
echo.

REM ---- Step 5: 复制 sidecar 到发布目录 ----
echo [5/5] 复制 sidecar 到发布目录...
if not exist src-tauri\target\release\sidecar mkdir src-tauri\target\release\sidecar
copy src-tauri\sidecar\parse_server.exe src-tauri\target\release\sidecar\parse_server.exe >nul
echo       完成
echo.

REM ---- 结果 ----
echo ============================================
echo   打包完成！
echo ============================================
echo.
echo 产物位置:
echo   主程序:   src-tauri\target\release\work-kb.exe
echo   NSIS安装包: src-tauri\target\release\bundle\nsis\work-kb_0.1.0_x64-setup.exe
echo   MSI安装包:  src-tauri\target\release\bundle\msi\work-kb_0.1.0_x64_en-US.msi
echo.
echo 分发说明:
echo   方式1 (推荐): 直接发安装包 (.exe setup)，安装后自带主程序
echo   方式2 (绿色版): 将 release\ 目录整个打包为 zip
echo                    需包含 work-kb.exe + sidecar\parse_server.exe
echo.
echo 终端用户要求:
echo   Windows 10/11 (需预装 WebView2，Win10/11 通常已自带)
echo   无需安装 Python / Rust / Node.js
echo ============================================
pause

//! 屏幕识别模块 —— Windows 自带 OCR + PowerShell 截图。
//!
//! 零依赖方案：不装 Overwolf / tesseract，只调用 Windows 内置能力：
//! - 截图：PowerShell 调 System.Drawing 抓取游戏窗口区域
//! - 识别：PowerShell 调 Windows.Media.Ocr（系统自带，支持中文 zh-Hans-CN）
//!
//! 用途：对局中读取商店 5 个棋子的名字，供前端做实时推荐。

use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

const OCR_SCRIPT: &str = include_str!("../resources/ocr.ps1");

#[derive(Debug, Error)]
pub enum OcrError {
    #[error("截图失败: {0}")]
    Capture(String),
    #[error("OCR 失败: {0}")]
    Ocr(String),
    #[error("找不到游戏窗口")]
    WindowNotFound,
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

/// 定位英雄联盟游戏窗口并截图指定区域。
/// `title_keyword` 用于匹配窗口标题（默认 "League of Legends"）。
/// `region` 传 `(x, y, w, h)` 相对窗口客户区；None 时截全客户区。
pub fn capture_game_window(
    save_path: &Path,
    title_keyword: &str,
    region: Option<(i32, i32, i32, i32)>,
) -> Result<(), OcrError> {
    let ps = r##"
param($OutPath, $TitleKeyword, $RegionX, $RegionY, $RegionW, $RegionH)
Add-Type -AssemblyName System.Drawing | Out-Null
Add-Type -AssemblyName System.Windows.Forms | Out-Null
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32 {
    [DllImport("user32.dll")] public static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, out POINT p);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
    public struct RECT { public int L, T, R, B; }
    public struct POINT { public int X, Y; }
}
"@
$hwnd = [Win32]::FindWindow($null, $TitleKeyword)
if ($hwnd -eq [IntPtr]::Zero) { exit 1 }
# 如果最小化则先恢复
if ([Win32]::IsIconic($hwnd)) { [Win32]::ShowWindow($hwnd, 9) | Out-Null; Start-Sleep -Milliseconds 300 }
[Win32]::SetForegroundWindow($hwnd) | Out-Null
Start-Sleep -Milliseconds 150
$rect = New-Object Win32+RECT
[Win32]::GetClientRect($hwnd, [ref]$rect) | Out-Null
$pt = New-Object Win32+POINT
[Win32]::ClientToScreen($hwnd, [ref]$pt) | Out-Null
# 客户区在屏幕上的绝对坐标
$absX = $pt.X
$absY = $pt.Y
$cw = $rect.R - $rect.L
$ch = $rect.B - $rect.T
if ($cw -le 0 -or $ch -le 0) { exit 2 }
# 区域：未指定则全客户区
$x = if ($RegionX -ge 0) { $absX + $RegionX } else { $absX }
$y = if ($RegionY -ge 0) { $absY + $RegionY } else { $absY }
$w = if ($RegionW -gt 0) { $RegionW } else { $cw }
$h = if ($RegionH -gt 0) { $RegionH } else { $ch }
# 截图
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($x, $y, 0, 0, (New-Object System.Drawing.Size($w, $h)))
$bmp.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
exit 0
"##;
    let ps = ps.trim();
    let out = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"])
        .arg(ps)
        .arg(save_path.to_str().unwrap_or(""))
        .arg(title_keyword)
        .arg(region.map(|r| r.0.to_string()).unwrap_or_else(|| "-1".into()))
        .arg(region.map(|r| r.1.to_string()).unwrap_or_else(|| "-1".into()))
        .arg(region.map(|r| r.2.to_string()).unwrap_or_else(|| "0".into()))
        .arg(region.map(|r| r.3.to_string()).unwrap_or_else(|| "0".into()))
        .output()
        .map_err(|e| OcrError::Capture(e.to_string()))?;

    if !out.status.success() {
        let code = out.status.code().unwrap_or(-1);
        return Err(match code {
            1 => OcrError::WindowNotFound,
            _ => OcrError::Capture(format!("exit code {code}")),
        });
    }
    Ok(())
}

/// 对指定图片执行 OCR，返回识别出的整段文本。
pub fn ocr_image(image_path: &Path) -> Result<String, OcrError> {
    let script_path = write_ocr_script()?;
    let out_file = image_path.with_extension("ocr.txt");

    let out = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script_path)
        .arg("-ImagePath")
        .arg(image_path)
        .arg("-OutFile")
        .arg(&out_file)
        .output()
        .map_err(|e| OcrError::Ocr(e.to_string()))?;

    if !out.status.success() {
        return Err(OcrError::Ocr(format!("exit code {}", out.status.code().unwrap_or(-1))));
    }
    if !out_file.exists() {
        return Err(OcrError::Ocr("OCR 未生成结果文件".into()));
    }
    let text = std::fs::read_to_string(&out_file)?;
    Ok(text.trim().to_string())
}

/// 把内嵌的 ocr.ps1 写到临时目录（PowerShell 需要真实文件路径）。
fn write_ocr_script() -> Result<PathBuf, OcrError> {
    let dir = std::env::temp_dir().join("tft-sidekick-ocr");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("ocr.ps1");
    std::fs::write(&path, OCR_SCRIPT)?;
    Ok(path)
}

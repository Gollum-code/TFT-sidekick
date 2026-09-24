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
/// 窗口查找：按进程名（LeagueClientUx/LeagueClient/LeagueClientUxRender/League of Legends）
/// 枚举顶层窗口，取 client 区域面积最大的一个（TFT 对局画面在客户端内渲染）。
/// `region` 传 `(x, y, w, h)`：0..100 = 窗口宽高百分比，>100 = 像素，-1 = 未指定。
/// `scale` 为图像放大倍数（放大后再送 OCR，提高小字识别率）。
pub fn capture_game_window(
    save_path: &Path,
    region: Option<(i32, i32, i32, i32)>,
    scale: f64,
) -> Result<(), OcrError> {
    let ps = r##"
param($OutPath, $RegionX, $RegionY, $RegionW, $RegionH, $Scale)
Add-Type -AssemblyName System.Drawing | Out-Null
Add-Type -AssemblyName System.Windows.Forms | Out-Null
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class Win32 {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder sb, int maxCount);
    [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, out POINT p);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    public struct RECT { public int L, T, R, B; }
    public struct POINT { public int X, Y; }
}
"@
# 按进程名找英雄联盟客户端/游戏窗口（覆盖国服/国际服）。
# TFT 对局画面在 LeagueClientUx 内渲染（无独立游戏窗口），且窗口可见性
# 探测不可靠；直接取所有相关进程中 client 区域面积最大的窗口。
function Find-LolWindow {
  $procIds = @()
  foreach ($name in @("LeagueClientUx", "LeagueClient", "LeagueClientUxRender", "League of Legends")) {
    Get-Process -Name $name -ErrorAction SilentlyContinue | ForEach-Object { $procIds += $_.Id }
  }
  if ($procIds.Count -eq 0) { return [IntPtr]::Zero }
  $script:procIds = $procIds
  $script:best = [IntPtr]::Zero
  $script:bestArea = -1
  $callback = [Win32+EnumWindowsProc]{
    param($hWnd, $lParam)
    $winPid = 0
    [Win32]::GetWindowThreadProcessId($hWnd, [ref]$winPid) | Out-Null
    if ($script:procIds -contains $winPid) {
      $cr = New-Object Win32+RECT
      [Win32]::GetClientRect($hWnd, [ref]$cr) | Out-Null
      $w = $cr.R - $cr.L; $h = $cr.B - $cr.T
      if ($w -gt 0 -and $h -gt 0) {
        $area = $w * $h
        if ($area -gt $script:bestArea) { $script:best = $hWnd; $script:bestArea = $area }
      }
    }
    return $true
  }
  [Win32]::EnumWindows($callback, [IntPtr]::Zero) | Out-Null
  return $script:best
}
$hwnd = Find-LolWindow
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
# 区域：参数 0..100 = 窗口宽高百分比；-1 = 未指定（全客户区）
function Px($val, $span, $default) {
  if ($val -ge 0 -and $val -le 100) { return [int]($span * $val / 100.0) }
  if ($val -gt 100) { return [int]$val }
  return $default
}
$x = $absX + (Px $RegionX $cw 0)
$y = $absY + (Px $RegionY $ch 0)
$w = Px $RegionW $cw $cw
$h = Px $RegionH $ch $ch
# 截图
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($x, $y, 0, 0, (New-Object System.Drawing.Size($w, $h)))
# 放大（默认 2 倍，提高 OCR 小字识别率）
$scale = if ($Scale -gt 0) { $Scale } else { 2.0 }
if ($scale -gt 1.0) {
  $sw = [int][Math]::Round($w * $scale)
  $sh = [int][Math]::Round($h * $scale)
  $big = New-Object System.Drawing.Bitmap($sw, $sh)
  $bg = [System.Drawing.Graphics]::FromImage($big)
  $bg.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $bg.DrawImage($bmp, 0, 0, $sw, $sh)
  $big.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
  $bg.Dispose(); $big.Dispose()
} else {
  $bmp.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
}
$g.Dispose(); $bmp.Dispose()
exit 0
"##;
    let ps = ps.trim();
    let out = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"])
        .arg(ps)
        .arg(save_path.to_str().unwrap_or(""))
        .arg(region.map(|r| r.0.to_string()).unwrap_or_else(|| "-1".into()))
        .arg(region.map(|r| r.1.to_string()).unwrap_or_else(|| "-1".into()))
        .arg(region.map(|r| r.2.to_string()).unwrap_or_else(|| "0".into()))
        .arg(region.map(|r| r.3.to_string()).unwrap_or_else(|| "0".into()))
        .arg(scale.to_string())
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

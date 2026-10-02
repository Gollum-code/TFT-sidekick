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
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
    public struct RECT { public int L, T, R, B; }
    public struct POINT { public int X, Y; }
}
"@
const uint PW_RENDERFULLCONTENT = 0x00000002; # Win 8.1+：可捕获 DirectX 内容
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
# 如果最小化则先恢复（PrintWindow 对最小化窗口抓不到渲染内容）
if ([Win32]::IsIconic($hwnd)) { [Win32]::ShowWindow($hwnd, 9) | Out-Null; Start-Sleep -Milliseconds 300 }
# 注意：不要 SetForegroundWindow —— 对局中扫描会抢走游戏焦点，且 PrintWindow(PW_RENDERFULLCONTENT)
# 即使窗口被遮挡也能捕获内容，不需要置前。
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
# 截图：用 PrintWindow 捕获窗口内容（能抓 DirectX/GPU 渲染画面），
# 再按区域裁剪并 2 倍放大，最后保存。
$bmp = New-Object System.Drawing.Bitmap($cw, $ch)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
$ok = [Win32]::PrintWindow($hwnd, $hdc, $PW_RENDERFULLCONTENT)
$g.ReleaseHdc($hdc)
if (-not $ok) {
  # 降级：GDI 屏幕复制（少数窗口无法 PrintWindow 时兜底）
  $g.CopyFromScreen($absX, $absY, 0, 0, (New-Object System.Drawing.Size($cw, $ch)))
}
# 裁剪区域（0..100 为百分比，>100 为像素）
function Px($val, $span, $default) {
  if ($val -ge 0 -and $val -le 100) { return [int]($span * $val / 100.0) }
  if ($val -gt 100) { return [int]$val }
  return $default
}
$rx = Px $RegionX $cw 0
$ry = Px $RegionY $ch 0
$rw = Px $RegionW $cw $cw
$rh = Px $RegionH $ch $ch
if ($rw -le 0 -or $rh -le 0) { $g.Dispose(); $bmp.Dispose(); exit 3 }
# 放大（默认 2 倍，提高 OCR 小字识别率）
$scale = if ($Scale -gt 0) { $Scale } else { 2.0 }
$sw = [int][Math]::Round($rw * $scale)
$sh = [int][Math]::Round($rh * $scale)
$big = New-Object System.Drawing.Bitmap($sw, $sh)
$bg = [System.Drawing.Graphics]::FromImage($big)
$bg.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
# 从窗口位图裁剪区域再缩放
$srcRect = New-Object System.Drawing.Rectangle($rx, $ry, $rw, $rh)
$dstRect = New-Object System.Drawing.Rectangle(0, 0, $sw, $sh)
$bg.DrawImage($bmp, $dstRect, $srcRect, [System.Drawing.GraphicsUnit]::Pixel)
$big.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
$bg.Dispose(); $big.Dispose(); $g.Dispose(); $bmp.Dispose()
exit 0
"##;
    let ps = ps.trim();

    // param() 只在 -File 模式生效；写入临时 .ps1 后用命名参数调用
    let dir = std::env::temp_dir().join("tft-sidekick-ocr");
    std::fs::create_dir_all(&dir)?;
    let script_path = dir.join("capture.ps1");
    std::fs::write(&script_path, ps)?;

    let out = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script_path)
        .arg("-OutPath")
        .arg(save_path.to_str().unwrap_or(""))
        .arg("-RegionX")
        .arg(region.map(|r| r.0.to_string()).unwrap_or_else(|| "-1".into()))
        .arg("-RegionY")
        .arg(region.map(|r| r.1.to_string()).unwrap_or_else(|| "-1".into()))
        .arg("-RegionW")
        .arg(region.map(|r| r.2.to_string()).unwrap_or_else(|| "0".into()))
        .arg("-RegionH")
        .arg(region.map(|r| r.3.to_string()).unwrap_or_else(|| "0".into()))
        .arg("-Scale")
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

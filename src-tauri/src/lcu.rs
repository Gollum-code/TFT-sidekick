//! LCU (League Client Update) 采集模块 —— 从 Node 库
//! `LOL-CN-Server-LCU-Integration-Solution` 移植到 Rust。
//!
//! 国服（腾讯）特性：
//! - lockfile 为 0 字节（被清空）。
//! - 进程命令行被 ACE 反作弊屏蔽（WMI 读不到），仅作兜底。
//! - 真正的 remoting 凭据在 `LeagueClientUx.log` 的 `Cef CommandLine` 行：
//!   `--remoting-auth-token=XXX --app-port=YYY`（`LeagueClient.log` 里只有
//!   `--riotclient-auth-token`，那是启动器凭据，不是 LCU 凭据）。
//! - 凭据发现链：手动 → 日志 → lockfile → 进程命令行。

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use thiserror::Error;

use crate::models::{MatchDetail, MatchSummary, MatchTrait, MatchUnit};

const WINDOWS_DRIVES: [&str; 6] = ["C:", "D:", "E:", "F:", "G:", "H:"];

#[derive(Debug, Error)]
pub enum LcuError {
    #[error("未找到 LCU 凭据（请先登录并运行英雄联盟客户端）")]
    CredentialsNotFound,
    #[error("尚未连接 LCU")]
    NotConnected,
    #[error("请求失败: {0}")]
    Request(String),
    #[error("响应状态码 {0}")]
    Status(u16),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    Log,
    Lockfile,
    Process,
    Manual,
}

impl CredentialSource {
    pub fn as_str(self) -> &'static str {
        match self {
            CredentialSource::Log => "log",
            CredentialSource::Lockfile => "lockfile",
            CredentialSource::Process => "process",
            CredentialSource::Manual => "manual",
        }
    }
}

/// LCU 连接凭据。每次客户端重启都会变。
#[derive(Debug, Clone)]
pub struct LcuCredentials {
    pub port: u16,
    pub token: String,
    pub source: CredentialSource,
}

#[derive(Debug, Clone, Default)]
pub struct RankedStats {
    pub tier: String,
    pub division: String,
    pub league_points: i32,
    pub wins: i32,
    pub losses: i32,
}



/// 解析 LCU 排位响应中的 TFT 队列。
fn parse_ranked(json: serde_json::Value) -> RankedStats {
    let queue = json
        .get("queueMap")
        .and_then(|q| q.get("RANKED_TFT"))
        .or_else(|| json.get("queueMap").and_then(|q| q.get("RANKED_TFT_TURBO")));
    let mut out = RankedStats::default();
    if let Some(q) = queue {
        out.tier = q.get("tier").and_then(|v| v.as_str()).unwrap_or("").to_string();
        out.division = q.get("division").and_then(|v| v.as_str()).unwrap_or("").to_string();
        out.league_points = q.get("leaguePoints").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        out.wins = q.get("wins").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        out.losses = q.get("losses").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    }
    out
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentSummoner {
    pub puuid: String,
    #[serde(default)]
    pub game_name: Option<String>,
    #[serde(default)]
    pub tag_line: Option<String>,
    #[serde(default)]
    pub summoner_level: Option<u32>,
}

pub struct LcuClient {
    client: reqwest::Client,
    credentials: Option<LcuCredentials>,
}

impl Default for LcuClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LcuClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .expect("failed to build reqwest client");
        Self {
            client,
            credentials: None,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.credentials.is_some()
    }

    pub fn source(&self) -> Option<&'static str> {
        self.credentials.as_ref().map(|c| c.source.as_str())
    }

    /// 完整发现链：手动 → 日志 → lockfile → 进程命令行。
    pub fn discover_and_connect(&mut self) -> Result<(), LcuError> {
        match discover(None) {
            Some(creds) => {
                self.credentials = Some(creds);
                Ok(())
            }
            None => Err(LcuError::CredentialsNotFound),
        }
    }

    async fn get_json(&self, path: &str) -> Result<serde_json::Value, LcuError> {
        let creds = self.credentials.as_ref().ok_or(LcuError::NotConnected)?;
        let url = format!("https://127.0.0.1:{}{}", creds.port, path);
        let resp = self
            .client
            .get(&url)
            .basic_auth("riot", Some(&creds.token))
            .send()
            .await
            .map_err(|e| LcuError::Request(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(LcuError::Status(status.as_u16()));
        }
        resp.json()
            .await
            .map_err(|e| LcuError::Request(e.to_string()))
    }

    /// 当前游戏流程阶段（Lobby/InProgress/PreEndOfGame/EndOfGame/...）。
    pub async fn gameflow_phase(&self) -> Result<String, LcuError> {
        let json = self.get_json("/lol-gameflow/v1/gameflow-phase").await?;
        Ok(json.as_str().unwrap_or("").to_string())
    }

    /// 当前 TFT 排位（段位/胜点/胜率）。
    pub async fn ranked_stats(&self) -> Result<RankedStats, LcuError> {
        let json = self.get_json("/lol-ranked/v1/current-ranked-stats").await?;
        Ok(parse_ranked(json))
    }

    pub async fn current_summoner(&self) -> Result<CurrentSummoner, LcuError> {
        let json = self
            .get_json("/lol-summoner/v1/current-summoner")
            .await?;
        serde_json::from_value(json).map_err(|e| LcuError::Request(e.to_string()))
    }

    /// 拉取个人 TFT 对局列表（默认最近 20 局），映射为完整对局（含阵容/羁绊）。
    pub async fn tft_matches(&self, puuid: &str) -> Result<Vec<MatchDetail>, LcuError> {
        let path = format!("/lol-match-history/v1/products/tft/{}/matches", puuid);
        let json = self.get_json(&path).await?;
        Ok(parse_matches(&json, puuid))
    }
}

// ---------------------------------------------------------------------------
// 凭据发现链（对应 lib/discovery.js，但针对国服日志格式修正）
// ---------------------------------------------------------------------------

fn program_data_files(filename: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(pd) = std::env::var("PROGRAMDATA") {
        candidates.push(PathBuf::from(pd).join("Riot Games").join(filename));
    }
    for d in WINDOWS_DRIVES {
        candidates.push(Path::new(d).join("ProgramData").join("Riot Games").join(filename));
    }
    candidates
}

fn read_json_safe(path: &Path) -> Option<serde_json::Value> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn collect_paths(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(a) => {
            for v in a {
                collect_paths(v, out);
            }
        }
        serde_json::Value::Object(o) => {
            for (_, v) in o {
                collect_paths(v, out);
            }
        }
        _ => {}
    }
}

fn find_install_dirs() -> Vec<PathBuf> {
    let mut dirs: HashSet<PathBuf> = HashSet::new();
    for p in program_data_files("RiotClientInstalls.json") {
        if let Some(json) = read_json_safe(&p) {
            let mut raw = Vec::new();
            collect_paths(&json, &mut raw);
            for s in raw {
                let matched = s.contains("LeagueClient")
                    || s.contains("英雄联盟")
                    || s.contains("WeGame")
                    || s.contains("wegame");
                if !matched {
                    continue;
                }
                let resolved = if s.ends_with(".exe") {
                    Path::new(&s)
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| PathBuf::from(&s))
                } else {
                    PathBuf::from(&s)
                };
                dirs.insert(resolved);
            }
        }
    }
    for d in WINDOWS_DRIVES {
        dirs.insert(Path::new(d).join("Riot Games").join("League of Legends"));
        dirs.insert(Path::new(d).join("Games").join("League of Legends"));
        dirs.insert(Path::new(d).join("WeGameApps").join("英雄联盟（含经典模式）"));
        dirs.insert(Path::new(d).join("WeGameApps").join("英雄联盟"));
        dirs.insert(Path::new(d).join("wegameapps").join("英雄联盟（含经典模式）"));
        dirs.insert(Path::new(d).join("wegameapps").join("英雄联盟"));
    }
    dirs.into_iter().filter(|d| d.exists()).collect()
}

fn extract_value(line: &str, needle: &str, valid: impl Fn(char) -> bool) -> Option<String> {
    let idx = line.find(needle)?;
    let rest = &line[idx + needle.len()..];
    let end = rest.find(|c| !valid(c)).unwrap_or(rest.len());
    let v = &rest[..end];
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

fn extract_credentials(line: &str) -> Option<LcuCredentials> {
    let token = extract_value(line, "--remoting-auth-token=", |c| {
        c.is_ascii_alphanumeric() || c == '_' || c == '-'
    })?;
    let port: u16 = extract_value(line, "--app-port=", |c| c.is_ascii_digit())?
        .parse()
        .ok()?;
    Some(LcuCredentials {
        port,
        token,
        source: CredentialSource::Log,
    })
}

fn read_lockfile(install_dirs: &[PathBuf]) -> Option<LcuCredentials> {
    let mut candidates = Vec::new();
    for d in install_dirs {
        candidates.push(d.join("lockfile"));
        candidates.push(d.join("lockfile_"));
    }
    for p in candidates {
        if let Ok(c) = std::fs::read_to_string(&p) {
            let c = c.trim();
            if c.starts_with("LeagueClient:") {
                let parts: Vec<&str> = c.split(':').collect();
                if parts.len() >= 4 {
                    if let Ok(port) = parts[1].parse() {
                        return Some(LcuCredentials {
                            port,
                            token: parts[2].to_string(),
                            source: CredentialSource::Lockfile,
                        });
                    }
                }
            }
        }
    }
    None
}

fn read_process_credentials() -> Option<LcuCredentials> {
    // 国服被 ACE 反作弊屏蔽，WMI 通常读不到，仅兜底。
    let script = "Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'LeagueClientUx.exe' -or $_.Name -eq 'LeagueClient.exe' } | Select-Object -ExpandProperty CommandLine";
    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if line.contains("--remoting-auth-token=") {
            if let Some(mut creds) = extract_credentials(line) {
                creds.source = CredentialSource::Process;
                return Some(creds);
            }
        }
    }
    None
}

fn find_log_candidates(install_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for d in install_dirs {
        // LeagueClientUx.log 在这里（含 remoting 凭据）
        dirs.push(d.join("LeagueClient"));
        // LeagueClient.log 在这里（只有 riotclient 凭据）
        dirs.push(d.join("Game").join("Logs").join("LeagueClient Logs"));
        dirs.push(d.join("Logs"));
        dirs.push(d.join("Game").join("Logs"));
    }
    let mut files = Vec::new();
    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|s| s.to_str()) == Some("log") {
                    files.push(p);
                }
            }
        }
    }
    files
}

fn file_stat(path: &Path) -> (u64, u64) {
    std::fs::metadata(path)
        .map(|m| {
            let mtime = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            (mtime, m.len())
        })
        .unwrap_or((0, 0))
}

/// 扫描单个日志文件，找含 `--remoting-auth-token=` 的行并提取凭据。
fn scan_file_for_credentials(path: &Path) -> Option<LcuCredentials> {
    let bytes = std::fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    for line in text.lines() {
        if line.contains("--remoting-auth-token=") {
            if let Some(creds) = extract_credentials(line) {
                return Some(creds);
            }
        }
    }
    None
}

fn read_log_credentials(install_dirs: &[PathBuf]) -> Option<LcuCredentials> {
    let mut files: Vec<(PathBuf, u64)> = find_log_candidates(install_dirs)
        .into_iter()
        .map(|f| {
            let (mtime, size) = file_stat(&f);
            (f, mtime * 10_000_000 + size)
        })
        .collect();
    // 最新优先（含大小作为次级排序，避免空文件排前）
    files.sort_by(|a, b| b.1.cmp(&a.1));
    for (f, _) in files {
        let (_, size) = file_stat(&f);
        if size == 0 {
            continue;
        }
        if let Some(mut creds) = scan_file_for_credentials(&f) {
            creds.source = CredentialSource::Log;
            return Some(creds);
        }
    }
    None
}

/// 凭据发现链：手动 → 日志 → lockfile → 进程命令行。
pub fn discover(manual: Option<(u16, String)>) -> Option<LcuCredentials> {
    if let Some((port, token)) = manual {
        return Some(LcuCredentials {
            port,
            token,
            source: CredentialSource::Manual,
        });
    }
    let install_dirs = find_install_dirs();
    read_log_credentials(&install_dirs)
        .or_else(|| read_lockfile(&install_dirs))
        .or_else(read_process_credentials)
}

// ---------------------------------------------------------------------------
// TFT 对局响应解析（字段名已对照真实响应）
//
// 顶层：{ "active_puuid": "", "games": [ { "json": {...}, "metadata": {...} } ] }
// 对局字段（games[i].json）：gameId/game_id、gameCreation(ms)、game_datetime(ms 字符串)、
//   game_length、game_version、queueId/queue_id、tft_set_core_name、tft_set_number、
//   participants[8]
// 玩家字段：puuid、riotIdGameName、riotIdTagline、placement、level、win、
//   units[]、traits[]
// 棋子：character_id、itemNames[]、name、rarity、tier
// 羁绊：name、num_units、style、tier_current、tier_total
// ---------------------------------------------------------------------------

fn pick<'a>(
    obj: &'a serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<&'a serde_json::Value> {
    keys.iter().find_map(|k| obj.get(*k))
}

fn epoch_ms_to_rfc3339(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| ms.to_string())
}

fn to_epoch_ms(v: &serde_json::Value) -> Option<i64> {
    match v {
        serde_json::Value::Number(n) => n.as_i64(),
        serde_json::Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    }
}

fn game_datetime(game: &serde_json::Value) -> String {
    let obj = game.as_object().unwrap();
    if let Some(v) = pick(obj, &["game_datetime", "gameCreation", "game_creation"]) {
        if let Some(ms) = to_epoch_ms(v) {
            return epoch_ms_to_rfc3339(ms);
        }
    }
    String::new()
}

fn game_id(game: &serde_json::Value) -> String {
    let obj = game.as_object().unwrap();
    match pick(obj, &["gameId", "game_id"]) {
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

pub(crate) fn map_units(participant: &serde_json::Value) -> Vec<MatchUnit> {
    participant
        .get("units")
        .and_then(|u| u.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|u| {
                    let o = u.as_object()?;
                    let character_id = o.get("character_id")?.as_str()?.to_string();
                    let items = o
                        .get("itemNames")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default();
                    Some(MatchUnit {
                        character_id,
                        name: o
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        rarity: o.get("rarity").and_then(|v| v.as_i64()).map(|r| r as i32),
                        tier: o.get("tier").and_then(|v| v.as_i64()).map(|t| t as i32),
                        items,
                        position_x: None,
                        position_y: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn map_traits(participant: &serde_json::Value) -> Vec<MatchTrait> {
    participant
        .get("traits")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let o = t.as_object()?;
                    let name = o.get("name")?.as_str()?.to_string();
                    Some(MatchTrait {
                        trait_id: name.clone(),
                        name: Some(name),
                        tier_current: o
                            .get("tier_current")
                            .and_then(|v| v.as_i64())
                            .map(|x| x as i32),
                        tier_total: o
                            .get("tier_total")
                            .and_then(|v| v.as_i64())
                            .map(|x| x as i32),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn map_game(game: &serde_json::Value, my_puuid: &str) -> Option<MatchDetail> {
    let obj = game.as_object()?;
    let game_id = game_id(game);
    if game_id.is_empty() {
        return None;
    }
    let game_mode = pick(obj, &["tftSetCoreName", "tft_set_core_name"])
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| {
            pick(obj, &["gameMode", "game_mode"])
                .and_then(|v| v.as_str())
                .map(String::from)
        });
    let game_version = pick(obj, &["gameVersion", "game_version"])
        .and_then(|v| v.as_str())
        .map(String::from);

    let participants = obj.get("participants")?.as_array()?;
    let me = participants
        .iter()
        .find(|p| p.get("puuid").and_then(|v| v.as_str()) == Some(my_puuid));

    let (placement, level, units, traits) = match me {
        Some(m) => (
            m.get("placement").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            m.get("level").and_then(|v| v.as_i64()).map(|l| l as i32),
            map_units(m),
            map_traits(m),
        ),
        None => (0, None, Vec::new(), Vec::new()),
    };

    Some(MatchDetail {
        summary: MatchSummary {
            game_id,
            game_mode,
            game_version,
            game_datetime: game_datetime(game),
            placement,
            level,
            // 国服 LCU 不提供海克斯强化数据。
            augments: Vec::new(),
        },
        units,
        traits,
    })
}

fn extract_games(root: &serde_json::Value) -> Vec<&serde_json::Value> {
    if let Some(arr) = root.get("games").and_then(|g| g.as_array()) {
        let out: Vec<&serde_json::Value> = arr
            .iter()
            .filter_map(|entry| entry.get("json").or(Some(entry)))
            .collect();
        if !out.is_empty() {
            return out;
        }
    }
    if let Some(inner) = root
        .get("games")
        .and_then(|g| g.get("games"))
        .and_then(|g| g.as_array())
    {
        return inner.iter().collect();
    }
    Vec::new()
}

fn parse_matches(root: &serde_json::Value, my_puuid: &str) -> Vec<MatchDetail> {
    extract_games(root)
        .iter()
        .filter_map(|g| map_game(g, my_puuid))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要登录并运行英雄联盟客户端"]
    async fn discover_and_fetch_live() {
        let creds = discover(None).expect("discovery failed");
        println!(
            "discovered: port={} token={} source={}",
            creds.port,
            creds.token,
            creds.source.as_str()
        );

        let mut client = LcuClient::new();
        client.credentials = Some(creds);

        let me = client.current_summoner().await.expect("summoner failed");
        println!(
            "summoner: {}#{} lv{}",
            me.game_name.unwrap_or_default(),
            me.tag_line.unwrap_or_default(),
            me.summoner_level.unwrap_or(0)
        );

        let matches = client.tft_matches(&me.puuid).await.expect("matches failed");
        println!("matches: {}", matches.len());
        for m in matches.iter().take(3) {
            println!(
                "  id={} p={} mode={:?} dt={} units={} traits={}",
                m.summary.game_id,
                m.summary.placement,
                m.summary.game_mode,
                m.summary.game_datetime,
                m.units.len(),
                m.traits.len()
            );
        }
        assert!(!matches.is_empty(), "expected at least one match");
    }
}

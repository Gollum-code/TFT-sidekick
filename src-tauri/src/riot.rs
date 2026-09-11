//! Riot API（国际服）客户端 + 高分段 meta 聚合。
//!
//! 国服 LCU 拿不到全服数据，高分段 meta 走 Riot API：
//!   challenger/grandmaster/master 榜 → 玩家 puuid → 对局 ID → 对局详情。
//! 对局详情与国服 LCU 的格式一致，复用 `lcu::map_units` / `lcu::map_traits`。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use thiserror::Error;

use crate::lcu::{map_traits, map_units};
use crate::models::{ParticipantComp, RiotMatch};

#[derive(Debug, Error)]
pub enum RiotError {
    #[error("请求失败: {0}")]
    Request(String),
    #[error("HTTP {0}")]
    Status(u16),
    #[error("解析失败: {0}")]
    Parse(String),
}

pub struct RiotClient {
    http: reqwest::Client,
    api_key: String,
    platform: String, // 如 na1
    routing: String,  // 如 americas
    last_request: Mutex<Instant>,
}

fn routing_for(platform: &str) -> &'static str {
    match platform {
        "na1" | "br1" | "lan1" | "las1" | "oc1" | "la1" | "la2" => "americas",
        "kr" | "jp1" => "asia",
        "eun1" | "euw1" | "tr1" | "ru" => "europe",
        "ph2" | "sg2" | "th2" | "tw2" | "vn2" => "sea",
        _ => "americas",
    }
}

impl RiotClient {
    pub fn new(api_key: String, platform: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key,
            routing: routing_for(platform).to_string(),
            platform: platform.to_string(),
            last_request: Mutex::new(Instant::now() - Duration::from_secs(10)),
        }
    }

    async fn get_json(&self, url: &str) -> Result<serde_json::Value, RiotError> {
        // 简单限流：相邻请求间隔 >= 1.1s（开发 key 约 100 请求/2 分钟）
        let wait = {
            let last = self.last_request.lock().unwrap();
            let elapsed = last.elapsed();
            if elapsed < Duration::from_millis(1100) {
                Some(Duration::from_millis(1100) - elapsed)
            } else {
                None
            }
        };
        if let Some(w) = wait {
            tokio::time::sleep(w).await;
        }
        {
            let mut last = self.last_request.lock().unwrap();
            *last = Instant::now();
        }

        let resp = self
            .http
            .get(url)
            .header("X-Riot-Token", &self.api_key)
            .send()
            .await
            .map_err(|e| RiotError::Request(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(RiotError::Status(status.as_u16()));
        }
        resp.json()
            .await
            .map_err(|e| RiotError::Parse(e.to_string()))
    }

    /// 高分段榜玩家 puuid（challenger / grandmaster / master）。
    pub async fn leaderboard(&self, tier: &str) -> Result<Vec<String>, RiotError> {
        let url = format!(
            "https://{}.api.riotgames.com/tft/league/v1/{}",
            self.platform, tier
        );
        let json = self.get_json(&url).await?;
        let puuids = json
            .get("entries")
            .and_then(|e| e.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e.get("puuid").and_then(|v| v.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(puuids)
    }

    /// 某玩家最近对局 ID 列表。
    pub async fn match_ids(&self, puuid: &str, count: u32) -> Result<Vec<String>, RiotError> {
        let url = format!(
            "https://{}.api.riotgames.com/tft/match/v1/matches/by-puuid/{}/ids?count={}",
            self.routing, puuid, count
        );
        let json = self.get_json(&url).await?;
        Ok(json
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default())
    }

    /// 对局详情（解析全部 8 名玩家）。
    pub async fn match_detail(&self, match_id: &str) -> Result<RiotMatch, RiotError> {
        let url = format!(
            "https://{}.api.riotgames.com/tft/match/v1/matches/{}",
            self.routing, match_id
        );
        let json = self.get_json(&url).await?;
        Ok(parse_match(&json))
    }
}

pub fn parse_match(raw: &serde_json::Value) -> RiotMatch {
    let info = raw.get("info");
    let match_id = raw
        .get("metadata")
        .and_then(|m| m.get("match_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let queue_id = info
        .and_then(|i| i.get("queueId").or_else(|| i.get("queue_id")))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    let tft_set_number = info
        .and_then(|i| i.get("tft_set_number"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let participants = info
        .and_then(|i| i.get("participants"))
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter()
                .map(|p| ParticipantComp {
                    placement: p.get("placement").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                    win: p.get("win").and_then(|v| v.as_bool()).unwrap_or(false),
                    level: p.get("level").and_then(|v| v.as_i64()).map(|l| l as i32),
                    traits: map_traits(p),
                    units: map_units(p),
                })
                .collect()
        })
        .unwrap_or_default();

    RiotMatch {
        match_id,
        queue_id,
        tft_set_number,
        participants,
    }
}

/// 羁绊/阵容聚合结果。
#[derive(Debug, Clone)]
pub struct CompStat {
    pub id: String,
    pub appearances: u32,
    pub wins: u32,
    pub top4: u32,
}

impl CompStat {
    pub fn win_rate(&self) -> f64 {
        if self.appearances == 0 {
            0.0
        } else {
            self.wins as f64 / self.appearances as f64
        }
    }
    pub fn top4_rate(&self) -> f64 {
        if self.appearances == 0 {
            0.0
        } else {
            self.top4 as f64 / self.appearances as f64
        }
    }
}

/// 抓取高分段对局（challenger → grandmaster → master），去重，最多 max_matches 局。
pub async fn crawl_matches(
    client: &RiotClient,
    max_matches: usize,
) -> Result<Vec<RiotMatch>, RiotError> {
    let mut matches = Vec::new();
    let mut seen = std::collections::HashSet::new();
    'outer: for tier in ["challenger", "grandmaster", "master"] {
        let puuids = client.leaderboard(tier).await?;
        for puuid in puuids.iter().take(15) {
            let ids = client.match_ids(puuid, 8).await?;
            for id in ids.into_iter().take(4) {
                if !seen.insert(id.clone()) {
                    continue;
                }
                if let Ok(m) = client.match_detail(&id).await {
                    matches.push(m);
                }
                if matches.len() >= max_matches {
                    break 'outer;
                }
            }
        }
    }
    Ok(matches)
}

/// 装备聚合结果。
#[derive(Debug, Clone)]
pub struct ItemStat {
    pub id: String,
    pub appearances: u32,
    pub wins: u32,
    pub top4: u32,
}

impl ItemStat {
    pub fn win_rate(&self) -> f64 {
        if self.appearances == 0 {
            0.0
        } else {
            self.wins as f64 / self.appearances as f64
        }
    }
    pub fn top4_rate(&self) -> f64 {
        if self.appearances == 0 {
            0.0
        } else {
            self.top4 as f64 / self.appearances as f64
        }
    }
}

/// 按「装备在玩家棋盘上的出现」聚合：出现次数、吃鸡数、前四数。
pub fn aggregate_items(matches: &[RiotMatch]) -> Vec<ItemStat> {
    let mut map: HashMap<String, ItemStat> = HashMap::new();
    for m in matches {
        for p in &m.participants {
            for u in &p.units {
                for item in &u.items {
                    let e = map.entry(item.clone()).or_insert(ItemStat {
                        id: item.clone(),
                        appearances: 0,
                        wins: 0,
                        top4: 0,
                    });
                    e.appearances += 1;
                    if p.placement == 1 {
                        e.wins += 1;
                    }
                    if p.placement <= 4 {
                        e.top4 += 1;
                    }
                }
            }
        }
    }
    let mut out: Vec<ItemStat> = map.into_values().collect();
    out.sort_by(|a, b| b.appearances.cmp(&a.appearances));
    out
}

/// (英雄, 装备) 组合聚合。
#[derive(Debug, Clone)]
pub struct HeroItemStat {
    pub hero: String,
    pub item: String,
    pub appearances: u32,
    pub wins: u32,
    pub top4: u32,
}

/// 按 (英雄, 装备) 聚合：某英雄身上某装备的出现/吃鸡/前四。
pub fn aggregate_unit_items(matches: &[RiotMatch]) -> Vec<HeroItemStat> {
    let mut map: std::collections::HashMap<(String, String), HeroItemStat> =
        std::collections::HashMap::new();
    for m in matches {
        for p in &m.participants {
            for u in &p.units {
                for item in &u.items {
                    let key = (u.character_id.clone(), item.clone());
                    let e = map.entry(key).or_insert(HeroItemStat {
                        hero: u.character_id.clone(),
                        item: item.clone(),
                        appearances: 0,
                        wins: 0,
                        top4: 0,
                    });
                    e.appearances += 1;
                    if p.placement == 1 {
                        e.wins += 1;
                    }
                    if p.placement <= 4 {
                        e.top4 += 1;
                    }
                }
            }
        }
    }
    let mut out: Vec<HeroItemStat> = map.into_values().collect();
    out.sort_by(|a, b| {
        b.appearances
            .cmp(&a.appearances)
            .then_with(|| b.top4.cmp(&a.top4))
    });
    out
}

/// 按「主羁绊」聚合胜率/前四率（按出现次数降序，仅保留样本足够的）。
pub fn aggregate_comps(matches: &[RiotMatch]) -> Vec<CompStat> {
    let mut map: HashMap<String, CompStat> = HashMap::new();
    for m in matches {
        for p in &m.participants {
            let primary = p
                .traits
                .iter()
                .filter(|t| t.tier_current.unwrap_or(0) > 0)
                .max_by_key(|t| t.tier_total.unwrap_or(0))
                .map(|t| t.trait_id.clone());
            let Some(key) = primary else { continue };
            let e = map.entry(key.clone()).or_insert(CompStat {
                id: key,
                appearances: 0,
                wins: 0,
                top4: 0,
            });
            e.appearances += 1;
            if p.placement == 1 {
                e.wins += 1;
            }
            if p.placement <= 4 {
                e.top4 += 1;
            }
        }
    }
    let mut out: Vec<CompStat> = map.into_values().collect();
    out.sort_by(|a, b| b.appearances.cmp(&a.appearances));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要 Riot API key，且会真实消耗请求配额"]
    async fn fetch_and_aggregate_live() {
        let key = std::env::var("RIOT_API_KEY").unwrap_or_else(|_| String::new());
        if key.is_empty() {
            eprintln!("skip: RIOT_API_KEY not set");
            return;
        }
        let client = RiotClient::new(key, "na1");

        let puuids = client.leaderboard("challenger").await.expect("leaderboard");
        println!("challenger players: {}", puuids.len());

        let matches = crawl_matches(&client, 30).await.expect("crawl");
        println!("fetched matches: {}", matches.len());

        let stats = aggregate_comps(&matches);
        println!("comps: {}", stats.len());
        for s in stats.iter().take(10) {
            println!(
                "  {} : {} appearances, win {:.1}%, top4 {:.1}%",
                s.id,
                s.appearances,
                s.win_rate() * 100.0,
                s.top4_rate() * 100.0
            );
        }
        assert!(!stats.is_empty());

        let items = aggregate_items(&matches);
        println!("items: {}", items.len());
        for it in items.iter().take(10) {
            println!(
                "  item {} : {} appearances, top4 {:.1}%",
                it.id,
                it.appearances,
                it.top4_rate() * 100.0
            );
        }
        assert!(!items.is_empty());
    }
}

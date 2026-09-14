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
use crate::static_data::StaticData;

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
    let game_creation = info
        .and_then(|i| i.get("gameCreation").or_else(|| i.get("game_creation")))
        .and_then(|v| v.as_i64());

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
        game_creation,
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
    pub sum_placement: f64,
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

/// 按「主羁绊」聚合，输出阵容构成详情（核心棋子、装备、羁绊）。
pub fn aggregate_comp_details(
    matches: &[RiotMatch],
    sd: &StaticData,
) -> Vec<crate::models::CompDetail> {
    #[derive(Default)]
    struct UnitAgg {
        count: u32,
        sum_tier: f64,
        items: HashMap<String, u32>,
    }
    #[derive(Default)]
    struct CompAgg {
        appearances: u32,
        wins: u32,
        top4: u32,
        sum_placement: f64,
        units: HashMap<String, UnitAgg>,
        traits: HashMap<String, u32>,
    }

    let mut comp_agg: HashMap<String, CompAgg> = HashMap::new();
    for m in matches {
        for p in &m.participants {
            let primary = p
                .traits
                .iter()
                .filter(|t| t.tier_current.unwrap_or(0) > 0)
                .max_by_key(|t| t.tier_total.unwrap_or(0))
                .map(|t| t.trait_id.clone());
            let Some(pkey) = primary else {
                continue;
            };
            let agg = comp_agg.entry(pkey.clone()).or_default();
            agg.appearances += 1;
            if p.placement == 1 {
                agg.wins += 1;
            }
            if p.placement <= 4 {
                agg.top4 += 1;
            }
            agg.sum_placement += p.placement as f64;
            for u in &p.units {
                let ue = agg.units.entry(u.character_id.clone()).or_default();
                ue.count += 1;
                ue.sum_tier += u.tier.unwrap_or(0) as f64;
                for it in &u.items {
                    *ue.items.entry(it.clone()).or_insert(0) += 1;
                }
            }
            for t in &p.traits {
                if t.tier_current.unwrap_or(0) > 0 {
                    *agg.traits.entry(t.trait_id.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    let mut out: Vec<crate::models::CompDetail> = comp_agg
        .iter()
        .map(|(pkey, agg)| {
            let mut units: Vec<crate::models::CompUnit> = agg
                .units
                .iter()
                .map(|(id, ua)| {
                    let top_items = ua
                        .items
                        .iter()
                        .map(|(iid, cnt)| crate::models::HeroItem {
                            id: iid.clone(),
                            name: sd
                                .item_name(iid)
                                .map(String::from)
                                .unwrap_or_else(|| iid.clone()),
                            icon_url: sd.item_icon(iid).unwrap_or_default(),
                            appearances: *cnt,
                            win_rate: 0.0,
                            top4_rate: 0.0,
                        })
                        .collect();
                    crate::models::CompUnit {
                        id: id.clone(),
                        name: sd
                            .champion_name(id)
                            .map(String::from)
                            .unwrap_or_else(|| id.clone()),
                        icon_url: sd.champion_icon(id).unwrap_or_default(),
                        count: ua.count,
                        rate: if agg.appearances > 0 {
                            ua.count as f64 / agg.appearances as f64
                        } else {
                            0.0
                        },
                        avg_tier: if ua.count > 0 {
                            ua.sum_tier / ua.count as f64
                        } else {
                            0.0
                        },
                        items: top_items,
                    }
                })
                .collect();
            units.sort_by(|a, b| b.count.cmp(&a.count));

            let mut traits: Vec<crate::models::CompTraitRow> = agg
                .traits
                .iter()
                .map(|(tid, cnt)| crate::models::CompTraitRow {
                    id: tid.clone(),
                    name: sd
                        .trait_name(tid)
                        .map(String::from)
                        .unwrap_or_else(|| tid.clone()),
                    count: *cnt,
                })
                .collect();
            traits.sort_by(|a, b| b.count.cmp(&a.count));

            crate::models::CompDetail {
                id: pkey.clone(),
                name: sd
                    .trait_name(pkey)
                    .map(String::from)
                    .unwrap_or_else(|| pkey.clone()),
                icon_url: sd.trait_icon(pkey).unwrap_or_default(),
                appearances: agg.appearances,
                win_rate: if agg.appearances > 0 {
                    agg.wins as f64 / agg.appearances as f64
                } else {
                    0.0
                },
                top4_rate: if agg.appearances > 0 {
                    agg.top4 as f64 / agg.appearances as f64
                } else {
                    0.0
                },
                avg_placement: if agg.appearances > 0 {
                    agg.sum_placement / agg.appearances as f64
                } else {
                    0.0
                },
                units,
                traits,
            }
        })
        .collect();
    out.sort_by(|a, b| b.appearances.cmp(&a.appearances));
    out
}

/// 主羁绊识别：激活羁绊中层级最高者。
fn primary_trait_of(p: &ParticipantComp) -> Option<String> {
    p.traits
        .iter()
        .filter(|t| t.tier_current.unwrap_or(0) > 0)
        .max_by_key(|t| t.tier_total.unwrap_or(0))
        .map(|t| t.trait_id.clone())
}

/// 按日期聚合各阵容的占比（用于每日统计 / 阵容趋势）。
pub fn aggregate_daily(matches: &[RiotMatch]) -> Vec<crate::models::DailyRow> {
    use std::collections::HashMap;
    let mut by_day: HashMap<String, Vec<&RiotMatch>> = HashMap::new();
    for m in matches {
        let day = m
            .game_creation
            .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms))
            .map(|t| t.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "未知".to_string());
        by_day.entry(day).or_default().push(m);
    }

    let mut out = Vec::new();
    let mut days: Vec<_> = by_day.into_iter().collect();
    days.sort_by(|a, b| a.0.cmp(&b.0));
    for (date, day_matches) in days {
        // 聚合成当日的 CompStat 列表
        let mut map: HashMap<String, CompStat> = HashMap::new();
        for m in &day_matches {
            for p in &m.participants {
                let Some(pkey) = primary_trait_of(p) else { continue };
                let e = map.entry(pkey.clone()).or_insert(CompStat {
                    id: pkey,
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
        let mut stats: Vec<CompStat> = map.into_values().collect();
        stats.sort_by(|a, b| b.appearances.cmp(&a.appearances));
        // 转成 MetaComp（不含名称，前端可再补；这里简单给 id/占比）
        let comps = stats
            .into_iter()
            .map(|s| {
                let name = s.id.clone();
                crate::models::MetaComp {
                    id: name.clone(),
                    name,
                    icon_url: String::new(),
                    appearances: s.appearances,
                    win_rate: s.win_rate(),
                    top4_rate: s.top4_rate(),
                    win_share: s.wins as f64 / (day_matches.len().max(1) as f64),
                    top4_share: s.top4 as f64 / ((day_matches.len() * 4).max(1) as f64),
                }
            })
            .collect();
        out.push(crate::models::DailyRow { date, comps });
    }
    out
}

/// 互害/受益分析：主羁绊 A 与 B 同场时 A 的表现 vs A 全局表现。
/// 输出「B 在场时 A 的均值名次」低于全局名次，即为被 B 克制。
pub fn aggregate_counters(
    matches: &[RiotMatch],
    min_shared: u32,
) -> Vec<crate::models::CounterRow> {
    use std::collections::HashMap;
    // (a_id, b_id) -> sum placement, count, wins
    let mut pair: HashMap<(String, String), (f64, u32, u32)> = HashMap::new();
    // a_id -> 全局 sum placement, count, wins
    let mut global: HashMap<String, (f64, u32, u32)> = HashMap::new();

    for m in matches {
        // 收集本局各玩家的主羁绊与名次
        let comps: Vec<(String, i32, bool)> = m
            .participants
            .iter()
            .filter_map(|p| {
                primary_trait_of(p).map(|t| (t, p.placement, p.placement == 1))
            })
            .collect();
        for (a, pa, wa) in &comps {
            let g = global.entry(a.clone()).or_insert((0.0, 0, 0));
            g.0 += *pa as f64;
            g.1 += 1;
            if *wa {
                g.2 += 1;
            }
            for (b, _, _) in comps.iter().filter(|(bb, _, _)| bb != a) {
                let e = pair.entry((a.clone(), b.clone())).or_insert((0.0, 0, 0));
                e.0 += *pa as f64;
                e.1 += 1;
                if *wa {
                    e.2 += 1;
                }
            }
        }
    }

    let mut out = Vec::new();
    for ((a_id, b_id), (sum, cnt, wins)) in pair {
        if cnt < min_shared {
            continue;
        }
        let Some(&(_, gcnt, _)) = global.get(&a_id) else { continue };
        if gcnt == 0 {
            continue;
        }
        out.push(crate::models::CounterRow {
            a_id: a_id.clone(),
            a_name: a_id,
            b_id: b_id.clone(),
            b_name: b_id,
            appearances: cnt,
            a_avg_placement: sum / cnt as f64,
            a_win_rate: wins as f64 / cnt as f64,
        });
    }
    // 按「同场均值名次 - 全局均值名次」的差异排序（差异大 = 更被克制）
    out.sort_by(|x, y| {
        let dx = x.a_avg_placement - (global.get(&x.a_id).map(|g| g.0 / g.1 as f64).unwrap_or(0.0));
        let dy = y.a_avg_placement - (global.get(&y.a_id).map(|g| g.0 / g.1 as f64).unwrap_or(0.0));
        dy.partial_cmp(&dx).unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

/// 费用分布：吃鸡/前四阵容中各费用（1~5费）弈子占比。
/// rarity 0~4 对应 1~5 费。
pub fn aggregate_cost_distribution(matches: &[RiotMatch]) -> (Vec<f64>, Vec<f64>) {
    let mut win_counts = [0f64; 5];
    let mut win_total = 0f64;
    let mut top4_counts = [0f64; 5];
    let mut top4_total = 0f64;

    for m in matches {
        for p in &m.participants {
            for u in &p.units {
                let cost = u.rarity.unwrap_or(0) as usize;
                if cost >= 5 {
                    continue;
                }
                if p.placement == 1 {
                    win_counts[cost] += 1.0;
                    win_total += 1.0;
                    top4_counts[cost] += 1.0;
                    top4_total += 1.0;
                } else if p.placement <= 4 {
                    top4_counts[cost] += 1.0;
                    top4_total += 1.0;
                }
            }
        }
    }

    let win = if win_total > 0.0 {
        win_counts.iter().map(|c| c / win_total).collect()
    } else {
        vec![0.0; 5]
    };
    let top4 = if top4_total > 0.0 {
        top4_counts.iter().map(|c| c / top4_total).collect()
    } else {
        vec![0.0; 5]
    };
    (win, top4)
}

/// 具名阵容聚合：主羁绊 + 核心C位（装备最多/费用最高的棋子）。
#[derive(Debug, Clone)]
pub struct NamedCompStat {
    pub key: String,
    pub primary: String,
    pub carry: String,
    pub appearances: u32,
    pub wins: u32,
    pub top4: u32,
    pub sum_placement: f64,
}

/// 识别参与者的具名阵容：返回 (主羁绊id, 核心C位id)。
/// C位 = 装备数优先，其次费用、星级。
fn identify_named_comp(p: &ParticipantComp) -> Option<(String, String)> {
    let primary = p
        .traits
        .iter()
        .filter(|t| t.tier_current.unwrap_or(0) > 0)
        .max_by_key(|t| t.tier_total.unwrap_or(0))
        .map(|t| t.trait_id.clone())?;
    let carry = p
        .units
        .iter()
        .max_by(|a, b| {
            let score = |u: &crate::models::MatchUnit| {
                u.items.len() as i32 * 10 + u.rarity.unwrap_or(0) * 3 + u.tier.unwrap_or(0)
            };
            score(a).cmp(&score(b))
        })
        .map(|u| u.character_id.clone())?;
    Some((primary, carry))
}

/// 具名阵容聚合：按 (主羁绊, C位) 分组。
pub fn aggregate_named_comps(matches: &[RiotMatch]) -> Vec<NamedCompStat> {
    let mut map: HashMap<(String, String), NamedCompStat> = HashMap::new();
    for m in matches {
        for p in &m.participants {
            let Some((primary, carry)) = identify_named_comp(p) else {
                continue;
            };
            let e = map.entry((primary.clone(), carry.clone())).or_insert(NamedCompStat {
                key: format!("{primary}|{carry}"),
                primary,
                carry,
                appearances: 0,
                wins: 0,
                top4: 0,
                sum_placement: 0.0,
            });
            e.appearances += 1;
            if p.placement == 1 {
                e.wins += 1;
            }
            if p.placement <= 4 {
                e.top4 += 1;
            }
            e.sum_placement += p.placement as f64;
        }
    }
    let mut out: Vec<NamedCompStat> = map.into_values().collect();
    out.sort_by(|a, b| b.appearances.cmp(&a.appearances));
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

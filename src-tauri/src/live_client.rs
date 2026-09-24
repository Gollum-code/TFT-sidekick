//! 对局内实时状态读取 —— Game Client API（`127.0.0.1:2999`）。
//!
//! 对局进行中，本地会启动游戏客户端进程，并在 2999 端口暴露只读 HTTP API
//! （Live Client Data API，无需认证）。本模块读取其中的：
//! - `/liveclientdata/activeplayer`：gold / level / health / xp
//! - `/liveclientdata/gamestats`   ：gameTime / gameMode
//! - `/liveclientdata/playerlist`  ：全体玩家 HP / level / 排名
//!
//! 注意：TFT 模式下 board/shop 数据不在此 API 中（需要 Overwolf GEP 或
//! 屏幕识别），本模块只提供 gold / HP / level / stage 这些「确定性数据」。

use std::time::Duration;

use crate::models::{LiveGameState, LivePlayer};

const BASE_URL: &str = "http://127.0.0.1:2999";
/// 单请求超时；三次请求并行，整体最坏等待约 2.5 秒。
const REQUEST_TIMEOUT: Duration = Duration::from_millis(2500);

#[derive(Clone)]
pub struct GameClientReader {
    client: reqwest::Client,
}

impl Default for GameClientReader {
    fn default() -> Self {
        Self::new()
    }
}

impl GameClientReader {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("failed to build reqwest client");
        Self { client }
    }

    async fn get_json(&self, path: &str) -> Option<serde_json::Value> {
        let url = format!("{BASE_URL}{path}");
        self.client
            .get(&url)
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?
            .json()
            .await
            .ok()
    }

    /// 读取当前对局即时状态。任何一步失败返回 None（视为未在对局中）。
    /// 三个端点为并行请求，避免串行 6s 阻塞轮询周期。
    pub async fn read_state(&self) -> Option<LiveGameState> {
        let (active, stats, players) = tokio::join!(
            self.get_json("/liveclientdata/activeplayer"),
            self.get_json("/liveclientdata/gamestats"),
            self.get_json("/liveclientdata/playerlist"),
        );
        let active = active?;
        let stats = stats?;

        let game_time = stats
            .get("gameTime")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let my_summoner = active
            .get("summonerName")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // playerlist 顺序不保证是名次，按血量降序排序后重新编号 1..8。
        let mut list: Vec<LivePlayer> = players
            .unwrap_or(serde_json::json!([]))
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| {
                        let o = p.as_object()?;
                        let name = o
                            .get("summonerName")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_default();
                        let health = o
                            .get("health")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        let max_health = o
                            .get("maxHealth")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(100.0);
                        Some(LivePlayer {
                            puuid: o
                                .get("puuid")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            name,
                            health,
                            max_health,
                            level: o.get("level").and_then(|v| v.as_i64()),
                            rank: None,
                            is_me: match &my_summoner {
                                Some(me) => {
                                    o.get("summonerName")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s == me)
                                        .unwrap_or(false)
                                }
                                None => false,
                            },
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        list.sort_by(|a, b| {
            b.health
                .partial_cmp(&a.health)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (i, p) in list.iter_mut().enumerate() {
            p.rank = Some(i as i64 + 1);
        }

        Some(LiveGameState {
            available: true,
            gold: normalize_gold(active.get("currentGold")),
            level: active.get("level").and_then(|v| v.as_i64()),
            health: active.get("currentHealth").and_then(|v| v.as_f64()),
            max_health: active.get("maxHealth").and_then(|v| v.as_f64()),
            xp: active
                .get("xp")
                .and_then(|v| v.as_f64())
                .or_else(|| active.get("xp").and_then(|v| v.as_i64()).map(|x| x as f64)),
            stage: estimate_stage(game_time),
            game_time,
            players: list,
        })
    }
}

/// TFT 的 currentGold 在部分版本返回无意义小数（如 553.04），实际金币总是整数且在 0..=500。
/// 过滤异常值（负数 / 超大值），并做取整，避免把错误数据展示给用户。
fn normalize_gold(v: Option<&serde_json::Value>) -> Option<i64> {
    let n = v?.as_f64()?;
    if !(0.0..=500.0).contains(&n) {
        return None;
    }
    Some(n.round() as i64)
}

/// 由 gameTime(秒) 估算当前 stage/回合。TFT 每回合时长随阶段递增：
/// 阶段 1 约 35s/回合，阶段 2 约 50s，阶段 3+ 约 60s，采用分段近似。
/// 阶段 1 为 1-1..1-4，之后每阶段 7 回合（2-1..2-7, 3-1..3-7, ...）。
fn estimate_stage(game_time_s: f64) -> String {
    // 累计时长（秒）→ 回合号。逐回合累加标准时长。
    // durations[i] = 第 i+1 回合的估算时长
    let durations = [
        40.0, // 1-1
        40.0, // 1-2
        45.0, // 1-3
        45.0, // 1-4
    ];
    let mut t = game_time_s;
    let mut round: i32 = 1;
    // 阶段 1
    for d in durations {
        if t < d {
            return format!("1-{}", round);
        }
        t -= d;
        round += 1;
    }
    // 阶段 2 起：每回合时长随阶段增长
    let mut stage = 2;
    let mut minor = 1;
    loop {
        let round_dur = 50.0 + (stage - 2) as f64 * 2.5; // 2阶段≈50s，3阶段≈52.5...
        if t < round_dur {
            return format!("{}-{}", stage, minor);
        }
        t -= round_dur;
        minor += 1;
        if minor > 7 {
            minor = 1;
            stage += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_estimation() {
        assert_eq!(estimate_stage(0.0), "1-1");
        assert_eq!(estimate_stage(20.0), "1-1");
        assert_eq!(estimate_stage(45.0), "1-2");
        assert_eq!(estimate_stage(85.0), "1-3");
        assert_eq!(estimate_stage(130.0), "1-4");
        assert_eq!(estimate_stage(171.0), "2-1");
        // 2 阶段 7 回合 × 50s = 350s → 3-1
        assert_eq!(estimate_stage(171.0 + 350.0), "3-1");
        // 3 阶段开始每回合 ~52.5s
        assert_eq!(estimate_stage(171.0 + 350.0 + 52.5), "3-2");
    }

    #[test]
    fn gold_normalization() {
        let j = |n: f64| serde_json::json!(n);
        assert_eq!(normalize_gold(Some(&j(50.0))), Some(50));
        assert_eq!(normalize_gold(Some(&j(12.0))), Some(12));
        assert_eq!(normalize_gold(Some(&j(12.4))), Some(12));
        // 负数值判非法
        assert_eq!(normalize_gold(Some(&j(-1.0))), None);
        // 超大值（正常对局金币不会 >500）判非法
        assert_eq!(normalize_gold(Some(&j(553.04))), None);
        assert_eq!(normalize_gold(Some(&j(5000.0))), None);
        assert_eq!(normalize_gold(None), None);
    }
}
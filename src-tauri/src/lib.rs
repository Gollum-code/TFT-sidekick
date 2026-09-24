mod db;
mod lcu;
mod live_client;
mod ocr;
pub mod models;
pub mod riot;
pub mod static_data;
pub mod updater;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::lcu::LcuClient;
use crate::live_client::GameClientReader;
use crate::models::{
    AppInfo, ChampionData, EnrichedMatch, ItemData, MatchSummary, HeroItemRow, LiveGameState,
    MetaComp, MetaItem, MetaResult, NamedCount, Profile, RulesSection, ShopOddsLevel, Stats,
    SyncResult, TraitData,
};
use crate::static_data::StaticData;

struct AppState {
    db: Arc<Mutex<db::Database>>,
    lcu: Arc<tokio::sync::Mutex<LcuClient>>,
    game_client: GameClientReader,
    static_data: Arc<tokio::sync::Mutex<Option<Arc<StaticData>>>>,
    data_dir: PathBuf,
    meta_json_path: PathBuf,
}

async fn ensure_static_data(state: &AppState) -> Result<Arc<StaticData>, String> {
    {
        let guard = state.static_data.lock().await;
        if let Some(sd) = guard.as_ref() {
            return Ok(sd.clone());
        }
    }

    let path = state.data_dir.join("cdragon_tft_zhcn.json");
    if !path.exists() {
        StaticData::download(&path).await.map_err(|e| e.to_string())?;
    }
    let sd = StaticData::load(&path)
        .map_err(|e| e.to_string())?
        .into_arc();
    *state.static_data.lock().await = Some(sd.clone());
    Ok(sd)
}

#[tauri::command]
fn list_matches(state: State<'_, AppState>, limit: Option<u32>) -> Result<Vec<MatchSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_matches(limit.unwrap_or(20)).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_match(state: State<'_, AppState>, game_id: String) -> Result<EnrichedMatch, String> {
    let detail = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_match(&game_id).map_err(|e| e.to_string())?
    };
    let sd = ensure_static_data(&state).await?;
    Ok(sd.enrich(detail))
}

#[tauri::command]
async fn get_stats(state: State<'_, AppState>) -> Result<Stats, String> {
    let raw = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_stats().map_err(|e| e.to_string())?
    };
    let sd = ensure_static_data(&state).await?;

    let total = raw.total as f64;
    Ok(Stats {
        total: raw.total,
        win_rate: if total > 0.0 { raw.wins as f64 / total } else { 0.0 },
        top4_rate: if total > 0.0 { raw.top4 as f64 / total } else { 0.0 },
        avg_placement: if total > 0.0 { raw.sum_placement as f64 / total } else { 0.0 },
        avg_level: if raw.level_count > 0 {
            raw.sum_level as f64 / raw.level_count as f64
        } else {
            0.0
        },
        placement_dist: raw.placement_dist.to_vec(),
        top_traits: raw
            .top_traits
            .iter()
            .map(|(id, count)| NamedCount {
                id: id.clone(),
                name: sd.trait_name(id).unwrap_or("").to_string(),
                icon_url: sd.trait_icon(id).unwrap_or_default(),
                count: *count,
            })
            .collect(),
        top_units: raw
            .top_units
            .iter()
            .map(|(id, count)| NamedCount {
                id: id.clone(),
                name: sd.champion_name(id).unwrap_or("").to_string(),
                icon_url: sd.champion_icon(id).unwrap_or_default(),
                count: *count,
            })
            .collect(),
    })
}

#[tauri::command]
async fn app_info(state: State<'_, AppState>, app: AppHandle) -> Result<AppInfo, String> {
    let db_path = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.path().display().to_string()
    };
    let lcu_connected = state.lcu.lock().await.is_connected();
    Ok(AppInfo {
        name: app.package_info().name.clone(),
        version: app.package_info().version.to_string(),
        db_path,
        lcu_connected,
    })
}

#[tauri::command]
async fn get_items(state: State<'_, AppState>) -> Result<Vec<ItemData>, String> {
    let sd = ensure_static_data(&state).await?;
    Ok(sd.items())
}

#[tauri::command]
async fn get_champions(state: State<'_, AppState>) -> Result<Vec<ChampionData>, String> {
    let sd = ensure_static_data(&state).await?;
    Ok(sd.champions())
}

#[tauri::command]
async fn get_traits(state: State<'_, AppState>) -> Result<Vec<TraitData>, String> {
    let sd = ensure_static_data(&state).await?;
    Ok(sd.traits())
}

#[tauri::command]
async fn get_augments(state: State<'_, AppState>) -> Result<Vec<ItemData>, String> {
    let sd = ensure_static_data(&state).await?;
    Ok(sd.augments())
}

#[tauri::command]
fn get_shop_odds() -> Vec<ShopOddsLevel> {
    const ODDS: [[f32; 5]; 10] = [
        [100.0, 0.0, 0.0, 0.0, 0.0],
        [100.0, 0.0, 0.0, 0.0, 0.0],
        [75.0, 25.0, 0.0, 0.0, 0.0],
        [55.0, 30.0, 15.0, 0.0, 0.0],
        [45.0, 33.0, 20.0, 2.0, 0.0],
        [30.0, 40.0, 25.0, 5.0, 0.0],
        [19.0, 35.0, 35.0, 10.0, 1.0],
        [18.0, 25.0, 36.0, 18.0, 3.0],
        [10.0, 20.0, 25.0, 35.0, 10.0],
        [5.0, 10.0, 20.0, 40.0, 25.0],
    ];
    ODDS.iter()
        .enumerate()
        .map(|(i, o)| ShopOddsLevel {
            level: (i + 1) as i32,
            odds: o.to_vec(),
        })
        .collect()
}

#[tauri::command]
fn get_game_rules() -> Vec<RulesSection> {
    vec![
        RulesSection {
            title: "利息机制".into(),
            rows: vec![
                vec!["规则".into(), "说明".into()],
                vec!["利息".into(), "每存 10 金币，回合结束额外 +1 金币".into()],
                vec!["利息上限".into(), "每回合最多 5 利息（存款 50 金币封顶）".into()],
                vec!["连胜/连败奖励".into(), "2 连胜 +1，4 连胜及以上 +2（具体数值随赛季微调）".into()],
            ],
        },
        RulesSection {
            title: "玩家伤害（失败掉血）".into(),
            rows: vec![
                vec!["阶段".into(), "基础掉血".into()],
                vec!["1-1 ~ 1-4".into(), "0（野怪回合不掉血）".into()],
                vec!["2-1 ~ 2-7".into(), "2".into()],
                vec!["3-1 ~ 3-7".into(), "3".into()],
                vec!["4-1 ~ 4-7".into(), "4".into()],
                vec!["5-1 及以上".into(), "5".into()],
                vec!["存活敌方单位加成".into(), "每个存活棋子追加伤害（约等于棋子费用，1 费≈2、5 费≈6）".into()],
            ],
        },
        RulesSection {
            title: "海克斯出现概率（各补丁可能调整）".into(),
            rows: vec![
                vec!["海克斯".into(), "白银".into(), "黄金".into(), "棱彩".into()],
                vec!["第 1 个海克斯".into(), "≈55%".into(), "≈35%".into(), "≈10%".into()],
                vec!["第 2 个海克斯".into(), "≈40%".into(), "≈50%".into(), "≈10%".into()],
                vec!["第 3 个海克斯".into(), "≈25%".into(), "≈60%".into(), "≈15%".into()],
            ],
        },
        RulesSection {
            title: "商店刷新规则".into(),
            rows: vec![
                vec!["规则".into(), "说明".into()],
                vec!["刷新费用".into(), "每次刷新商店 2 金币".into()],
                vec!["免费刷新".into(), "每次对战胜利后获得 1 次免费刷新（不可叠加）".into()],
                vec!["野怪回合".into(), "野怪回合结束免费刷新一次".into()],
            ],
        },
    ]
}

/// 读取 tftable_data.json（网站元数据快照）。
#[tauri::command]
fn get_tftable_data(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    const EMBEDDED: &str = include_str!("../../tftable_data.json");
    let path = state
        .meta_json_path
        .parent()
        .unwrap()
        .join("tftable_data.json");
    let raw = if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| e.to_string())?
    } else {
        EMBEDDED.to_string()
    };
    serde_json::from_str(&raw).map_err(|e| format!("tftable_data.json 格式错误: {e}"))
}

/// 检查 tftable.cc 上是否有更新的补丁数据。
/// 抓取 https://tftable.cc/meta 的 __NEXT_DATA__，提取 patch 号 / 样本量 / 时间戳，
/// 与本地 tftable_data.json 对比，返回是否有更新。
#[tauri::command]
async fn check_tftable_update(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    // 本地当前 patch
    let local_patch: Option<String> = {
        let current = get_tftable_data(state).ok();
        current
            .as_ref()
            .map(|v| v.get("metaStats").and_then(|m| m.get("metadata")))
            .flatten()
            .and_then(|m| m.get("patch").and_then(|p| p.as_str()))
            .map(|s| s.to_string())
    };

    // 抓取 tftable.cc/meta 页面
    let client = reqwest::Client::new();
    let html = client
        .get("https://tftable.cc/meta")
        .header("User-Agent", "TFT-Sidekick/0.1")
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    // 提取 __NEXT_DATA__
    let marker = r#"<script id="__NEXT_DATA__""#;
    let start = html
        .find(marker)
        .and_then(|i| html[i..].find('>').map(|j| i + j + 1))
        .ok_or("未找到 NEXT_DATA")?;
    let end = html[start..]
        .find("</script>")
        .ok_or("NEXT_DATA 未闭合")?
        + start;
    let json: serde_json::Value =
        serde_json::from_str(&html[start..end]).map_err(|e| format!("解析失败: {e}"))?;

    let meta_stats = json
        .pointer("/props/pageProps/metaStats")
        .ok_or("metaStats 缺失")?;
    let latest_patch = meta_stats
        .pointer("/metadata/patch")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let sample_size = meta_stats
        .pointer("/metadata/total_sample_size")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let updated_at = meta_stats
        .pointer("/metadata/timestamp")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let has_update = match &local_patch {
        Some(local) => local != &latest_patch,
        None => true,
    };

    Ok(serde_json::json!({
        "has_update": has_update,
        "local_patch": local_patch,
        "latest_patch": latest_patch,
        "sample_size": sample_size,
        "updated_at": updated_at,
    }))
}

/// 从 tftable.cc 抓取所有页面并重建 tftable_data.json（在线更新数据）。
/// 返回保存结果：成功与否 + 文件路径。
#[tauri::command]
async fn update_tftable_data(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let new_data = updater::update_tftable_data().await?;

    // 保存到 data_dir
    let path = state
        .meta_json_path
        .parent()
        .unwrap()
        .join("tftable_data.json");
    let json_str = serde_json::to_string_pretty(&new_data).map_err(|e| e.to_string())?;
    std::fs::write(&path, &json_str).map_err(|e| format!("写入 tftable_data.json 失败: {e}"))?;

    Ok(serde_json::json!({
        "success": true,
        "path": path.to_string_lossy(),
        "patch": new_data.get("patch").and_then(|v| v.as_str()).unwrap_or(""),
        "sample_size": new_data.get("sampleSize").and_then(|v| v.as_u64()).unwrap_or(0),
    }))
}

/// 项目里的 meta.json（旧数据，保留兼容）。
#[tauri::command]
async fn get_meta(state: State<'_, AppState>) -> Result<MetaResult, String> {
    let path = &state.meta_json_path;
    if !path.exists() {
        return Err("meta.json 未准备。请先运行 `cargo run --bin crawl_meta` 生成".into());
    }
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let v: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("meta.json 格式错误: {e}"))?;

    let comps: Vec<MetaComp> = serde_json::from_value(
        v.get("comps")
            .cloned()
            .unwrap_or(serde_json::json!([])),
    )
    .unwrap_or_default();
    let items: Vec<MetaItem> = serde_json::from_value(
        v.get("items")
            .cloned()
            .unwrap_or(serde_json::json!([])),
    )
    .unwrap_or_default();
    let hero_items: Vec<HeroItemRow> = serde_json::from_value(
        v.get("hero_items")
            .cloned()
            .unwrap_or(serde_json::json!([])),
    )
    .unwrap_or_default();
    let named_comps: Vec<crate::models::NamedComp> = serde_json::from_value(
        v.get("named_comps")
            .cloned()
            .unwrap_or(serde_json::json!([])),
    )
    .unwrap_or_default();

    let comp_details: Vec<crate::models::CompDetail> = serde_json::from_value(
        v.get("comp_details")
            .cloned()
            .unwrap_or(serde_json::json!([])),
    )
    .unwrap_or_default();
    let daily: Vec<crate::models::DailyRow> = serde_json::from_value(
        v.get("daily").cloned().unwrap_or(serde_json::json!([])),
    )
    .unwrap_or_default();
    let counters: Vec<crate::models::CounterRow> = serde_json::from_value(
        v.get("counters").cloned().unwrap_or(serde_json::json!([])),
    )
    .unwrap_or_default();

    let cost_win: Vec<f64> = serde_json::from_value(
        v.get("cost_win").cloned().unwrap_or(serde_json::json!([])),
    )
    .unwrap_or_default();
    let cost_top4: Vec<f64> = serde_json::from_value(
        v.get("cost_top4").cloned().unwrap_or(serde_json::json!([])),
    )
    .unwrap_or_default();

    let sample_count = v
        .get("sample_count")
        .and_then(|x| x.as_u64())
        .unwrap_or(0) as u32;

    Ok(MetaResult {
        comps,
        named_comps,
        items,
        hero_items,
        comp_details,
        daily,
        counters,
        cost_win,
        cost_top4,
        sample_count,
        added_count: 0,
    })
}

/// 对局中悬浮窗的个人信息：段位 + 历史战绩摘要。
#[tauri::command]
async fn get_profile(state: State<'_, AppState>) -> Result<Profile, String> {
    let ranked = {
        let mut lcu = state.lcu.lock().await;
        if !lcu.is_connected() {
            lcu.discover_and_connect().map_err(|e| e.to_string())?;
        }
        lcu.ranked_stats().await.map_err(|e| e.to_string())?
    };

    let (wins, top4, sum_placement, total) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let ms = db.list_matches(100).map_err(|e| e.to_string())?;
        let mut wins = 0u32;
        let mut top4 = 0u32;
        let mut sum_placement = 0f64;
        for m in &ms {
            if m.placement == 1 {
                wins += 1;
            }
            if m.placement <= 4 {
                top4 += 1;
            }
            sum_placement += m.placement as f64;
        }
        (wins, top4, sum_placement, ms.len() as u32)
    };

    Ok(Profile {
        rank_tier: ranked.tier,
        rank_division: ranked.division,
        rank_lp: ranked.league_points,
        rank_wins: ranked.wins,
        rank_losses: ranked.losses,
        win_rate: if total > 0 { wins as f64 / total as f64 } else { 0.0 },
        top4_rate: if total > 0 { top4 as f64 / total as f64 } else { 0.0 },
        avg_placement: if total > 0 {
            sum_placement / total as f64
        } else {
            0.0
        },
        total,
    })
}

/// 读取当前对局即时状态（2999 Game Client API）。不在对局中返回 available=false。
#[tauri::command]
async fn get_live_state(state: State<'_, AppState>) -> Result<LiveGameState, String> {
    Ok(state
        .game_client
        .read_state()
        .await
        .unwrap_or_default())
}

async fn do_sync(
    lcu: &Arc<tokio::sync::Mutex<LcuClient>>,
    db: &Arc<Mutex<db::Database>>,
) -> Result<SyncResult, String> {
    let mut lcu_guard = lcu.lock().await;
    if !lcu_guard.is_connected() {
        lcu_guard.discover_and_connect().map_err(|e| e.to_string())?;
    }
    let me = lcu_guard.current_summoner().await.map_err(|e| e.to_string())?;
    let matches = lcu_guard.tft_matches(&me.puuid).await.map_err(|e| e.to_string())?;
    let source = lcu_guard.source().unwrap_or("").to_string();
    let summoner = Some(models::SummonerInfo {
        puuid: me.puuid.clone(),
        game_name: me.game_name,
        tag_line: me.tag_line,
        summoner_level: me.summoner_level,
    });
    drop(lcu_guard);

    let db_guard = db.lock().map_err(|e| e.to_string())?;
    let mut count = 0u32;
    for detail in matches {
        if db_guard.insert_match(&detail).map_err(|e| e.to_string())? {
            count += 1;
        }
    }
    drop(db_guard);

    Ok(SyncResult {
        count,
        source,
        summoner,
    })
}

#[tauri::command]
async fn sync_matches(state: State<'_, AppState>) -> Result<SyncResult, String> {
    do_sync(&state.lcu, &state.db).await
}

/// 后台轮询 gameflow-phase，检测对局结束并自动同步 + 通知前端。
async fn gameflow_loop(
    app: AppHandle,
    lcu: Arc<tokio::sync::Mutex<LcuClient>>,
    db: Arc<Mutex<db::Database>>,
    game_client: GameClientReader,
) {
    let mut prev_phase = String::new();
    let mut connected = false;
    loop {
        let delay = if connected { 3 } else { 8 };
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;

        let phase = {
            let mut guard = lcu.lock().await;
            if !guard.is_connected() && guard.discover_and_connect().is_err() {
                None
            } else {
                guard.gameflow_phase().await.ok()
            }
        };

        match phase {
            Some(p) => {
                // 从「未连接」变为「已连接」：同步一次并通知前端。
                if !connected {
                    connected = true;
                    if let Ok(sr) = do_sync(&lcu, &db).await {
                        let _ = app.emit("lcu-status", Some(sr));
                    }
                }

                // 对局中：轮询 2999 实时状态并推送给 overlay / 主窗口。
                // read_state 内部并行请求，任一失败返回 None；无需再单独探测 available。
                if p == "InProgress" {
                    if let Some(live) = game_client.read_state().await {
                        let _ = app.emit("overlay-live", live);
                    }
                }

                // 对局结束：InProgress -> 其它阶段
                if prev_phase == "InProgress" && p != "InProgress" {
                    if let Ok(sr) = do_sync(&lcu, &db).await {
                        if sr.count > 0 {
                            let latest = {
                                let d = db.lock().unwrap();
                                d.list_matches(1).ok().and_then(|mut v| {
                                    if v.is_empty() {
                                        None
                                    } else {
                                        Some(v.remove(0))
                                    }
                                })
                            };
                            let _ = app.emit("match-ended", latest);
                        }
                    }
                    // 通知 overlay：对局结束
                    let _ = app.emit("overlay-phase", "EndOfGame");
                    // 清理实时状态
                    let _ = app.emit("overlay-live", LiveGameState::default());
                    // 对局结束自动收起悬浮窗
                    if let Some(w) = app.get_webview_window("overlay") {
                        if w.is_visible().unwrap_or(false) {
                            let _ = w.hide();
                        }
                    }
                }

                // 进入对局：通知 overlay 并自动弹出
                if p == "InProgress" && prev_phase != "InProgress" {
                    let _ = app.emit("overlay-phase", "InProgress");
                    if let Some(w) = app.get_webview_window("overlay") {
                        if !w.is_visible().unwrap_or(true) {
                            let _ = w.show();
                        }
                    }
                }
                prev_phase = p;
            }
            None => {
                if connected {
                    connected = false;
                    let _ = app.emit("lcu-status", None::<SyncResult>);
                }
                prev_phase = String::new();
            }
        }
    }
}

/// 保存窗口位置到 window_state.json（main / overlay 各自独立记录）。
/// 兼容 `tauri::Window` 与 `tauri::WebviewWindow`。
fn save_window_state<R: tauri::Runtime>(window: &tauri::window::Window<R>) -> tauri::Result<()> {
    let size = window.outer_size()?;
    let position = window.outer_position().ok();
    let path = window.app_handle().path().app_data_dir()?.join("window_state.json");
    let (x, y) = match position {
        Some(p) => (Some(p.x), Some(p.y)),
        None => (None, None),
    };
    let key = window.label().to_string();

    // 读取已有状态，合并写入
    let mut state: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    if !state.is_object() {
        state = serde_json::json!({});
    }
    let entry = serde_json::json!({
        "width": size.width,
        "height": size.height,
        "x": x,
        "y": y,
    });
    if let Some(obj) = state.as_object_mut() {
        obj.insert(key.to_string(), entry);
    }
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    let _ = std::fs::write(&path, state.to_string());
    Ok(())
}

/// 启动时恢复窗口尺寸/位置（如果存在且是有效值）。
fn restore_window_state(app: &tauri::AppHandle) {
    let path = match app.path().app_data_dir() {
        Ok(d) => d.join("window_state.json"),
        Err(_) => return,
    };
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return,
    };
    let v: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return,
    };

    // 兼容旧格式：顶层直接是 main 的字段
    let get_entry = |label: &str| -> Option<serde_json::Value> {
        if let Some(e) = v.get(label) {
            return Some(e.clone());
        }
        if label == "main" && v.get("width").is_some() {
            return Some(v.clone());
        }
        None
    };

    for (label, min_w, min_h) in [("main", 400u32, 300u32), ("overlay", 200, 200)] {
        let Some(entry) = get_entry(label) else {
            continue;
        };
        let Some(w) = app.get_webview_window(label) else {
            continue;
        };
        if let (Some(wd), Some(h)) = (
            entry.get("width").and_then(|x| x.as_u64()),
            entry.get("height").and_then(|x| x.as_u64()),
        ) {
            if wd >= min_w as u64 && h >= min_h as u64 && label == "main" {
                let _ = w.set_size(tauri::PhysicalSize {
                    width: wd as u32,
                    height: h as u32,
                });
            }
        }
        if let (Some(x), Some(y)) = (
            entry.get("x").and_then(|x| x.as_i64()),
            entry.get("y").and_then(|x| x.as_i64()),
        ) {
            let _ = w.set_position(tauri::PhysicalPosition { x: x as i32, y: y as i32 });
        }
    }
}

/// 游戏内辅助悬浮窗：创建 or 显示/隐藏。
#[tauri::command]
fn toggle_overlay(app: AppHandle) -> Result<bool, String> {
    let Some(w) = app.get_webview_window("overlay") else {
        return Err("overlay 窗口未定义".into());
    };
    if w.is_visible().unwrap_or(false) {
        // 隐藏前保存位置（写入 window_state.json 的 overlay 键）
        if let Ok(size) = w.outer_size() {
            if let Ok(position) = w.outer_position() {
                if let Ok(dir) = app.path().app_data_dir() {
                    let path = dir.join("window_state.json");
                    let mut state: serde_json::Value = std::fs::read_to_string(&path)
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_else(|| serde_json::json!({}));
                    if !state.is_object() {
                        state = serde_json::json!({});
                    }
                    if let Some(obj) = state.as_object_mut() {
                        obj.insert(
                            "overlay".to_string(),
                            serde_json::json!({
                                "width": size.width,
                                "height": size.height,
                                "x": position.x,
                                "y": position.y,
                            }),
                        );
                    }
                    let _ = std::fs::create_dir_all(path.parent().unwrap());
                    let _ = std::fs::write(&path, state.to_string());
                }
            }
        }
        let _ = w.hide();
        Ok(false)
    } else {
        let _ = w.show();
        let _ = w.set_focus();
        Ok(true)
    }
}

/// 截图游戏窗口商店区域并 OCR 识别文字（Windows 自带 OCR，零依赖）。
/// 商店 5 格位于游戏窗口客户区底部中间；按百分比定位以适配任意分辨率/无边框窗口。
/// 返回识别出的整段文本（棋子名之间有空白，前端模糊匹配）。
#[tauri::command]
fn ocr_shop_screen() -> Result<String, String> {
    let dir = std::env::temp_dir().join("tft-sidekick-ocr");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let png = dir.join("shop_capture.png");
    // 商店区域（百分比，相对客户区）：横向 15%~85%，纵向 86%~100%（底部一条）。
    // 2 倍放大后再 OCR，提高棋子名小字识别率。
    ocr::capture_game_window(&png, Some((15, 86, 70, 14)), 2.0)
        .map_err(|e| e.to_string())?;
    let text = ocr::ocr_image(&png).map_err(|e| e.to_string())?;
    Ok(text)
}

/// 更新 overlay 中显示的对局状态（由 gameflow_loop 调用）。
#[tauri::command]
async fn set_overlay_state(state: State<'_, AppState>, app: AppHandle) -> Result<serde_json::Value, String> {
    let phase = {
        let mut lcu = state.lcu.lock().await;
        if !lcu.is_connected() {
            lcu.discover_and_connect().map_err(|e| e.to_string())?;
        }
        lcu.gameflow_phase().await.map_err(|e| e.to_string())?
    };
    let rank = {
        let lcu = state.lcu.lock().await;
        lcu.ranked_stats().await.map_err(|e| e.to_string())?
    };
    let me = {
        let lcu = state.lcu.lock().await;
        lcu.current_summoner().await.map_err(|e| e.to_string())?
    };
    let _ = app.emit("overlay-state", serde_json::json!({
        "phase": phase,
        "summoner": me.game_name,
        "rank_tier": rank.tier,
        "rank_division": rank.division,
        "rank_lp": rank.league_points,
    }));
    Ok(serde_json::json!({
        "phase": phase,
        "summoner": me.game_name,
        "rank_tier": rank.tier,
        "rank_division": rank.division,
        "rank_lp": rank.league_points,
    }))
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "打开主界面", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "刷新数据", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &refresh, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("TFT Sidekick")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "refresh" => {
                let _ = app.emit("refresh-data", "");
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let database = db::Database::open(&data_dir.join("tft-sidekick.db"))?;

            let db = Arc::new(Mutex::new(database));
            let lcu = Arc::new(tokio::sync::Mutex::new(LcuClient::new()));
            let game_client = GameClientReader::new();
            let game_client_loop = game_client.clone();
            let static_data = Arc::new(tokio::sync::Mutex::new(None::<Arc<StaticData>>));

            // 定位 meta.json：优先项目根目录（开发/工具生成处），否则数据目录
            let meta_json_path = {
                let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../meta.json");
                if project_root.exists() {
                    project_root
                } else {
                    data_dir.join("meta.json")
                }
            };

            app.manage(AppState {
                db: db.clone(),
                lcu: lcu.clone(),
                game_client,
                static_data: static_data.clone(),
                data_dir: data_dir.clone(),
                meta_json_path,
            });

            setup_tray(app.handle())?;
            restore_window_state(app.handle());

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                gameflow_loop(handle, lcu, db, game_client_loop).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    // 保存窗口尺寸/位置（下次启动恢复）
                    let _ = save_window_state(window);
                    // 点关闭只隐藏到托盘，不退出进程。
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_matches,
            get_match,
            get_stats,
            app_info,
            sync_matches,
            get_items,
            get_champions,
            get_traits,
            get_augments,
            get_shop_odds,
            get_game_rules,
            get_meta,
            get_tftable_data,
            check_tftable_update,
            update_tftable_data,
            get_profile,
            toggle_overlay,
            set_overlay_state,
            get_live_state,
            ocr_shop_screen
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "需要登录并运行英雄联盟客户端"]
    async fn full_pipeline_live() {
        let mut client = LcuClient::new();
        client.discover_and_connect().expect("discover failed");
        let me = client.current_summoner().await.expect("summoner failed");
        let matches = client.tft_matches(&me.puuid).await.expect("matches failed");
        assert!(!matches.is_empty());

        let path = std::env::temp_dir().join("cdragon_tft_zhcn.json");
        let sd = if path.exists() {
            StaticData::load(&path).expect("load static")
        } else {
            StaticData::download(&path).await.expect("download static");
            StaticData::load(&path).expect("load static")
        };

        let detail = matches.into_iter().next().unwrap();
        let enriched = sd.enrich(detail);
        assert!(enriched.units.iter().all(|u| !u.name.is_empty()));
    }
}

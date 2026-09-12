mod db;
mod lcu;
pub mod models;
pub mod riot;
pub mod static_data;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::lcu::LcuClient;
use crate::models::{
    AppInfo, ChampionData, EnrichedMatch, ItemData, MatchSummary, HeroItemRow, MetaComp, MetaItem,
    MetaResult, NamedCount, Profile, RulesSection, ShopOddsLevel, Stats, SyncResult, TraitData,
};
use crate::static_data::StaticData;

struct AppState {
    db: Arc<Mutex<db::Database>>,
    lcu: Arc<tokio::sync::Mutex<LcuClient>>,
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

/// 读取项目里的 meta.json（提前由 crawl_meta 工具生成）。
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

    let sample_count = v
        .get("sample_count")
        .and_then(|x| x.as_u64())
        .unwrap_or(0) as u32;

    Ok(MetaResult {
        comps,
        items,
        hero_items,
        comp_details,
        daily,
        counters,
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

fn create_overlay(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window("overlay").is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("index.html".into()))
        .title("TFT Sidekick 悬浮窗")
        .inner_size(280.0, 160.0)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .build()?;
    Ok(())
}

#[tauri::command]
fn toggle_overlay(app: AppHandle) -> Result<bool, String> {
    if let Some(w) = app.get_webview_window("overlay") {
        let visible = w.is_visible().unwrap_or(false);
        if visible {
            let _ = w.hide();
        } else {
            let _ = w.show();
        }
        Ok(!visible)
    } else {
        create_overlay(&app).map_err(|e| e.to_string())?;
        Ok(true)
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "打开主界面", true, None::<&str>)?;
    let overlay = MenuItem::with_id(app, "overlay", "显示/隐藏悬浮窗", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &overlay, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("TFT Sidekick")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "overlay" => {
                let _ = toggle_overlay(app.clone());
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
                static_data: static_data.clone(),
                data_dir: data_dir.clone(),
                meta_json_path,
            });

            setup_tray(app.handle())?;

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                gameflow_loop(handle, lcu, db).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
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
            toggle_overlay,
            get_items,
            get_champions,
            get_traits,
            get_augments,
            get_shop_odds,
            get_game_rules,
            get_meta,
            get_profile
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

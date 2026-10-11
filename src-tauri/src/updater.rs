//! 在线更新 tftable_data.json：从 tftable.cc 抓取所有页面并合并重建。
//! 数据来源是各页面 HTML 内嵌的 __NEXT_DATA__ JSON。

use std::collections::HashMap;

use serde_json::{json, Map, Value};

const COMP_SLUGS: [&str; 29] = [
    "dragon_95",
    "elderwood_95",
    "vanguard_morgana",
    "hunter_sivir",
    "veigar_reroll",
    "zyra_fatalis",
    "juggernaut_flex",
    "nidalee_aphelios_flex",
    "inferno_95",
    "blossom_ahri",
    "elderwood",
    "solar_akali",
    "elderwood_aphelios",
    "riftbeast_reroll",
    "yunara",
    "tristana_reroll",
    "khazix_reroll",
    "solar_kayle",
    "adaptor_reroll",
    "vanguard_aphelios",
    "unrivaled",
    "caitlyn",
    "rapidfire_aphelios",
    "defender_cassiopeia",
    "dark_ritual",
    "azir_reroll",
    "warwick_murkwolf",
    "elderwood_blossom",
    "inferno_hunter",
];

/// 基础页面：path → 合并时写入的 key（或自定义处理标记）
const BASE_PAGES: &[(&str, &str)] = &[
    ("meta", "meta"),
    ("comps", "comps"),
    ("trends", "trends"),
    ("daily-stats", "daily-stats"),
    ("units", "units"),
    ("items", "items"),
    ("item-tierlist", "item-tierlist"),
];

/// 从 HTML 提取 __NEXT_DATA__ JSON。
fn extract_next_data(html: &str) -> Result<Value, String> {
    let marker = r#"<script id="__NEXT_DATA__""#;
    let start = html
        .find(marker)
        .and_then(|i| html[i..].find('>').map(|j| i + j + 1))
        .ok_or("未找到 __NEXT_DATA__")?;
    let end = html[start..]
        .find("</script>")
        .ok_or("__NEXT_DATA__ 未闭合")?
        + start;
    serde_json::from_str(&html[start..end]).map_err(|e| format!("解析 __NEXT_DATA__ 失败: {e}"))
}

async fn fetch_page(client: &reqwest::Client, path: &str) -> Result<String, String> {
    let url = format!("https://tftable.cc{}", path);
    let resp = client
        .get(&url)
        .header("User-Agent", "TFT-Sidekick/0.1")
        .send()
        .await
        .map_err(|e| format!("请求 {} 失败: {e}", url))?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    resp.text().await.map_err(|e| e.to_string())
}

/// 抓取单页并提取 pageProps。
async fn page_props(client: &reqwest::Client, path: &str) -> Result<Value, String> {
    let html = fetch_page(client, path).await?;
    let nd = extract_next_data(&html)?;
    Ok(nd.pointer("/props/pageProps")
        .cloned()
        .ok_or_else(|| format!("{} 缺少 pageProps", path.trim_start_matches('/')))?)
}

/// 并发抓取一组页面（限流 `concurrency` 个在途），返回 (页面名, 结果)。
/// 单页失败不中断整批。36 个页面串行需要 1~2 分钟，并发后大幅缩短。
async fn fetch_all(
    client: &reqwest::Client,
    pages: Vec<String>,
    concurrency: usize,
) -> Vec<(String, Result<Value, String>)> {
    let mut set = tokio::task::JoinSet::new();
    let mut it = pages.into_iter();
    let concurrency = concurrency.max(1);

    // 先灌满在途任务
    for _ in 0..concurrency {
        if let Some(p) = it.next() {
            set.spawn(fetch_one(client.clone(), p));
        }
    }
    let mut out = Vec::new();
    while let Some(res) = set.join_next().await {
        if let Ok(item) = res {
            out.push(item);
            // 每完成一个，补一个新任务，维持并发度
            if let Some(p) = it.next() {
                set.spawn(fetch_one(client.clone(), p));
            }
        }
    }
    out
}

async fn fetch_one(
    client: reqwest::Client,
    page: String,
) -> (String, Result<Value, String>) {
    let name = page.trim_start_matches('/').to_string();
    let r = page_props(&client, &page).await;
    (name, r)
}

/// 抓取并合并全部数据，返回新的 tftable_data.json。
pub async fn update_tftable_data() -> Result<Value, String> {
    // 整体超时 + 单请求超时，避免 tftable.cc 挂起时永久阻塞更新按钮。
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let mut merged: Map<String, Value> = Map::new();
    let mut meta_pp: Option<Value> = None;

    // ---- 基础页（7 个）并发抓取 ----
    let base_paths: Vec<String> = BASE_PAGES.iter().map(|(p, _)| format!("/{}", p)).collect();
    let base_results = fetch_all(&client, base_paths, 4).await;
    let mut by_name: HashMap<String, Value> = HashMap::new();
    for (name, res) in base_results {
        match res {
            Ok(pp) => {
                by_name.insert(name, pp);
            }
            Err(e) => {
                // 基础页缺失会导致对应前端模块整体不可用，记为致命错误。
                return Err(format!("抓取 {name} 失败: {e}"));
            }
        }
    }

    for (page, kind) in BASE_PAGES {
        let Some(pp) = by_name.remove(*page) else {
            continue;
        };
        match *kind {
            "meta" => {
                let ms = pp
                    .get("metaStats")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                merged.insert("metaStats".into(), ms);
                merged.insert(
                    "compositionCards".into(),
                    pp.get("compositionCards")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                );
                merged.insert(
                    "compStrengthTrends".into(),
                    pp.get("compStrengthTrends")
                        .cloned()
                        .unwrap_or_else(|| json!({})),
                );
                meta_pp = Some(pp);
            }
            "comps" => {
                let view = pp.get("view").cloned().unwrap_or_else(|| json!({}));
                merged.insert(
                    "compAtlas".into(),
                    json!({
                        "compositionSummaries": view.get("compositionSummaries").cloned().unwrap_or_else(|| json!([])),
                        "compStats": pp.get("compStats").cloned().unwrap_or_else(|| json!({})),
                    }),
                );
            }
            "trends" => {
                let view = pp.get("view").cloned().unwrap_or_else(|| json!({}));
                merged.insert("trends".into(), view);
            }
            "daily-stats" => {
                let view = pp.get("view").cloned().unwrap_or_else(|| json!({}));
                merged.insert("dailyStats".into(), view);
            }
            "units" => {
                let view = pp.get("view").cloned().unwrap_or_else(|| json!({}));
                merged.insert(
                    "unitItems".into(),
                    view.get("priorityMatrixRows")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                );
            }
            "items" => {
                let view = pp.get("view").cloned().unwrap_or_else(|| json!({}));
                merged.insert(
                    "itemGuide".into(),
                    json!({
                        "items": view.get("items").cloned().unwrap_or_else(|| json!([])),
                        "artifacts": view.get("artifacts").cloned().unwrap_or_else(|| json!([])),
                        "emblems": view.get("emblems").cloned().unwrap_or_else(|| json!([])),
                    }),
                );
            }
            "item-tierlist" => {
                merged.insert(
                    "itemTierlist".into(),
                    pp.get("dataset").cloned().unwrap_or_else(|| json!({})),
                );
            }
            _ => {}
        }
    }

    // ---- 阵容详情页并发抓取（单页失败只跳过，不影响其他阵容） ----
    let comp_paths: Vec<String> = COMP_SLUGS.iter().map(|s| format!("/{}", s)).collect();
    let comp_results = fetch_all(&client, comp_paths, 6).await;
    let mut details = Map::new();
    let mut failed = Vec::new();
    for (slug, res) in comp_results {
        match res {
            Ok(pp) => {
                details.insert(
                    slug,
                    json!({
                        "composition": pp.get("composition").cloned().unwrap_or_else(|| json!({})),
                        "itemCategories": pp.get("itemCategories").cloned().unwrap_or_else(|| json!({})),
                        "unitCategories": pp.get("unitCategories").cloned().unwrap_or_else(|| json!({})),
                    }),
                );
            }
            Err(e) => failed.push(format!("{slug}: {e}")),
        }
    }
    if !failed.is_empty() {
        eprintln!(
            "[tft-sidekick] {} 个阵容详情页抓取失败: {}",
            failed.len(),
            failed.join("; ")
        );
    }
    if details.is_empty() {
        return Err("所有阵容详情页都抓取失败，已中止更新（避免写入残缺数据）".into());
    }
    merged.insert("compDetails".into(), Value::Object(details));

    // 补丁配置表（商店概率/规则/节奏）不来自 tftable.cc，跨更新沿用旧文件内容，
    // 旧文件缺失时回落到内嵌默认 patch_tables.json。
    let pt = serde_json::from_str::<Value>(include_str!("../../tftable_data.json"))
        .ok()
        .and_then(|o| o.get("patchTables").cloned())
        .filter(|v| !v.is_null())
        .or_else(|| serde_json::from_str(include_str!("../patch_tables.json")).ok())
        .unwrap_or_else(|| json!({}));
    merged.insert("patchTables".into(), pt);

    // 顶层元信息
    if let Some(pp) = &meta_pp {
        let ms = pp.get("metaStats").cloned().unwrap_or_else(|| json!({}));
        merged.insert(
            "patch".into(),
            ms.pointer("/metadata/patch").cloned().unwrap_or_else(|| json!("")),
        );
        merged.insert(
            "ranked_tiers".into(),
            ms.pointer("/metadata/ranked_tiers")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        merged.insert(
            "sampleSize".into(),
            ms.pointer("/metadata/total_sample_size")
                .cloned()
                .unwrap_or_else(|| json!(0)),
        );
        merged.insert(
            "timestamp".into(),
            ms.pointer("/metadata/timestamp")
                .cloned()
                .unwrap_or_else(|| json!("")),
        );
    }

    Ok(Value::Object(merged))
}

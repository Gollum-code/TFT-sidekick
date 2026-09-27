//! 在线更新 tftable_data.json：从 tftable.cc 抓取所有页面并合并重建。
//! 数据来源是各页面 HTML 内嵌的 __NEXT_DATA__ JSON。

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

/// 抓取并合并全部数据，返回新的 tftable_data.json。
pub async fn update_tftable_data() -> Result<Value, String> {
    let client = reqwest::Client::new();
    let mut merged: Map<String, Value> = Map::new();
    let mut meta_pp: Option<Value> = None;

    for &(page, kind) in BASE_PAGES {
        let html = fetch_page(&client, &format!("/{}", page)).await?;
        let nd = extract_next_data(&html)?;
        let pp = nd
            .pointer("/props/pageProps")
            .ok_or_else(|| format!("{} 缺少 pageProps", page))?
            .clone();

        match kind {
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

    // 阵容详情页（24 个）
    let mut details = Map::new();
    for slug in COMP_SLUGS {
        let html = fetch_page(&client, &format!("/{}", slug)).await?;
        let nd = extract_next_data(&html)?;
        let pp = nd
            .pointer("/props/pageProps")
            .ok_or_else(|| format!("{} 缺少 pageProps", slug))?
            .clone();
        details.insert(
            slug.to_string(),
            json!({
                "composition": pp.get("composition").cloned().unwrap_or_else(|| json!({})),
                "itemCategories": pp.get("itemCategories").cloned().unwrap_or_else(|| json!({})),
                "unitCategories": pp.get("unitCategories").cloned().unwrap_or_else(|| json!({})),
            }),
        );
    }
    merged.insert("compDetails".into(), Value::Object(details));

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

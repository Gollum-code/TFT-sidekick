//! Meta 数据抓取工具。
//!
//! 用法: RIOT_API_KEY=xxx cargo run --bin crawl_meta [-- meta.json]
//!
//! 抓取高分段对局，聚合阵容/装备榜，生成 meta.json（提交到项目中）。

use std::path::PathBuf;

use tft_sidekick_lib::models::{
    CompDetail, HeroItem, HeroItemRow, MetaComp, MetaItem,
};
use tft_sidekick_lib::riot::{
    aggregate_comps, aggregate_items, aggregate_unit_items, crawl_matches, RiotClient,
};
use tft_sidekick_lib::static_data::StaticData;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("RIOT_API_KEY")
        .expect("请设置环境变量 RIOT_API_KEY（Riot 开发 key）");

    let out: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("meta.json"));

    let max_matches: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    // 1. 加载 CDragon 静态数据（用于中文名映射）
    let cache_dir = std::env::temp_dir().join("tft_sidekick_cache");
    std::fs::create_dir_all(&cache_dir).ok();
    let cdragon_path = cache_dir.join("cdragon_tft_zhcn.json");
    if !cdragon_path.exists() {
        eprintln!("[1/4] 下载 CDragon 数据...");
        StaticData::download(&cdragon_path).await?;
    }
    eprintln!("[1/4] 静态数据已就绪");
    let sd = StaticData::load(&cdragon_path)?;

    // 2. 抓取高分段对局
    eprintln!("[2/4] 开始抓取 {max_matches} 局高分段对局...");
    let client = RiotClient::new(key, "na1");
    let matches = crawl_matches(&client, max_matches).await?;
    eprintln!("[2/4] 成功抓取 {} 局", matches.len());

    // 3. 聚合
    eprintln!("[3/4] 计算阵容/装备榜...");
    let comp_stats = aggregate_comps(&matches);
    let item_stats = aggregate_items(&matches);

    let total_matches = matches.len() as f64;
    let mut comp_out: Vec<MetaComp> = comp_stats
        .into_iter()
        .filter(|s| s.appearances >= 2)
        .map(|s| {
            let id = s.id.clone();
            MetaComp {
                name: sd
                    .trait_name(&id)
                    .map(String::from)
                    .unwrap_or_else(|| id.clone()),
                icon_url: sd.trait_icon(&id).unwrap_or_default(),
                id,
                appearances: s.appearances,
                win_rate: s.win_rate(),
                top4_rate: s.top4_rate(),
                // 登顶占比 = 吃鸡局 / 总对局；前四占比 = 前四局 / (总对局×4)
                win_share: s.wins as f64 / total_matches,
                top4_share: s.top4 as f64 / (total_matches * 4.0),
            }
        })
        .collect();
    comp_out.sort_by(|a, b| {
        b.top4_share
            .partial_cmp(&a.top4_share)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut item_out: Vec<MetaItem> = item_stats
        .into_iter()
        .filter(|s| s.appearances >= 3)
        .map(|s| {
            let id = s.id.clone();
            MetaItem {
                name: sd
                    .item_name(&id)
                    .map(String::from)
                    .unwrap_or_else(|| id.clone()),
                icon_url: sd.item_icon(&id).unwrap_or_default(),
                id,
                appearances: s.appearances,
                win_rate: s.win_rate(),
                top4_rate: s.top4_rate(),
            }
        })
        .collect();
    item_out.sort_by(|a, b| {
        b.top4_rate
            .partial_cmp(&a.top4_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 3.5 英雄最佳装备：(英雄, 装备) 聚合 → 按英雄分组
    let hero_items_stats = aggregate_unit_items(&matches);
    let mut hero_map: std::collections::HashMap<String, HeroItemRow> =
        std::collections::HashMap::new();
    for s in hero_items_stats {
        if s.appearances < 3 {
            continue;
        }
        let item_name = sd.item_name(&s.item).map(String::from).unwrap_or_default();
        let hero = hero_map
            .entry(s.hero.clone())
            .or_insert_with(|| HeroItemRow {
                id: s.hero.clone(),
                name: sd
                    .champion_name(&s.hero)
                    .map(String::from)
                    .unwrap_or_default(),
                icon_url: sd.champion_icon(&s.hero).unwrap_or_default(),
                items: Vec::new(),
            });
        hero.items.push(HeroItem {
            id: s.item.clone(),
            name: item_name,
            icon_url: sd.item_icon(&s.item).unwrap_or_default(),
            appearances: s.appearances,
            win_rate: (s.wins as f64) / (s.appearances as f64),
            top4_rate: (s.top4 as f64) / (s.appearances as f64),
        });
    }
    let mut hero_out: Vec<HeroItemRow> = hero_map.into_values().collect();
    for h in hero_out.iter_mut() {
        h.items.sort_by(|a, b| b.appearances.cmp(&a.appearances));
        h.items.truncate(5);
    }
    // 按英雄装备的总出现次数降序（热门英雄排前）
    hero_out.sort_by(|a, b| {
        let total_a: u32 = a.items.iter().map(|i| i.appearances).sum();
        let total_b: u32 = b.items.iter().map(|i| i.appearances).sum();
        total_b.cmp(&total_a)
    });

    // 3.5 阵容详情：按主羁绊分组的构成与统计
    let comp_details: Vec<CompDetail> =
        tft_sidekick_lib::riot::aggregate_comp_details(&matches, &sd);

    // 4. 写文件
    eprintln!("[4/4] 写入 {} ...", out.display());
    let output = serde_json::json!({
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "sample_count": matches.len(),
        "region": "na1",
        "tiers": ["challenger", "grandmaster", "master"],
        "comps": comp_out,
        "items": item_out,
        "hero_items": hero_out,
        "comp_details": comp_details,
    });
    std::fs::write(&out, serde_json::to_string_pretty(&output)?)?;
    eprintln!("完成！已写入 {}", out.display());
    Ok(())
}

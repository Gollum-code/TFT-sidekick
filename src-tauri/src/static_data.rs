//! TFT 静态数据（CDragon datamine）。
//!
//! 数据源：`https://raw.communitydragon.org/latest/cdragon/tft/zh_cn.json`
//! 顶层为 `items`（全赛季物品）+ `sets`（按赛季组织的 champions/traits）。
//! `apiName` 全局唯一，与 LCU 对局里的 `character_id` / `itemNames` / 羁绊 `name` 直接对应。
//!
//! 图标：CDragon 数据里的 `.tex` 路径需换成 `.png`，
//! 前缀 `https://raw.communitydragon.org/latest/game/`。

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use thiserror::Error;

use crate::models::{
    ChampionData, EnrichedItem, EnrichedMatch, EnrichedTrait, EnrichedUnit, ItemData, MatchDetail,
    TraitData, TraitTierData, TraitUnit,
};

const CDRAGON_URL: &str = "https://raw.communitydragon.org/latest/cdragon/tft/zh_cn.json";
const ICON_BASE: &str = "https://raw.communitydragon.org/latest/game/";

#[derive(Debug, Error)]
pub enum StaticDataError {
    #[error("下载静态数据失败: {0}")]
    Download(String),
    #[error("解析静态数据失败: {0}")]
    Parse(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
struct ChampionInfo {
    name: String,
    tile_icon: Option<String>,
    cost: Option<i32>,
    hp: f64,
    damage: f64,
    armor: f64,
    magic_resist: f64,
    mana: f64,
    ability_name: String,
    ability_desc: String,
    traits: Vec<String>,
}

#[derive(Debug, Clone)]
struct ItemInfo {
    name: String,
    icon: Option<String>,
    composition: Vec<String>,
    desc: String,
    is_augment: bool,
}

#[derive(Debug, Clone)]
struct TraitInfo {
    name: String,
    icon: Option<String>,
    desc: String,
    tiers: Vec<TraitTierData>,
}

#[derive(Debug, Clone, Default)]
pub struct StaticData {
    champions: HashMap<String, ChampionInfo>,
    items: HashMap<String, ItemInfo>,
    traits: HashMap<String, TraitInfo>,
    set_code: String,
    current_champions: Vec<String>,
    current_traits: Vec<String>,
}

#[derive(serde::Deserialize)]
struct CdragonRoot {
    #[serde(default)]
    items: Vec<CdragonItem>,
    #[serde(default)]
    sets: HashMap<String, CdragonSet>,
}

#[derive(serde::Deserialize)]
struct CdragonSet {
    #[serde(default)]
    champions: Vec<CdragonChampion>,
    #[serde(default)]
    traits: Vec<CdragonTrait>,
}

#[derive(serde::Deserialize)]
struct CdragonItem {
    #[serde(rename = "apiName")]
    api_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    composition: Vec<String>,
    #[serde(default)]
    desc: Option<String>,
    #[serde(default, rename = "isAugment")]
    is_augment: bool,
}

#[derive(serde::Deserialize)]
struct CdragonChampion {
    #[serde(rename = "apiName")]
    api_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default, rename = "tileIcon")]
    tile_icon: Option<String>,
    #[serde(default)]
    cost: Option<i32>,
    #[serde(default)]
    stats: Option<CdragonStats>,
    #[serde(default)]
    ability: Option<CdragonAbility>,
    #[serde(default)]
    traits: Vec<String>,
}

#[derive(serde::Deserialize)]
struct CdragonStats {
    #[serde(default)]
    hp: Option<f64>,
    #[serde(default)]
    damage: Option<f64>,
    #[serde(default)]
    armor: Option<f64>,
    #[serde(default, rename = "magicResist")]
    magic_resist: Option<f64>,
    #[serde(default)]
    mana: Option<f64>,
}

#[derive(serde::Deserialize)]
struct CdragonAbility {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    desc: Option<String>,
}

#[derive(serde::Deserialize)]
struct CdragonTrait {
    #[serde(rename = "apiName")]
    api_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    desc: Option<String>,
    #[serde(default)]
    effects: Vec<CdragonTraitEffect>,
}

#[derive(serde::Deserialize)]
struct CdragonTraitEffect {
    #[serde(default, rename = "minUnits")]
    min_units: Option<i32>,
    #[serde(default, rename = "maxUnits")]
    max_units: Option<i32>,
    #[serde(default)]
    style: Option<i32>,
}

impl StaticData {
    pub fn parse(bytes: &[u8]) -> Result<Self, StaticDataError> {
        let root: CdragonRoot =
            serde_json::from_slice(bytes).map_err(|e| StaticDataError::Parse(e.to_string()))?;

        let mut sd = StaticData::default();

        for item in root.items {
            if let Some(name) = item.name {
                sd.items.insert(
                    item.api_name,
                    ItemInfo {
                        name,
                        icon: item.icon,
                        composition: item.composition,
                        desc: Self::clean_desc(item.desc.as_deref().unwrap_or("")),
                        is_augment: item.is_augment,
                    },
                );
            }
        }

        // 确定当前赛季（数字 key 最大）
        let current_set = root
            .sets
            .keys()
            .filter_map(|k| k.parse::<u32>().ok())
            .max()
            .map(|n| n.to_string());

        if let Some(set_key) = &current_set {
            if let Some(set) = root.sets.get(set_key) {
                for c in set.champions.iter() {
                    sd.champions.insert(
                        c.api_name.clone(),
                        ChampionInfo {
                            name: c.name.clone().unwrap_or_default(),
                            tile_icon: c.tile_icon.clone(),
                            cost: c.cost,
                            hp: c.stats.as_ref().and_then(|s| s.hp).unwrap_or(0.0),
                            damage: c.stats.as_ref().and_then(|s| s.damage).unwrap_or(0.0),
                            armor: c.stats.as_ref().and_then(|s| s.armor).unwrap_or(0.0),
                            magic_resist: c
                                .stats
                                .as_ref()
                                .and_then(|s| s.magic_resist)
                                .unwrap_or(0.0),
                            mana: c.stats.as_ref().and_then(|s| s.mana).unwrap_or(0.0),
                            ability_name: c
                                .ability
                                .as_ref()
                                .and_then(|a| a.name.clone())
                                .unwrap_or_default(),
                            ability_desc: Self::clean_desc(
                                c.ability
                                    .as_ref()
                                    .and_then(|a| a.desc.as_deref())
                                    .unwrap_or(""),
                            ),
                            traits: c.traits.clone(),
                        },
                    );
                }
                sd.current_champions = set
                    .champions
                    .iter()
                    .map(|c| c.api_name.clone())
                    .collect();
                for t in set.traits.iter() {
                    if let Some(name) = t.name.clone() {
                        sd.traits.insert(
                            t.api_name.clone(),
                            TraitInfo {
                                name,
                                icon: t.icon.clone(),
                                desc: Self::clean_desc(t.desc.as_deref().unwrap_or("")),
                                tiers: t
                                    .effects
                                    .iter()
                                    .map(|e| TraitTierData {
                                        min_units: e.min_units.unwrap_or(0),
                                        max_units: e.max_units.unwrap_or(0),
                                        style: e.style.unwrap_or(0),
                                    })
                                    .collect(),
                            },
                        );
                    }
                }
                sd.current_traits = set.traits.iter().map(|t| t.api_name.clone()).collect();
            }
        }

        // 当前赛季的物品前缀（如 "DA"）。跳过全赛季共用的野怪（"TFT" 前缀）。
        sd.set_code = sd
            .current_champions
            .iter()
            .map(|name| name.split('_').next().unwrap_or("").to_string())
            .find(|s| !s.is_empty() && !s.starts_with("TFT"))
            .unwrap_or_default();

        Ok(sd)
    }

    pub fn load(path: &Path) -> Result<Self, StaticDataError> {
        let bytes = std::fs::read(path)?;
        Self::parse(&bytes)
    }

    pub async fn download(path: &Path) -> Result<(), StaticDataError> {
        let bytes = reqwest::get(CDRAGON_URL)
            .await
            .map_err(|e| StaticDataError::Download(e.to_string()))?
            .bytes()
            .await
            .map_err(|e| StaticDataError::Download(e.to_string()))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &bytes)?;
        Ok(())
    }

    fn icon_url(path: &str) -> String {
        format!("{}{}", ICON_BASE, path.replace(".tex", ".png"))
    }

    /// 清洗 CDragon 描述文案：`@变量@` 占位符、`%i:scaleX%` 缩放标记、
    /// HTML 标签（`<br>`/`<keyword>`/`<Rules>` 等）和 `\n` 转义 → 可读中文。
    fn clean_desc(raw: &str) -> String {
        if raw.is_empty() {
            return String::new();
        }
        let mut s = raw.to_string();

        let markers = [
            ("%i:scaleAP%", "法术强度"),
            ("%i:scaleAD%", "攻击力"),
            ("%i:scaleHealth%", "生命值"),
            ("%i:scaleAS%", "攻速"),
            ("%i:scaleDA%", "伤害增幅"),
            ("%i:scaleArmor%", "护甲"),
            ("%i:scaleMR%", "魔抗"),
            ("%i:scaleDR%", "伤害减免"),
            ("%i:scaleCrit%", "暴击率"),
            ("%i:scaleCritMult%", "暴击倍率"),
            ("%i:scaleSouls%", "灵魂"),
            ("%i:scaleSV%", "法术吸血"),
            ("%i:goldCoins%", "金币"),
            ("%i:TFTBaseAD%", "基础攻击力"),
            ("%i:scaleMana%", "法力值"),
            ("%i:TFTManaRegen%", "法力回复"),
            ("%i:scaleSerpents%", "巨蛇"),
            ("%i:scaleRange%", "攻击距离"),
            ("%i:scaleManaRegen%", "法力回复"),
            ("%i:scaleHPRegen%", "生命回复"),
            ("%i:star%", "星"),
        ];
        for (m, r) in markers {
            s = s.replace(m, r);
        }

        // 移除 @占位符@（保留偶数段）
        let parts: Vec<&str> = s.split('@').collect();
        let mut out = String::new();
        for (i, part) in parts.iter().enumerate() {
            if i % 2 == 0 {
                out.push_str(part);
            }
        }
        s = out;

        // <br> -> 换行
        s = s.replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n");

        // 移除其余 HTML 标签
        if let Ok(tag_re) = regex::Regex::new(r"</?[a-zA-Z]+>") {
            s = tag_re.replace_all(&s, "").to_string();
        }

        // 转义换行
        s = s
            .replace("\\r\\n", "\n")
            .replace("\\n", "\n")
            .replace("\\r", "");

        // 清理空白
        if let Ok(ws_re) = regex::Regex::new(r"[ \t]+") {
            s = ws_re.replace_all(&s, " ").to_string();
        }
        if let Ok(nl_re) = regex::Regex::new(r"\n{2,}") {
            s = nl_re.replace_all(&s, "\n").to_string();
        }

        // 移除空括号
        s = s.replace("()", "").replace("（）", "").replace("( )", "");

        // 移除孤立百分号（前面不是数字）
        let mut cleaned = String::with_capacity(s.len());
        let chars: Vec<char> = s.chars().collect();
        for i in 0..chars.len() {
            if chars[i] == '%' && (i == 0 || !chars[i - 1].is_ascii_digit()) {
                continue;
            }
            cleaned.push(chars[i]);
        }
        s = cleaned;

        // 再次清理空白
        if let Ok(ws_re2) = regex::Regex::new(r"[ \t]+") {
            s = ws_re2.replace_all(&s, " ").to_string();
        }
        s.trim().to_string()
    }

    pub fn champion_name(&self, id: &str) -> Option<&str> {
        self.champions.get(id).map(|c| c.name.as_str())
    }

    pub fn champion_icon(&self, id: &str) -> Option<String> {
        self.champions
            .get(id)
            .and_then(|c| c.tile_icon.as_deref())
            .map(Self::icon_url)
    }

    pub fn item_name(&self, id: &str) -> Option<&str> {
        self.items.get(id).map(|i| i.name.as_str())
    }

    pub fn item_icon(&self, id: &str) -> Option<String> {
        self.items
            .get(id)
            .and_then(|i| i.icon.as_deref())
            .map(Self::icon_url)
    }

    pub fn trait_name(&self, id: &str) -> Option<&str> {
        self.traits.get(id).map(|t| t.name.as_str())
    }

    pub fn trait_icon(&self, id: &str) -> Option<String> {
        self.traits
            .get(id)
            .and_then(|t| t.icon.as_deref())
            .map(Self::icon_url)
    }

    pub fn enrich(&self, detail: MatchDetail) -> EnrichedMatch {
        let units = detail
            .units
            .into_iter()
            .map(|u| {
                let champion = self.champions.get(&u.character_id);
                let items = u
                    .items
                    .into_iter()
                    .map(|id| match self.items.get(&id) {
                        Some(info) => EnrichedItem {
                            id,
                            name: info.name.clone(),
                            icon_url: info
                                .icon
                                .as_deref()
                                .map(Self::icon_url)
                                .unwrap_or_default(),
                        },
                        None => EnrichedItem {
                            id,
                            name: String::new(),
                            icon_url: String::new(),
                        },
                    })
                    .collect();
                EnrichedUnit {
                    character_id: u.character_id,
                    name: champion.map(|c| c.name.clone()).unwrap_or_default(),
                    icon_url: champion
                        .and_then(|c| c.tile_icon.as_deref())
                        .map(Self::icon_url)
                        .unwrap_or_default(),
                    cost: champion.and_then(|c| c.cost),
                    tier: u.tier,
                    items,
                }
            })
            .collect();

        let traits = detail
            .traits
            .into_iter()
            .map(|t| {
                let info = self.traits.get(&t.trait_id);
                EnrichedTrait {
                    id: t.trait_id,
                    name: info.map(|i| i.name.clone()).unwrap_or_default(),
                    icon_url: info
                        .and_then(|i| i.icon.as_deref())
                        .map(Self::icon_url)
                        .unwrap_or_default(),
                    tier_current: t.tier_current,
                    tier_total: t.tier_total,
                }
            })
            .collect();

        EnrichedMatch {
            summary: detail.summary,
            units,
            traits,
        }
    }

    // ---- M4：隐藏数据表查询 ----

    /// 当前赛季的装备：可合成成品（composition 非空）+ 散件（含 "Component"），
    /// 排除海克斯/升级等噪音。
    pub fn items(&self) -> Vec<ItemData> {
        let prefix = format!("{}_", self.set_code);
        let mut out: Vec<ItemData> = self
            .items
            .iter()
            .filter(|(id, _)| id.starts_with(&prefix))
            .filter(|(id, info)| !info.composition.is_empty() || id.contains("Component"))
            .map(|(id, info)| ItemData {
                id: id.clone(),
                name: info.name.clone(),
                icon_url: info.icon.as_deref().map(Self::icon_url).unwrap_or_default(),
                composition: info.composition.clone(),
                desc: info.desc.clone(),
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// 当前赛季海克斯（isAugment + 当前赛季前缀）。
    pub fn augments(&self) -> Vec<ItemData> {
        let prefix = format!("{}_", self.set_code);
        let mut out: Vec<ItemData> = self
            .items
            .iter()
            .filter(|(id, info)| id.starts_with(&prefix) && info.is_augment)
            .map(|(id, info)| ItemData {
                id: id.clone(),
                name: info.name.clone(),
                icon_url: info.icon.as_deref().map(Self::icon_url).unwrap_or_default(),
                composition: Vec::new(),
                desc: info.desc.clone(),
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// 当前赛季全部英雄。
    pub fn champions(&self) -> Vec<ChampionData> {
        let mut out: Vec<ChampionData> = self
            .current_champions
            .iter()
            .filter_map(|id| {
                self.champions.get(id).map(|c| ChampionData {
                    id: id.clone(),
                    name: c.name.clone(),
                    icon_url: c
                        .tile_icon
                        .as_deref()
                        .map(Self::icon_url)
                        .unwrap_or_default(),
                    cost: c.cost.unwrap_or(0),
                    hp: c.hp,
                    damage: c.damage,
                    armor: c.armor,
                    magic_resist: c.magic_resist,
                    mana: c.mana,
                    ability_name: c.ability_name.clone(),
                    ability_desc: c.ability_desc.clone(),
                    traits: c.traits.clone(),
                })
            })
            .collect();
        out.sort_by(|a, b| (a.cost, &a.name).cmp(&(b.cost, &b.name)));
        out
    }

    /// 当前赛季全部羁绊（含成员英雄）。
    pub fn traits(&self) -> Vec<TraitData> {
        // 构建 羁绊显示名 -> 成员英雄 的映射
        let mut units_by_name: std::collections::HashMap<&str, Vec<TraitUnit>> =
            std::collections::HashMap::new();
        for cid in &self.current_champions {
            if let Some(c) = self.champions.get(cid) {
                let unit = TraitUnit {
                    id: cid.clone(),
                    name: c.name.clone(),
                    icon_url: c
                        .tile_icon
                        .as_deref()
                        .map(Self::icon_url)
                        .unwrap_or_default(),
                };
                for tn in &c.traits {
                    units_by_name.entry(tn.as_str()).or_default().push(unit.clone());
                }
            }
        }

        let mut out: Vec<TraitData> = self
            .current_traits
            .iter()
            .filter_map(|id| {
                self.traits.get(id).map(|t| TraitData {
                    id: id.clone(),
                    name: t.name.clone(),
                    icon_url: t.icon.as_deref().map(Self::icon_url).unwrap_or_default(),
                    desc: t.desc.clone(),
                    tiers: t.tiers.clone(),
                    units: units_by_name
                        .get(t.name.as_str())
                        .cloned()
                        .unwrap_or_default(),
                })
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MatchDetail, MatchSummary, MatchTrait, MatchUnit};

    #[test]
    #[ignore = "需要先下载 cdragon_tft_zhcn.json 到临时目录"]
    fn parse_and_enrich_from_temp() {
        let path = std::env::temp_dir().join("cdragon_tft_zhcn.json");
        if !path.exists() {
            eprintln!("skip: {} not found", path.display());
            return;
        }
        let sd = StaticData::load(&path).expect("load failed");

        let items = sd.items();
        let champs = sd.champions();
        let traits = sd.traits();
        println!(
            "items={} champs={} traits={} set_code={}",
            items.len(),
            champs.len(),
            traits.len(),
            sd.set_code
        );
        assert!(items.iter().any(|i| !i.composition.is_empty()));
        assert!(champs.iter().any(|c| c.cost > 0 && c.hp > 0.0));
        assert!(traits.iter().any(|t| !t.tiers.is_empty()));
        // 验证羁绊→英雄映射有效
        let brawler = traits.iter().find(|t| t.name == "斗士");
        if let Some(b) = brawler {
            println!("斗士 units: {}", b.units.len());
            for u in &b.units {
                println!("  {} {}", u.name, u.id);
            }
            assert!(!b.units.is_empty(), "斗士应有英雄成员");
        }

        if let Some(it) = items.iter().find(|i| !i.composition.is_empty()) {
            println!("sample item: {} <- {:?}", it.name, it.composition);
        }
        if let Some(c) = champs.iter().find(|c| c.cost > 0) {
            println!(
                "sample champ: {} cost={} hp={} ability={} traits={:?}",
                c.name, c.cost, c.hp, c.ability_name, c.traits
            );
            println!("  ability_desc: {}", c.ability_desc);
        }
        if let Some(t) = traits.iter().find(|t| !t.tiers.is_empty()) {
            println!(
                "sample trait: {} tiers={:?}",
                t.name,
                t.tiers.iter().map(|x| x.min_units).collect::<Vec<_>>()
            );
            println!("  trait_desc: {}", t.desc);
        }

        let detail = MatchDetail {
            summary: MatchSummary {
                game_id: "1".into(),
                game_mode: Some("TFTSet18".into()),
                game_version: None,
                game_datetime: "2026-09-08T00:00:00Z".into(),
                placement: 1,
                level: Some(9),
                augments: vec![],
            },
            units: vec![MatchUnit {
                character_id: "DA_18_Xayah".into(),
                name: None,
                rarity: Some(0),
                tier: Some(3),
                items: vec!["DA_StrikersFlail".into(), "DA_InfinityEdge".into()],
                position_x: None,
                position_y: None,
            }],
            traits: vec![MatchTrait {
                trait_id: "DA_18_Brawler".into(),
                name: None,
                tier_current: Some(2),
                tier_total: Some(3),
            }],
        };

        let enriched = sd.enrich(detail);
        assert!(!enriched.units[0].name.is_empty());
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchSummary {
    pub game_id: String,
    pub game_mode: Option<String>,
    pub game_version: Option<String>,
    pub game_datetime: String,
    pub placement: i32,
    pub level: Option<i32>,
    pub augments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchUnit {
    pub character_id: String,
    pub name: Option<String>,
    pub rarity: Option<i32>,
    pub tier: Option<i32>,
    pub items: Vec<String>,
    pub position_x: Option<i32>,
    pub position_y: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchTrait {
    pub trait_id: String,
    pub name: Option<String>,
    pub tier_current: Option<i32>,
    pub tier_total: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchDetail {
    pub summary: MatchSummary,
    pub units: Vec<MatchUnit>,
    pub traits: Vec<MatchTrait>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub db_path: String,
    pub lcu_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummonerInfo {
    pub puuid: String,
    pub game_name: Option<String>,
    pub tag_line: Option<String>,
    pub summoner_level: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub count: u32,
    pub source: String,
    pub summoner: Option<SummonerInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnrichedItem {
    pub id: String,
    pub name: String,
    pub icon_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnrichedUnit {
    pub character_id: String,
    pub name: String,
    pub icon_url: String,
    pub cost: Option<i32>,
    pub tier: Option<i32>,
    pub items: Vec<EnrichedItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnrichedTrait {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    pub tier_current: Option<i32>,
    pub tier_total: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnrichedMatch {
    pub summary: MatchSummary,
    pub units: Vec<EnrichedUnit>,
    pub traits: Vec<EnrichedTrait>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NamedCount {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    pub total: u32,
    pub win_rate: f64,
    pub top4_rate: f64,
    pub avg_placement: f64,
    pub avg_level: f64,
    pub placement_dist: Vec<u32>,
    pub top_traits: Vec<NamedCount>,
    pub top_units: Vec<NamedCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemData {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    pub composition: Vec<String>,
    pub desc: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChampionData {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    pub cost: i32,
    pub hp: f64,
    pub damage: f64,
    pub armor: f64,
    pub magic_resist: f64,
    pub mana: f64,
    pub ability_name: String,
    pub ability_desc: String,
    pub traits: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitTierData {
    pub min_units: i32,
    pub max_units: i32,
    pub style: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitUnit {
    pub id: String,
    pub name: String,
    pub icon_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitData {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    pub desc: String,
    pub tiers: Vec<TraitTierData>,
    pub units: Vec<TraitUnit>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShopOddsLevel {
    pub level: i32,
    pub odds: Vec<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RulesSection {
    pub title: String,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantComp {
    pub placement: i32,
    pub win: bool,
    pub level: Option<i32>,
    pub traits: Vec<MatchTrait>,
    pub units: Vec<MatchUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiotMatch {
    pub match_id: String,
    pub queue_id: i32,
    pub tft_set_number: i32,
    pub participants: Vec<ParticipantComp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaComp {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    pub appearances: u32,
    pub win_rate: f64,
    pub top4_rate: f64,
    #[serde(default)]
    pub win_share: f64,
    #[serde(default)]
    pub top4_share: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaItem {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    pub appearances: u32,
    pub win_rate: f64,
    pub top4_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroItem {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    pub appearances: u32,
    pub win_rate: f64,
    pub top4_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroItemRow {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    pub items: Vec<HeroItem>,
}

/// 阵容详情：某主羁绊阵容的构成与统计。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompDetail {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    pub appearances: u32,
    pub win_rate: f64,
    pub top4_rate: f64,
    pub avg_placement: f64,
    pub units: Vec<CompUnit>,
    pub traits: Vec<CompTraitRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompUnit {
    pub id: String,
    pub name: String,
    pub icon_url: String,
    pub count: u32,
    pub rate: f64,
    pub avg_tier: f64,
    pub items: Vec<HeroItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompTraitRow {
    pub id: String,
    pub name: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaResult {
    pub comps: Vec<MetaComp>,
    pub items: Vec<MetaItem>,
    pub hero_items: Vec<HeroItemRow>,
    pub comp_details: Vec<CompDetail>,
    pub sample_count: u32,
    pub added_count: u32,
}

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
    pub queue_id: Option<i32>,
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
    pub position_x: Option<i32>,
    pub position_y: Option<i32>,
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
    /// 海克斯稀有度：silver / gold / prismatic / standard（普通装备为空）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopOddsLevel {
    pub level: i32,
    pub odds: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesSection {
    pub title: String,
    pub rows: Vec<Vec<String>>,
}

/// 对局中悬浮窗展示的个人信息摘要。
#[derive(Debug, Clone, Serialize, Default)]
pub struct Profile {
    pub rank_tier: String,
    pub rank_division: String,
    pub rank_lp: i32,
    pub rank_wins: i32,
    pub rank_losses: i32,
    pub win_rate: f64,
    pub top4_rate: f64,
    pub avg_placement: f64,
    pub total: u32,
}

/// 对局内即时状态（2999 Game Client API）。
#[derive(Debug, Clone, Serialize, Default)]
pub struct LiveGameState {
    pub available: bool,
    pub gold: Option<i64>,
    pub level: Option<i64>,
    pub health: Option<f64>,
    pub max_health: Option<f64>,
    pub xp: Option<f64>,
    pub stage: String,
    pub game_time: f64,
    pub players: Vec<LivePlayer>,
}

/// 对局中某名玩家的实时简要信息。
#[derive(Debug, Clone, Serialize, Default)]
pub struct LivePlayer {
    pub puuid: Option<String>,
    pub name: String,
    pub health: f64,
    pub max_health: f64,
    pub level: Option<i64>,
    pub rank: Option<i64>,
    pub is_me: bool,
}
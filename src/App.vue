<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

const isOverlay = getCurrentWindow().label === "overlay";

interface MatchSummary {
  game_id: string;
  game_mode: string | null;
  game_version: string | null;
  game_datetime: string;
  placement: number;
  level: number | null;
  augments: string[];
}

interface AppInfo {
  name: string;
  version: string;
  db_path: string;
  lcu_connected: boolean;
}

interface SummonerInfo {
  puuid: string;
  game_name: string | null;
  tag_line: string | null;
  summoner_level: number | null;
}

interface SyncResult {
  count: number;
  source: string;
  summoner: SummonerInfo | null;
}

interface EnrichedItem {
  id: string;
  name: string;
  icon_url: string;
}

interface EnrichedUnit {
  character_id: string;
  name: string;
  icon_url: string;
  cost: number | null;
  tier: number | null;
  items: EnrichedItem[];
}

interface EnrichedTrait {
  id: string;
  name: string;
  icon_url: string;
  tier_current: number | null;
  tier_total: number | null;
}

interface EnrichedMatch {
  summary: MatchSummary;
  units: EnrichedUnit[];
  traits: EnrichedTrait[];
}

interface NamedCount {
  id: string;
  name: string;
  icon_url: string;
  count: number;
}

interface Stats {
  total: number;
  win_rate: number;
  top4_rate: number;
  avg_placement: number;
  avg_level: number;
  placement_dist: number[];
  top_traits: NamedCount[];
  top_units: NamedCount[];
}

interface ItemData {
  id: string;
  name: string;
  icon_url: string;
  composition: string[];
  desc: string;
}

interface ChampionData {
  id: string;
  name: string;
  icon_url: string;
  cost: number;
  hp: number;
  damage: number;
  armor: number;
  magic_resist: number;
  mana: number;
  ability_name: string;
  ability_desc: string;
  traits: string[];
}

interface TraitTierData {
  min_units: number;
  max_units: number;
  style: number;
}

interface TraitUnit {
  id: string;
  name: string;
  icon_url: string;
}

interface TraitData {
  id: string;
  name: string;
  icon_url: string;
  desc: string;
  tiers: TraitTierData[];
  units: TraitUnit[];
}

interface ShopOddsLevel {
  level: number;
  odds: number[];
}

interface Profile {
  rank_tier: string;
  rank_division: string;
  rank_lp: number;
  rank_wins: number;
  rank_losses: number;
  win_rate: number;
  top4_rate: number;
  avg_placement: number;
  total: number;
}

interface RulesSection {
  title: string;
  rows: string[][];
}

// ===== TFTable 数据（tftable_data.json） =====

interface TFTraitShare {
  trait_id: string;
  trait_name_cn: string;
  trait_name_en: string;
  tier: number | null;
  units_required: number | null;
  wins: number;
}

interface TFCompShare {
  slug: string;
  wins: number;
}

interface TFMetaStats {
  metadata: { patch: string; ranked_tiers: string[]; total_sample_size: number; timestamp: string };
  winShare: { sample_size: number; compositions: TFCompShare[]; traits: TFTraitShare[] };
  top4Share: { sample_size: number; compositions: TFCompShare[]; traits: TFTraitShare[] };
  balance: {
    involution_index: number;
    involution_k: number;
    diversity_index: number;
    notes: { involution: string; diversity: string };
  };
  core_cost_share: { win_share: Record<string, number>; top4_share: Record<string, number> };
}

interface TFCompositionCard {
  key: string;
  slug: string;
  display_name_cn: string;
  display_name_en: string;
  compIcon: string;
  compCost: number;
  averagePlacementValue: number;
}

interface TFStrengthSeries {
  key: string;
  slug: string;
  points: { date: string; strength: number }[];
}

interface TFCompStrengthTrends {
  patch: string;
  comparisonPatch: string;
  generatedAt: string;
  dates: string[];
  series: TFStrengthSeries[];
  patchChangeIndices: number[];
  dateToPatch: Record<string, string>;
  balance_daily: Record<
    string,
    {
      involution_index: number;
      diversity_index: number;
      win_share_by_cost: Record<string, number>;
      top4_share_by_cost: Record<string, number>;
      delta_vs_previous_day: Record<string, number> | null;
    }
  >;
}

interface TFCompSummary {
  slug: string;
  avgPlacementValue: number;
  trendDelta: number;
  playRateDelta: number;
  winShareDelta: number;
}

interface TFCompStat {
  strength: number;
  playRate: number;
  strengthDelta: number;
}

interface TFRowDelta {
  text: string;
  positive: boolean | null;
  value: number | null;
}

interface TFTrendRow {
  slug: string;
  name: string;
  playRate: string;
  playRateDelta: TFRowDelta;
  avgPlacement: string;
  avgPlacementDelta: TFRowDelta;
  avgPlacementValue: number;
  versionDeltaAvg: TFRowDelta | null;
  versionDeltaPlay: TFRowDelta | null;
  versionDeltaValue: number | null;
  dateMetrics: { date: string; avgPlacement: number }[];
}

interface TFTrendItem {
  slug: string;
  name: string;
  playRate: string;
  playRateDelta: TFRowDelta;
  winShare: string | null;
  winShareDelta: TFRowDelta | null;
  top4Share: string | null;
  top4ShareDelta: TFRowDelta | null;
  avgPlacement: string;
  avgPlacementDelta: TFRowDelta;
}

interface TFUnitItem {
  unitId: string;
  unitCost: number;
  unitPriority: number | null;
  buckets: {
    Core: { itemId: string; necessity: number; priorityScore: number | null }[];
    Important: { itemId: string; necessity: number; priorityScore: number | null }[];
    Optional: { itemId: string; necessity: number; priorityScore: number | null }[];
    HighFrequency: { itemId: string; necessity: number; priorityScore: number | null }[];
  };
  bisItemIds: string[];
  artifacts: { itemId: string; avgPlacement: number }[];
  coreCompStrengths: { comp_key: string }[];
}

interface TFItemGuideEntry {
  itemId: string;
  itemName: string;
  costBuckets: Record<string, { unitId: string; normalized: number; necessity: number }[]>;
  compositions: { slug: string; name: string; unit_id: string }[];
  type: string;
}

interface TFTierlistComp {
  comp_id: string;
  item_strength: number;
  top_carrier_unit_id: string;
  top_pair?: { item_ids: string[] };
}

interface TFTierlistRow {
  artifact_id: string;
  comps: TFTierlistComp[];
}

interface TFCompDetailUnit {
  id: string;
  name: string;
  nameCn: string;
  nameEn: string;
  cost: number;
  builds: { index: number; items: { id: string }[] }[];
}

interface TFCompDetailTrait {
  id: string;
  icon: string;
  unitsRequired: number;
  tierIndex: number | null;
}

interface TFPriorityRow {
  unitId: string;
  unitCost: number;
  unitPriority: number | null;
  bisItemIds: string[] | null;
  buckets: {
    Core: { itemId: string; necessity: number; priorityScore: number | null }[];
    Important: { itemId: string; necessity: number; priorityScore: number | null }[];
    Optional: { itemId: string; necessity: number; priorityScore: number | null }[];
    HighFrequency: { itemId: string; necessity: number; priorityScore: number | null }[];
  };
}

interface TFArtifactPick {
  kind: string;
  artifactId: string;
  artifactName: string;
  artifactIcon: string;
  carrierUnitId: string;
  carrierName: string;
  carrierIcon: string;
  carrierCost: number;
  strength: number;
  topPair: { items: { itemId: string; name: string; icon: string }[] } | null;
}

interface TFCompDetail {
  slug: string;
  key: string;
  title: string;
  titles: Record<string, string>;
  overview: {
    stats: { avgPlacement: string; avgPlacementValue: number; winRate: string; winRateValue: number };
    coreUnits: TFCompDetailUnit[];
    traits: TFCompDetailTrait[];
    teamCode: string;
  };
  summary: { sampleSize: number; winRate: string };
  metatftLink?: string;
  filters?: { expr: string; carrierExclusions: string[]; exprByLocale: Record<string, string>; carrierExclusionsByLocale: Record<string, string> };
  variants: {
    is_primary: boolean;
    variant_id: string;
    variant_label: string;
    clustered_stats: { share: number; avg_placement: number };
    units: { id: string }[];
    traits: TFCompDetailTrait[];
  }[];
  priorityMatrixRows: TFPriorityRow[];
  specialGroups: {
    artifacts: { unitId: string; items: { id: string; avgPlacement: number }[] }[];
    radiants: { unitId: string; items: { id: string; avgPlacement: number }[] }[];
    emblems: {
      itemId: string;
      avgPlacement: number;
      appearanceRate: number;
      carriers: { unit_id: string; best: boolean }[];
    }[];
  };
  matchups: {
    benefits: { key: string; delta: number }[];
    counteredBy: { key: string; delta: number }[];
    mutualConstraints: { key: string; delta: number }[];
  } | null;
  artifactPicks: { compStrength: number | null; picks: TFArtifactPick[] } | null;
  dataSource: { patch: string; sample_count: number };
}

interface TFTtableData {
  patch: string;
  ranked_tiers: string[];
  sampleSize: number;
  timestamp: string;
  metaStats: TFMetaStats;
  compositionCards: TFCompositionCard[];
  compStrengthTrends: TFCompStrengthTrends;
  compAtlas: {
    compositionSummaries: TFCompSummary[];
    compStats: Record<string, TFCompStat>;
  };
  trends: {
    dateColumns: string[];
    versionDeltaMode: string;
    versionDeltaWindowHint: string;
    compositionRows: TFTrendRow[];
    compositionTrendUp: TFTrendItem[];
    compositionTrendDown: TFTrendItem[];
    winshareTrendUp: TFTrendItem[];
    winshareTrendDown: TFTrendItem[];
  };
  dailyStats: {
    dateColumns: string[];
    versionDeltaMode: string;
    versionDeltaWindowHint: string;
    compositionRows: TFTrendRow[];
    compositionTrendUp: TFTrendItem[];
    compositionTrendDown: TFTrendItem[];
    winshareTrendUp: TFTrendItem[];
    winshareTrendDown: TFTrendItem[];
  };
  unitItems: TFUnitItem[];
  itemGuide: {
    items: TFItemGuideEntry[];
    artifacts: TFItemGuideEntry[];
    emblems: TFItemGuideEntry[];
  };
  itemTierlist: {
    generated_at?: string;
    tabs: {
      artifacts: { rows: TFTierlistRow[] };
      "anima-items": { rows: TFTierlistRow[] };
      emblems: { rows: TFTierlistRow[]; essentials?: TFTierlistRow[] };
    };
  };
  compDetails: Record<string, {
    composition: TFCompDetail;
    itemCategories: Record<string, string[]>;
    unitCategories: Record<string, string[]>;
  }>;
}

const view = ref<"matches" | "stats" | "comps" | "data" | "meta">("matches");
const matches = ref<MatchSummary[]>([]);
const info = ref<AppInfo | null>(null);
const summoner = ref<SummonerInfo | null>(null);
const lcuSource = ref("");
const loading = ref(false);
const error = ref("");
const lastSync = ref("");

const selected = ref<EnrichedMatch | null>(null);
const detailLoading = ref(false);

const stats = ref<Stats | null>(null);
const statsLoading = ref(false);

const toast = ref("");
let toastTimer: ReturnType<typeof setTimeout> | null = null;

function showToast(msg: string) {
  toast.value = msg;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (toast.value = ""), 5000);
}

const overlayMatches = ref<MatchSummary[]>([]);
const overlayAvg = ref("");
const overlayProfile = ref<Profile | null>(null);

async function loadOverlay() {
  try {
    const [matches, profile] = await Promise.all([
      invoke<MatchSummary[]>("list_matches", { limit: 8 }),
      invoke<Profile>("get_profile"),
    ]);
    overlayMatches.value = matches;
    const p = matches.map((m) => m.placement);
    overlayAvg.value =
      p.length > 0
        ? (p.reduce((a, b) => a + b, 0) / p.length).toFixed(1)
        : "";
    overlayProfile.value = profile;
  } catch {
    /* 静默失败 */
  }
}

async function toggleOverlay() {
  try {
    await invoke<boolean>("toggle_overlay");
  } catch (e) {
    error.value = String(e);
  }
}

async function minWin() {
  await getCurrentWindow().minimize();
}

async function maxWin() {
  const w = getCurrentWindow();
  if (await w.isMaximized()) {
    await w.unmaximize();
  } else {
    await w.maximize();
  }
}

async function hideWin() {
  await getCurrentWindow().hide();
}

async function openStats() {
  view.value = "stats";
  statsLoading.value = true;
  error.value = "";
  try {
    stats.value = await invoke<Stats>("get_stats");
  } catch (e) {
    error.value = String(e);
  } finally {
    statsLoading.value = false;
  }
}

const dataTab = ref<
  "overview" | "champions" | "traits" | "items" | "augments" | "wisps" | "loot" | "gold" | "reroll" | "odds" | "rules"
>("overview");
const dataLoading = ref(false);
const items = ref<ItemData[]>([]);
const champions = ref<ChampionData[]>([]);
const traits = ref<TraitData[]>([]);
const augments = ref<ItemData[]>([]);
const shopOdds = ref<ShopOddsLevel[]>([]);
const gameRules = ref<RulesSection[]>([]);
const lootTables: { title: string; rows: string[][] }[] = [
  {
    title: "常见战利品球（按回合）",
    rows: [
      ["回合", "掉落"],
      ["1-1 ~ 1-3", "起始基础战利品球（金币/散件）"],
      ["2-1 / 2-5", "野怪回合掉落（金币、散件）"],
      ["3-1 / 3-5", "野怪回合更高价值（完整装、散件、金币）"],
      ["4-1 ~ 4-7", "高价值合成装 / 纹章 / 金币"],
      ["5-1+", "取决于野怪，可能含强装 / 神器"],
    ],
  },
];
const goldTables: { title: string; rows: string[][] }[] = [
  {
    title: "金币 / 利息规则",
    rows: [
      ["项目", "说明"],
      ["基础回合收入", "每回合自动 +5 金币"],
      ["利息", "每存 10 金币回合结束 +1（上限 +5，存 50 封顶）"],
      ["连胜/连败", "2 连胜 +1，4 连胜及以上 +2"],
      ["胜利奖励", "胜利方每回合 +1 金币"],
      ["野怪回合", "野怪回合结束免费刷新一次"],
    ],
  },
];
const rerollTables: { title: string; rows: string[][] }[] = [
  {
    title: "商店刷新规则",
    rows: [
      ["项目", "说明"],
      ["刷新费用", "每次刷新商店 2 金币"],
      ["免费刷新", "胜利后获得 1 次免费刷新（不可叠加）"],
      ["野怪回合", "野怪回合结束免费刷新一次"],
      ["商店英雄数", "标准回合 5 个英雄"],
    ],
  },
];
const search = ref("");

const itemMap = computed(() => {
  const m: Record<string, ItemData> = {};
  for (const i of items.value) m[i.id] = i;
  return m;
});
const components = computed(() => items.value.filter((i) => i.composition.length === 0));
const craftable = computed(() => items.value.filter((i) => i.composition.length > 0));

function matchName(q: string) {
  return (name: string) => !q || name.toLowerCase().includes(q.toLowerCase());
}

const filteredItems = computed(() => {
  let list = craftable.value.filter((i) => matchName(search.value)(i.name));
  if (itemFilter.value !== "all") {
    const needle = itemFilter.value === "artifact" ? "Artifact" : "Emblem";
    list = list.filter((i) => i.id.toLowerCase().includes(needle.toLowerCase()));
  }
  return list;
});
const filteredChampions = computed(() =>
  champions.value.filter(
    (c) =>
      matchName(search.value)(c.name) || c.traits.some((t) => matchName(search.value)(t))
  )
);
const filteredTraits = computed(() =>
  traits.value.filter((t) => matchName(search.value)(t.name))
);

// 小精灵（Wisps）：是从 items 里 apiName 含 "Wisp" 的特殊道具
const wisps = computed(() =>
  items.value.filter(
    (i) => i.id.toLowerCase().includes("wisp") && !i.id.toLowerCase().includes("augment")
  )
);

// 赛季总览汇总数据
const overview = computed(() => {
  const costGroups: Record<number, ChampionData[]> = { 1: [], 2: [], 3: [], 4: [], 5: [] };
  for (const c of champions.value) {
    if (costGroups[c.cost]) costGroups[c.cost].push(c);
  }
  return {
    champions: champions.value.length,
    traits: traits.value.length,
    items: craftable.value.length + components.value.length,
    augments: augments.value.length,
    wisps: wisps.value.length,
    costGroups,
  };
});

// 反查：每件装备 → 适合哪些弈子（由 itemGuide 直接提供，见下方）
const costColors = ["#9aa4af", "#35c66b", "#3fa8f4", "#c26bf1", "#f5b23e"];

// 阵容趋势图配色
const trendColors = ["#f0c24b", "#4d9de0", "#45d483", "#c26bf1", "#f0716f"];

const filteredAugments = computed(() =>
  augments.value.filter((a) => matchName(search.value)(a.name))
);

// 装备筛选：全部 / 神器 / 纹章
const itemFilter = ref<"all" | "artifact" | "emblem">("all");

// 从赛季总览点击跳到对应 tab 并搜索
function openDataTo(tab: "champions" | "traits" | "items", query: string) {
  dataTab.value = tab;
  search.value = query;
}

async function openData() {
  view.value = "data";
  dataLoading.value = true;
  error.value = "";
  try {
    const [i, c, t, a, o, r] = await Promise.all([
      invoke<ItemData[]>("get_items"),
      invoke<ChampionData[]>("get_champions"),
      invoke<TraitData[]>("get_traits"),
      invoke<ItemData[]>("get_augments"),
      invoke<ShopOddsLevel[]>("get_shop_odds"),
      invoke<RulesSection[]>("get_game_rules"),
    ]);
    items.value = i;
    champions.value = c;
    traits.value = t;
    augments.value = a;
    shopOdds.value = o;
    gameRules.value = r;
  } catch (e) {
    error.value = String(e);
  } finally {
    dataLoading.value = false;
  }
}

const tftData = ref<TFTtableData | null>(null);
const metaTab = ref<"overview" | "trends" | "daily" | "tierlist" | "unitItems" | "itemGuide">("overview");
const metaLoading = ref(false);
const selectedCompSlug = ref<string | null>(null);
const tierlistTab = ref<"artifacts" | "emblems">("artifacts");
const itemGuideTab = ref<"items" | "artifacts" | "emblems">("items");
const itemSearch = ref("");
const dailyMode = ref<"playRate" | "avp" | "patch">("playRate");
const showDetails = ref(false);

const TF = "https://tftable.cc";
function tfImg(path: string | undefined): string {
  return path ? (path.startsWith("/") ? TF + path : path) : "";
}
function unitIcon(id: string): string { return TF + "/set18/avatar-webp/" + id + ".webp?v=5"; }
function itemIcon(id: string): string { return TF + "/set18/item-webp/" + id + ".webp?v=4"; }
// eslint-disable-next-line @typescript-eslint/no-unused-vars
function traitIcon(id: string): string { return TF + "/set18/trait-svg/" + id + ".svg?v=1"; }

// ===== 名称映射（中文优先） =====
const NAME_MAP = {"units":{"DA_18_ElderDragon":{"cn":"远古巨龙","en":"Elder Dragon","c":5},"DA_Draven18":{"cn":"德莱文","en":"Draven","c":5},"DA_18_Maokai":{"cn":"茂凯","en":"Maokai","c":5},"DA_18_Kennen":{"cn":"凯南","en":"Kennen","c":5},"DA_Taric18":{"cn":"塔里克","en":"Taric","c":5},"DA_Amumu18":{"cn":"阿木木","en":"Amumu","c":4},"DA_Sentinel18":{"cn":"苍蓝雕纹魔像","en":"Sentinel","c":4},"DA_Lux18_Base":{"cn":"拉克丝","en":"Lux","c":5},"DA_18_Ivern":{"cn":"艾翁","en":"Ivern","c":5},"DA_18_Ezreal":{"cn":"伊泽瑞尔","en":"Ezreal","c":4},"DA_18_GnarSmall":{"cn":"纳尔","en":"Gnar","c":5},"DA_18_Alistar":{"cn":"阿利斯塔","en":"Alistar","c":2},"DA_18_Ahri":{"cn":"阿狸","en":"Ahri","c":4},"DA_18_Morgana":{"cn":"莫甘娜","en":"Morgana","c":4},"DA_18_Sett":{"cn":"瑟提","en":"Sett","c":4},"DA_18_Alune":{"cn":"拉露恩","en":"Alune","c":5},"DA_18_Ashe":{"cn":"艾希","en":"Ashe","c":5},"DA_Krug18":{"cn":"远古石甲虫","en":"Krug","c":3},"DA_18_Sentry":{"cn":"苍蓝哨戒","en":"Pebbles","c":1},"DA_18_Malphite":{"cn":"墨菲特","en":"Malphite","c":4},"DA_Nidalee18_AP":{"cn":"奈德丽","en":"Nidalee","c":4},"DA_18_Sivir":{"cn":"希维尔","en":"Sivir","c":4},"DA_KogMaw18_AD":{"cn":"克格莫","en":"Kog'Maw","c":3},"DA_Cinderling18":{"cn":"绯红树怪","en":"Cinderling","c":1},"DA_18_RekSai":{"cn":"雷克塞","en":"Rek'Sai","c":1},"DA_18_Veigar":{"cn":"维迦","en":"Veigar","c":1},"DA_18_Ornn":{"cn":"奥恩","en":"Ornn","c":1},"DA_18_LeBlanc":{"cn":"乐芙兰","en":"LeBlanc","c":2},"DA_18_Cassiopeia":{"cn":"卡西奥佩娅","en":"Cassiopeia","c":3},"DA_Fiddlesticks18":{"cn":"费德提克","en":"Fiddlesticks","c":3},"DA_18_Soraka":{"cn":"索拉卡","en":"Soraka","c":4},"DA_18_Zyra":{"cn":"婕拉","en":"Zyra","c":4},"DA_18_Azir":{"cn":"阿兹尔","en":"Azir","c":3},"DA_18_Yorick":{"cn":"约里克","en":"Yorick","c":1},"DA_Vi18":{"cn":"蔚","en":"Vi","c":3},"DA_18_Rakan":{"cn":"洛","en":"Rakan","c":1},"DA_18_Aphelios":{"cn":"厄斐琉斯","en":"Aphelios","c":4},"DA_18_Diana":{"cn":"黛安娜","en":"Diana","c":3},"DA_18_Varus":{"cn":"韦鲁斯","en":"Varus","c":1},"DA_Karma18":{"cn":"卡尔玛","en":"Karma","c":1},"DA_18_Hecarim":{"cn":"赫卡里姆","en":"Hecarim","c":3},"DA_18_Akali_AD":{"cn":"阿卡丽","en":"Akali","c":1},"DA_18_Camille":{"cn":"卡蜜尔","en":"Camille","c":1},"DA_18_Leona":{"cn":"蕾欧娜","en":"Leona","c":1},"DA_18_Kayle":{"cn":"凯尔","en":"Kayle","c":2},"DA_18_Sejuani":{"cn":"瑟庄妮","en":"Sejuani","c":2},"DA_18_Lillia":{"cn":"莉莉娅","en":"Lillia","c":4},"DA_18_Xayah":{"cn":"霞","en":"Xayah","c":1},"DA_Scuttlecrab18":{"cn":"峡谷迅捷蟹","en":"Scuttlecrab","c":2},"DA_Brambleback18":{"cn":"绯红印记树怪","en":"Brambleback","c":4},"DA_Murkwolf18":{"cn":"暗影狼","en":"Murkwolf","c":2},"DA_18_Yunara":{"cn":"芸阿娜","en":"Yunara","c":2},"DA_18_Rengar":{"cn":"雷恩加尔","en":"Rengar","c":3},"DA_18_Tristana":{"cn":"崔丝塔娜","en":"Tristana","c":3},"DA_18_Rammus":{"cn":"拉莫斯","en":"Rammus","c":3},"DA_18_KhaZix":{"cn":"卡兹克","en":"Kha'Zix","c":3},"DA_18_MasterYi_AD":{"cn":"易","en":"Master Yi","c":3},"DA_CrimsonRaptor18":{"cn":"深红锋喙鸟","en":"Mama Beak","c":3},"DA_18_Caitlyn":{"cn":"凯特琳","en":"Caitlyn","c":2},"DA_18_Shen":{"cn":"慎","en":"Shen","c":2},"DA_18_Elise":{"cn":"伊莉丝","en":"DA_18_Elise","c":0},"DA_Gromp18_AP":{"cn":"魔沼蛙","en":"DA_Gromp18_AP","c":0},"DA_18_Teemo":{"cn":"提莫","en":"DA_18_Teemo","c":0},"DA_18_Kobuko":{"cn":"可酷伯","en":"DA_18_Kobuko","c":0},"DA_18_Warwick":{"cn":"沃里克","en":"DA_18_Warwick","c":0}},"items":{"DA_AdaptiveHelm":{"cn":"适应性头盔","en":"Adaptive Helm"},"DA_ArchangelsStaff":{"cn":"大天使之杖","en":"Archangel's Staff"},"DA_BlueBuff":{"cn":"蓝霸符","en":"Blue Buff"},"DA_BrambleVest":{"cn":"棘刺背心","en":"Bramble Vest"},"DA_Crownguard":{"cn":"冕卫","en":"Crownguard"},"DA_DragonsClaw":{"cn":"巨龙之爪","en":"Dragon's Claw"},"DA_EdgeOfNight":{"cn":"夜之锋刃","en":"Edge of Night"},"DA_Evenshroud":{"cn":"薄暮法袍","en":"Evenshroud"},"DA_GargoyleStoneplate":{"cn":"石像鬼石板甲","en":"Gargoyle Stoneplate"},"DA_GiantSlayer":{"cn":"巨人杀手","en":"Giant Slayer"},"DA_GuinsoosRageblade":{"cn":"鬼索的狂暴之刃","en":"Guinsoo's Rageblade"},"DA_HandOfJustice":{"cn":"正义之手","en":"Hand Of Justice"},"DA_HextechGunblade":{"cn":"海克斯科技枪刃","en":"Hextech Gunblade"},"DA_InfinityEdge":{"cn":"无尽之刃","en":"Infinity Edge"},"DA_IonicSpark":{"cn":"离子火花","en":"Ionic Spark"},"DA_KrakensFury":{"cn":"海妖之怒","en":"Kraken's Fury"},"DA_LastWhisper":{"cn":"最后的轻语","en":"Last Whisper"},"DA_Morellonomicon":{"cn":"莫雷洛秘典","en":"Morellonomicon"},"DA_ProtectorsVow":{"cn":"圣盾使的誓约","en":"Protector's Vow"},"DA_Quicksilver":{"cn":"水银","en":"Quicksilver"},"DA_RabadonsDeathcap":{"cn":"灭世者的死亡之帽","en":"Rabadon's Deathcap"},"DA_RedBuff":{"cn":"红霸符","en":"Red Buff"},"DA_SpearOfShojin":{"cn":"朔极之矛","en":"Spear of Shojin"},"DA_SpiritVisage":{"cn":"振奋盔甲","en":"Spirit Visage"},"DA_SteraksGage":{"cn":"斯特拉克的挑战护手","en":"Sterak's Gage"},"DA_StrikersFlail":{"cn":"强袭者的链枷","en":"Striker's Flail"},"DA_SunfireCape":{"cn":"日炎斗篷","en":"Sunfire Cape"},"DA_ThiefsGloves":{"cn":"窃贼手套","en":"Thief's Gloves"},"DA_TitansResolve":{"cn":"泰坦的坚决","en":"Titan's Resolve"},"DA_VoidStaff":{"cn":"虚空之杖","en":"Void Staff"},"DA_WarmogsArmor":{"cn":"狂徒铠甲","en":"Warmogs Armor"},"DA_Deathblade":{"cn":"死亡之刃","en":"Deathblade"},"DA_SteadfastHeart":{"cn":"坚定之心","en":"Steadfast Heart"},"DA_JeweledGauntlet":{"cn":"珠光护手","en":"Jeweled Gauntlet"},"DA_Bloodthirster":{"cn":"饮血剑","en":"Bloodthirster"},"DA_NashorsTooth":{"cn":"纳什之牙","en":"Nashor's Tooth"},"DA_Artifact_SilvermereDawn":{"cn":"密银黎明","en":"Silvermere Dawn"},"DA_Artifact_ZhonyasParadox":{"cn":"Zhonya's Paradox","en":"Zhonya's Paradox"},"DA_Artifact_LightshieldCrest":{"cn":"光盾徽章","en":"Lightshield Crest"},"DA_Artifact_TheIndomitable":{"cn":"顽强不屈","en":"The Indomitable"},"DA_Artifact_Dawncore":{"cn":"黎明核心","en":"Dawncore"},"DA_Artifact_VoidGauntlet":{"cn":"虚空护手","en":"Void Gauntlet"},"DA_Artifact_GoldCollector":{"cn":"金币收集者","en":"Gold Collector"},"DA_Artifact_AegisOfDusk":{"cn":"黄昏圣盾","en":"Aegis of Dusk"},"DA_Artifact_LudensTempest":{"cn":"卢登的激荡","en":"Luden's Tempest"},"DA_Artifact_HellfireHatchet":{"cn":"Hellfire Hatchet","en":"Hellfire Hatchet"},"DA_Artifact_SeekersArmguard":{"cn":"探索者的护臂","en":"Seeker's Armguard"},"DA_Artifact_WitsEnd":{"cn":"Wit's End","en":"Wit's End"},"DA_Artifact_HorizonFocus":{"cn":"视界专注","en":"Horizon Focus"},"DA_Artifact_LichBane":{"cn":"巫妖之祸","en":"Lich Bane"},"DA_Artifact_RapidFireCannon":{"cn":"疾射火炮","en":"Rapid Firecannon"},"DA_Artifact_Mittens":{"cn":"Mittens","en":"Mittens"},"DA_Artifact_NavoriFlickerblade":{"cn":"烁刃","en":"Flickerblades"},"DA_Artifact_BlightingJewel":{"cn":"Blighting Jewel","en":"Blighting Jewel"},"DA_Artifact_EternalPact":{"cn":"永恒契约","en":"Eternal Pact"},"DA_Artifact_Manazane":{"cn":"魔蕴","en":"Manazane"},"DA_Artifact_Fishbones":{"cn":"鱼骨头","en":"Fishbones"},"DA_Artifact_InfinityForce":{"cn":"三相之力","en":"Infinity Force"},"DA_Artifact_TitanicHydra":{"cn":"巨型九头蛇","en":"Titanic Hydra"},"DA_Artifact_Hullcrusher":{"cn":"Hullcrusher","en":"Hullcrusher"},"DA_Artifact_StatikkShiv":{"cn":"Statikk Shiv","en":"Statikk Shiv"},"DA_Artifact_ForbiddenIdol":{"cn":"Forbidden Idol","en":"Forbidden Idol"},"DA_Artifact_AegisOfDawn":{"cn":"黎明圣盾","en":"Aegis of Dawn"},"DA_18_EmblemElderwood":{"cn":"永恒之森纹章","en":"Elderwood Emblem"},"DA_18_EmblemInvoker":{"cn":"神谕纹章","en":"Invoker Emblem"},"DA_18_EmblemJuggernaut":{"cn":"主宰纹章","en":"Juggernaut Emblem"},"DA_18_EmblemFloraFatalisAugment":{"cn":"绝命花妖纹章","en":"Flora Fatalis Emblem"},"DA_18_EmblemExecutioner":{"cn":"裁决使纹章","en":"Executioner Emblem"},"DA_18_EmblemRapidfire":{"cn":"迅捷射手纹章","en":"Rapidfire Emblem"},"DA_18_EmblemFae":{"cn":"花仙子纹章","en":"Fae Emblem"},"DA_18_EmblemSpellweaver":{"cn":"法师纹章","en":"Spellweaver Emblem"},"DA_18_EmblemVanguard":{"cn":"重装战士纹章","en":"Vanguard Emblem"},"DA_18_EmblemBrawler":{"cn":"斗士纹章","en":"Brawler Emblem"},"DA_18_EmblemDefender":{"cn":"护卫纹章","en":"Defender Emblem"},"DA_18_EmblemSlayer":{"cn":"狂战士纹章","en":"Ravager Emblem"},"DA_18_EmblemHunter":{"cn":"猎人纹章","en":"Hunter Emblem"},"DA_18_EmblemInferno":{"cn":"地狱火纹章","en":"Inferno Emblem"},"DA_18_EmblemFloraFatalis":{"cn":"绝命花妖纹章","en":"Flora Fatalis Emblem"},"DA_18_EmblemBlossom":{"cn":"灵魂莲华纹章","en":"Blossom Emblem"},"DA_18_EmblemPrimal":{"cn":"野兽之灵纹章","en":"Primal Emblem"},"DA_18_EmblemLunar":{"cn":"月蚀骑士纹章","en":"Lunar Emblem"},"DA_18_EmblemBlackthorn":{"cn":"黑荆棘纹章","en":"Blackthorn Emblem"},"DA_18_EmblemSprykin":{"cn":"约德尔人和朋友纹章","en":"Sprykin Emblem"},"DA_Artifact_MogulsMail":{"cn":"大亨之铠","en":"Mogul'sMail"},"DA_Item_Artifact_TalismanOfAscension":{"cn":"Talisman of Ascension","en":"Talisman of Ascension"},"DA_TacticiansShield":{"cn":"金锅锅冠冕","en":"DA_TacticiansShield"},"DA_Component_TearOfTheGoddess":{"cn":"女神之泪","en":"DA_Component_TearOfTheGoddess"},"DA_Component_ChainVest":{"cn":"锁子甲","en":"DA_Component_ChainVest"},"DA_Component_BFSword":{"cn":"暴风大剑","en":"DA_Component_BFSword"},"DA_TacticiansCrown":{"cn":"金铲铲冠冕","en":"DA_TacticiansCrown"},"DA_Component_NeedlesslyLargeRod":{"cn":"无用大棒","en":"DA_Component_NeedlesslyLargeRod"},"DA_TacticiansCape":{"cn":"金锅铲冠冕","en":"DA_TacticiansCape"},"DA_Component_NegatronCloak":{"cn":"负极斗篷","en":"DA_Component_NegatronCloak"},"DA_Component_GiantsBelt":{"cn":"巨人腰带","en":"DA_Component_GiantsBelt"},"DA_Component_SparringGloves":{"cn":"拳套","en":"DA_Component_SparringGloves"},"DA_Component_RecurveBow":{"cn":"反曲之弓","en":"DA_Component_RecurveBow"}},"traits":{"__others__":{"cn":"其他 (<0.3%)","en":"Others (<0.3%)"},"DA_18_Blossom":{"cn":"灵魂莲华","en":"Blossom"},"DA_18_Executioner":{"cn":"裁决使","en":"Executioner"},"DA_18_Lunar":{"cn":"月蚀骑士","en":"Lunar"},"DA_18_Inferno":{"cn":"地狱火","en":"Inferno"},"DA_18_Vanguard":{"cn":"重装战士","en":"Vanguard"},"DA_Riftbeast18":{"cn":"峡谷野怪","en":"Riftbeast"},"DA_18_ApexPredator":{"cn":"顶级掠食者","en":"DA_18_ApexPredator"},"DA_18_Maokai_UniqueTrait":{"cn":"远古树精","en":"DA_18_Maokai_UniqueTrait"},"DA_Juggernaut18":{"cn":"主宰","en":"DA_Juggernaut18"},"DA_18_Greenfather":{"cn":"翠神","en":"DA_18_Greenfather"},"DA_Emerald18":{"cn":"宝石骑士","en":"DA_Emerald18"},"DA_Primal18":{"cn":"野兽之灵","en":"DA_Primal18"},"DA_18_Hunter":{"cn":"猎人","en":"DA_18_Hunter"},"DA_DravenUniqueTrait18":{"cn":"赏金猎人","en":"DA_DravenUniqueTrait18"},"DA_18_Brawler":{"cn":"斗士","en":"DA_18_Brawler"},"DA_18_LuxUniqueTrait":{"cn":"自然之力！大元素使","en":"DA_18_LuxUniqueTrait"},"DA_18_Invoker":{"cn":"神谕","en":"DA_18_Invoker"},"DA_18_Caustic":{"cn":"帝王斑蝶","en":"DA_18_Caustic"},"DA_AluneUniqueTrait18":{"cn":"月华神女","en":"DA_AluneUniqueTrait18"},"DA_18_Fae":{"cn":"花仙子","en":"DA_18_Fae"},"DA_18_Defender":{"cn":"护卫","en":"DA_18_Defender"},"DA_18_ZyraUniqueTrait":{"cn":"荆棘之兴","en":"DA_18_ZyraUniqueTrait"},"DA_FloraFatalis18":{"cn":"绝命花妖","en":"DA_FloraFatalis18"},"DA_18_Summoner":{"cn":"召唤师","en":"DA_18_Summoner"},"DA_18_Blackthorn":{"cn":"黑荆棘","en":"DA_18_Blackthorn"},"DA_18_Battlemage":{"cn":"魔岩巨兽","en":"DA_18_Battlemage"},"DA_18_Rival":{"cn":"宿敌","en":"DA_18_Rival"},"DA_18_Adaptor":{"cn":"魔战士","en":"DA_18_Adaptor"},"DA_18_Spellweaver":{"cn":"法师","en":"DA_18_Spellweaver"},"DA_18_Sprykin":{"cn":"约德尔人和朋友","en":"DA_18_Sprykin"},"DA_18_Rapidfire":{"cn":"迅捷射手","en":"DA_18_Rapidfire"},"DA_18_Slayer":{"cn":"狂战士","en":"DA_18_Slayer"},"DA_18_Elderwood":{"cn":"永恒之森","en":"DA_18_Elderwood"},"DA_18_Solar":{"cn":"日蚀骑士","en":"DA_18_Solar"},"DA_18_Coven":{"cn":"魔女","en":"DA_18_Coven"}}};

function mapLookup(map: Record<string, any>, id: string): string {
  if (!id) return "";
  const e = map[id];
  if (e) return e.cn || e.en || id;
  // 兜底：去掉 DA_ 前缀并按驼峰分词
  let s = String(id).replace(/^DA_18_/, "").replace(/^DA_/, "").replace(/^TFT_/, "");
  s = s.replace(/18$/i, "");
  s = s.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/_/g, " ");
  return s;
}
function unitName(id: string): string {
  return mapLookup(NAME_MAP.units as Record<string, any>, id);
}
function itemName(id: string): string {
  return mapLookup(NAME_MAP.items as Record<string, any>, id);
}
function traitName(id: string): string {
  return mapLookup(NAME_MAP.traits as Record<string, any>, id);
}
function unitNameEn(id: string): string {
  if (!id) return "";
  const e = (NAME_MAP.units as Record<string, any>)[id];
  return e ? e.en || e.cn || id : id;
}

function showCompDetail(slug: string) {
  selectedCompSlug.value = slug;
  view.value = "comps";
}
function closeCompDetail() {
  selectedCompSlug.value = null;
}
const selectedCompDetail = computed(() => {
  if (!selectedCompSlug.value || !tftData.value) return null;
  return tftData.value.compDetails[selectedCompSlug.value] ?? null;
});
const selectedCard = computed(() => {
  if (!selectedCompSlug.value) return null;
  return cardOf(selectedCompSlug.value);
});
const specialGroupsVisible = computed(() => {
  if (!selectedCompDetail.value) return false;
  const sg = selectedCompDetail.value.composition.specialGroups;
  return sg.artifacts.length > 0 || sg.radiants.length > 0 || sg.emblems.length > 0;
});

async function getMeta() {
  metaLoading.value = true;
  error.value = "";
  try {
    const r = await invoke<TFTtableData>("get_tftable_data");
    tftData.value = r;
    lastSync.value = r.sampleSize > 0 ? `${r.metaStats.metadata.patch} · ${r.sampleSize.toLocaleString()} 样本` : "";
  } catch (e) {
    error.value = String(e);
    tftData.value = null;
  } finally {
    metaLoading.value = false;
  }
}

// === helpers ===
function cardOf(slug: string): TFCompositionCard {
  return tftData.value?.compositionCards.find((c) => c.slug === slug) ?? { key: slug, slug, display_name_cn: slug, display_name_en: slug, compIcon: "", compCost: 0, averagePlacementValue: 0 };
}
function statOf(slug: string): TFCompStat {
  return tftData.value?.compAtlas.compStats[slug] ?? { strength: 0, playRate: 0, strengthDelta: 0 };
}
function compName(slug: string): string {
  return cardOf(slug).display_name_cn || cardOf(slug).display_name_en || slug;
}
function compCardIcon(slug: string): string {
  const c = cardOf(slug);
  return c.compIcon ? tfImg(c.compIcon) : unitIcon(slug);
}
function compCarryIcon(slug: string): string {
  const detail = tftData.value?.compDetails[slug];
  if (detail?.composition.overview.coreUnits.length) return unitIcon(detail.composition.overview.coreUnits[0].id);
  return slug;
}
function costShareArr(obj: Record<string, number>): number[] {
  return [1, 2, 3, 4, 5].map((k) => obj[String(k)] ?? 0);
}
function fmtSigned(v: number | undefined | null, suffix: string = ""): string {
  if (v == null) return "—";
  const s = v > 0 ? "+" : "";
  return s + (v * 100).toFixed(2) + suffix;
}

const sortedComps = computed(() => {
  if (!tftData.value) return [];
  return [...tftData.value.compAtlas.compositionSummaries].sort((a, b) => a.avgPlacementValue - b.avgPlacementValue);
});
const winShareComps = computed(() => tftData.value ? [...tftData.value.metaStats.winShare.compositions].sort((a, b) => b.wins - a.wins) : []);
const top4ShareComps = computed(() => tftData.value ? [...tftData.value.metaStats.top4Share.compositions].sort((a, b) => b.wins - a.wins) : []);
const sortedTrendRows = computed(() => tftData.value ? [...tftData.value.trends.compositionRows].sort((a, b) => a.avgPlacementValue - b.avgPlacementValue) : []);

const trendDates = computed(() => tftData.value?.compStrengthTrends.dates ?? []);
const trendTopNames = computed(() => {
  if (!tftData.value) return [];
  const top = [...tftData.value.compStrengthTrends.series]
    .sort((a, b) => (b.points[b.points.length - 1]?.strength ?? 0) - (a.points[a.points.length - 1]?.strength ?? 0))
    .slice(0, 5);
  return top.map((s) => compName(s.slug));
});
const trendLines = computed(() => {
  if (!tftData.value) return [];
  const top = [...tftData.value.compStrengthTrends.series]
    .sort((a, b) => (b.points[b.points.length - 1]?.strength ?? 0) - (a.points[a.points.length - 1]?.strength ?? 0))
    .slice(0, 5);
  const dates = tftData.value.compStrengthTrends.dates;
  return top.map((s) =>
    dates.map((d) => {
      const pt = s.points.find((p) => p.date === d);
      return pt ? pt.strength : 0;
    })
  );
});
const balanceTrendLines = computed(() => {
  if (!tftData.value) return ["", ""];
  const bd = tftData.value.compStrengthTrends.balance_daily;
  const dates = tftData.value.compStrengthTrends.dates;
  const toPoints = (fn: (v: any) => number) =>
    dates
      .map((d, j) => {
        const v = bd[d];
        const val = v ? fn(v) : 0;
        return (40 + j * (600 / Math.max(1, dates.length - 1))) + "," + (205 - val * 100);
      })
      .join(" ");
  return [toPoints((v) => v.involution_index), toPoints((v) => v.diversity_index)];
});

const currentItemGuide = computed(() => {
  if (!tftData.value) return [];
  const g = tftData.value.itemGuide;
  let list: TFItemGuideEntry[];
  if (itemGuideTab.value === "artifacts") list = g.artifacts;
  else if (itemGuideTab.value === "emblems") list = g.emblems;
  else list = g.items;
  if (!itemSearch.value) return list;
  const q = itemSearch.value.toLowerCase();
  return list.filter((it) => it.itemName.toLowerCase().includes(q) || it.itemId.toLowerCase().includes(q));
});
function sortedBuckets(buckets: Record<string, { unitId: string; normalized: number; necessity: number }[]>): Record<string, { unitId: string; normalized: number; necessity: number }[]> {
  const keys = Object.keys(buckets).sort();
  const sorted: Record<string, any[]> = {};
  for (const k of keys) sorted[k] = buckets[k];
  return sorted;
}

const navGroups: {
  key: "matches" | "stats" | "comps" | "data" | "meta";
  label: string;
  icon: string;
  children?: { label: string; go: () => void }[];
}[] = [
  { key: "matches", label: "战绩", icon: "◉" },
  { key: "stats", label: "统计", icon: "▤" },
  { key: "comps", label: "推荐阵容", icon: "♜" },
  {
    key: "data",
      label: "机制手册",
    icon: "▦",
    children: [
      { label: "装备合成", go: () => (dataTab.value = "items") },
      { label: "英雄", go: () => (dataTab.value = "champions") },
      { label: "羁绊", go: () => (dataTab.value = "traits") },
      { label: "海克斯", go: () => (dataTab.value = "augments") },
      { label: "商店概率", go: () => (dataTab.value = "odds") },
      { label: "机制", go: () => (dataTab.value = "rules") },
    ],
  },
  {
    key: "meta",
    label: "数据",
    icon: "★",
    children: [
      { label: "Meta总览", go: () => (metaTab.value = "overview") },
      { label: "阵容趋势", go: () => (metaTab.value = "trends") },
      { label: "每日数据", go: () => (metaTab.value = "daily") },
      { label: "装备梯队", go: () => (metaTab.value = "tierlist") },
      { label: "按弈子", go: () => (metaTab.value = "unitItems") },
      { label: "按装备", go: () => (metaTab.value = "itemGuide") },
    ],
  },
];

function setView(v: "matches" | "stats" | "comps" | "data" | "meta") {
  view.value = v;
  if (v === "stats") openStats();
  if (v === "data") openData();
  if (v === "meta") getMeta();
  if (v === "comps") getMeta();
}

function navGo(v: "matches" | "stats" | "comps" | "data" | "meta", child?: { go: () => void }) {
  setView(v);
  if (child) child.go();
}

function pct(x: number) {
  return (x * 100).toFixed(1) + "%";
}

const maxDist = () => Math.max(1, ...(stats.value?.placement_dist ?? []));

const COST_COLORS = ["", "#9aa4af", "#35c66b", "#3fa8f4", "#c26bf1", "#f5b23e"];

async function loadMatches() {
  loading.value = true;
  error.value = "";
  try {
    matches.value = await invoke<MatchSummary[]>("list_matches", { limit: 50 });
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function loadInfo() {
  info.value = await invoke<AppInfo>("app_info");
}

async function sync() {
  loading.value = true;
  error.value = "";
  try {
    const r = await invoke<SyncResult>("sync_matches");
    summoner.value = r.summoner;
    lcuSource.value = r.source;
    lastSync.value = `已同步 ${r.count} 局`;
    await Promise.all([loadInfo(), loadMatches()]);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function openMatch(gameId: string) {
  detailLoading.value = true;
  error.value = "";
  try {
    selected.value = await invoke<EnrichedMatch>("get_match", { gameId });
  } catch (e) {
    error.value = String(e);
  } finally {
    detailLoading.value = false;
  }
}

function back() {
  selected.value = null;
}

function costColor(cost: number | null) {
  return COST_COLORS[cost ?? 0] ?? "#fff";
}

function fmtDate(s: string) {
  if (!s) return "—";
  const d = new Date(s);
  if (isNaN(d.getTime())) return s;
  return d.toLocaleString("zh-CN", { hour12: false });
}

onMounted(async () => {
  if (isOverlay) {
    await loadOverlay();
    await listen<MatchSummary | null>("match-ended", () => {
      loadOverlay();
    });
    setInterval(loadOverlay, 30000);
    return;
  }

  loadMatches();
  loadInfo();

  await listen<MatchSummary | null>("match-ended", (e) => {
    loadMatches();
    if (e.payload) {
      showToast(`检测到新对局，名次 #${e.payload.placement}`);
    } else {
      showToast("检测到新对局");
    }
  });

  await listen<SyncResult | null>("lcu-status", (e) => {
    if (e.payload) {
      summoner.value = e.payload.summoner;
      lcuSource.value = e.payload.source;
      loadMatches();
    } else {
      summoner.value = null;
      lcuSource.value = "";
    }
  });

  });
</script>

<template>
  <div v-if="isOverlay" class="overlay" data-tauri-drag-region>
    <div class="overlay-header">
      <span class="dot"></span>
      <span class="overlay-title">TFT Sidekick</span>
      <span v-if="overlayAvg" class="overlay-avg">均名次 {{ overlayAvg }}</span>
      <button class="overlay-close" title="关闭" @click="toggleOverlay">✕</button>
    </div>
    <div v-if="overlayProfile" class="overlay-rank">
      {{ overlayProfile.rank_tier }} {{ overlayProfile.rank_division }}
      <span class="overlay-lp">{{ overlayProfile.rank_lp }} LP</span>
      <span class="overlay-winrate">{{ (overlayProfile.win_rate * 100).toFixed(1) }}% WR</span>
    </div>
    <div class="overlay-places">
      <span
        v-for="m in overlayMatches"
        :key="m.game_id"
        class="place-badge"
        :class="`p${m.placement}`"
      >
        {{ m.placement }}
      </span>
      <span v-if="overlayMatches.length === 0" class="overlay-empty">暂无记录</span>
    </div>
    <div class="overlay-footer">对局结束后自动记录</div>
  </div>

  <div v-else class="window">
    <div class="titlebar" data-tauri-drag-region>
      <div class="titlebar-left">
        <span class="titlebar-logo">◈</span>
        <span class="titlebar-title">TFT Sidekick</span>
      </div>
      <div class="titlebar-controls" data-tauri-drag-region="false">
        <button class="titlebar-btn" title="最小化" @click="minWin">─</button>
        <button class="titlebar-btn" title="最大化" @click="maxWin">□</button>
        <button class="titlebar-btn close" title="关闭" @click="hideWin">✕</button>
      </div>
    </div>

    <div class="shell">
    <aside class="sidebar">
      <div class="brand">
        <div class="brand-logo">◈</div>
        <div>
          <h1><span class="brand-accent">TFT</span> Sidekick</h1>
          <span v-if="info" class="version">v{{ info.version }}</span>
        </div>
      </div>
      <nav>
        <div
          v-for="g in navGroups"
          :key="g.key"
          class="nav-group"
        >
          <a
            class="nav-item"
            :class="{ active: view === g.key }"
            @click="navGo(g.key)"
          >
            <span class="nav-ico">{{ g.icon }}</span>{{ g.label }}
            <span v-if="g.children && g.children.length" class="nav-caret">▾</span>
          </a>
          <div v-if="g.children && g.children.length" class="nav-sub">
            <a
              v-for="c in g.children"
              :key="c.label"
              class="nav-sub-item"
              @click="navGo(g.key, c)"
            >{{ c.label }}</a>
          </div>
        </div>
      </nav>
      <div class="sidebar-footer">
        <div class="lcu-status" :class="summoner ? 'ok' : 'off'">
          <span v-if="summoner">
            {{ summoner.game_name }}#{{ summoner.tag_line }}
            <em v-if="summoner.summoner_level">Lv.{{ summoner.summoner_level }}</em>
          </span>
          <span v-else>LCU 未连接</span>
        </div>
        <div v-if="lcuSource" class="lcu-source">凭据来源: {{ lcuSource }}</div>
        <button class="ghost small" @click="toggleOverlay">悬浮窗</button>
      </div>
    </aside>

    <main class="content">
      <div v-if="toast" class="toast">{{ toast }}</div>

      <!-- 统计视图 -->
      <template v-if="view === 'stats'">
        <header class="topbar">
          <h2>统计</h2>
        </header>

        <div v-if="statsLoading" class="empty">加载中…</div>
        <p v-if="error" class="error">{{ error }}</p>

        <template v-if="stats">
          <div class="stat-cards">
            <div class="stat-card">
              <div class="stat-value">{{ stats.total }}</div>
              <div class="stat-label">总对局</div>
            </div>
            <div class="stat-card">
              <div class="stat-value">{{ pct(stats.win_rate) }}</div>
              <div class="stat-label">吃鸡率</div>
            </div>
            <div class="stat-card">
              <div class="stat-value">{{ pct(stats.top4_rate) }}</div>
              <div class="stat-label">前四率</div>
            </div>
            <div class="stat-card">
              <div class="stat-value">{{ stats.avg_placement.toFixed(2) }}</div>
              <div class="stat-label">平均名次</div>
            </div>
            <div class="stat-card">
              <div class="stat-value">{{ stats.avg_level.toFixed(1) }}</div>
              <div class="stat-label">平均等级</div>
            </div>
          </div>

          <section class="board-section">
            <h3>名次分布</h3>
            <div class="dist">
              <div v-for="(c, i) in stats.placement_dist" :key="i" class="dist-row">
                <span class="dist-rank">#{{ i + 1 }}</span>
                <div class="dist-bar-bg">
                  <div class="dist-bar" :style="{ width: (c / maxDist()) * 100 + '%' }"></div>
                </div>
                <span class="dist-count">{{ c }}</span>
              </div>
            </div>
          </section>

          <section class="board-section">
            <h3>常用阵容</h3>
            <div class="top-list">
              <div v-for="u in stats.top_units" :key="u.id" class="top-item">
                <img v-if="u.icon_url" :src="u.icon_url" :alt="u.name" class="top-icon" />
                <span class="top-name">{{ u.name || u.id }}</span>
                <span class="top-count">{{ u.count }} 局</span>
              </div>
            </div>
          </section>

          <section class="board-section">
            <h3>常用羁绊</h3>
            <div class="top-list">
              <div v-for="t in stats.top_traits" :key="t.id" class="top-item">
                <img v-if="t.icon_url" :src="t.icon_url" :alt="t.name" class="top-icon" />
                <span class="top-name">{{ t.name || t.id }}</span>
                <span class="top-count">{{ t.count }} 局</span>
              </div>
            </div>
          </section>
        </template>
      </template>

      <!-- 数据表视图 -->
      <template v-else-if="view === 'data'">
        <header class="topbar">
          <h2>隐藏数据表</h2>
          <input v-model="search" placeholder="搜索名称 / 羁绊…" class="search" />
        </header>

        <div class="tabs">
          <button
            v-for="t in [
      { key: 'overview', label: '赛季总览' },
      { key: 'champions', label: '英雄' },
      { key: 'traits', label: '羁绊' },
      { key: 'items', label: '装备合成' },
      { key: 'augments', label: '海克斯' },
      { key: 'wisps', label: '小精灵' },
      { key: 'loot', label: '掉落球' },
      { key: 'gold', label: '金币利息' },
      { key: 'reroll', label: '刷新订阅' },
      { key: 'odds', label: '商店概率' },
      { key: 'rules', label: '机制' },
            ]"
            :key="t.key"
            class="tab"
            :class="{ active: dataTab === t.key }"
            @click="dataTab = t.key as any"
          >
            {{ t.label }}
          </button>
        </div>

        <div v-if="dataLoading" class="empty">加载中…</div>
        <p v-if="error" class="error">{{ error }}</p>

        <!-- 赛季总览 -->
        <template v-if="dataTab === 'overview'">
          <div class="comp-stat-cards">
            <div class="stat-card"><div class="stat-value">{{ overview.champions }}</div><div class="stat-label">英雄</div></div>
            <div class="stat-card"><div class="stat-value">{{ overview.traits }}</div><div class="stat-label">羁绊</div></div>
            <div class="stat-card"><div class="stat-value">{{ overview.items }}</div><div class="stat-label">装备</div></div>
            <div class="stat-card"><div class="stat-value">{{ overview.augments }}</div><div class="stat-label">海克斯</div></div>
            <div class="stat-card"><div class="stat-value">{{ overview.wisps }}</div><div class="stat-label">小精灵</div></div>
          </div>

          <section class="board-section">
            <h3>费用分布</h3>
            <div v-for="(list, cost) in overview.costGroups" :key="'cost' + cost" class="cost-group">
              <div class="cost-label">
                <span class="cost" :style="{ color: costColor(Number(cost)) }">{{ "★".repeat(Number(cost)) }}</span>
                费 · {{ list.length }} 个
              </div>
              <div class="champ-list cost-grid">
                <div v-for="c in list" :key="c.id" class="champ-card" @click="openDataTo('champions', c.name)">
                  <div class="champ-head">
                    <img v-if="c.icon_url" :src="c.icon_url" :alt="c.name" class="champ-icon" />
                    <div class="champ-name">{{ c.name }}</div>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </template>

        <!-- 小精灵 -->
        <template v-else-if="dataTab === 'wisps'">
          <div class="augment-list">
            <div v-for="w in wisps" :key="w.id" class="augment-card">
              <img v-if="w.icon_url" :src="w.icon_url" :alt="w.name" class="craft-icon" />
              <div class="craft-body">
                <div class="craft-name">{{ w.name || w.id }}</div>
                <div class="ability-desc">{{ w.desc || "—" }}</div>
              </div>
            </div>
          </div>
          <p v-if="wisps.length === 0" class="empty">暂无小精灵数据</p>
        </template>

        <!-- 装备合成 -->
        <template v-else-if="dataTab === 'items'">
          <div class="tabs" style="margin-bottom: 14px">
            <button class="tab" :class="{ active: itemFilter === 'all' }" @click="itemFilter = 'all'">全部</button>
            <button class="tab" :class="{ active: itemFilter === 'artifact' }" @click="itemFilter = 'artifact'">神器</button>
            <button class="tab" :class="{ active: itemFilter === 'emblem' }" @click="itemFilter = 'emblem'">纹章</button>
          </div>
          <section class="board-section">
            <h3>散件</h3>
            <div class="component-list">
              <div v-for="c in components" :key="c.id" class="component" :title="c.name">
                <img v-if="c.icon_url" :src="c.icon_url" :alt="c.name" />
                <span class="component-name">{{ c.name }}</span>
              </div>
            </div>
          </section>

          <section class="board-section">
            <h3>成品装备</h3>
            <div class="craft-list">
              <div v-for="item in filteredItems" :key="item.id" class="craft-item">
                <img v-if="item.icon_url" :src="item.icon_url" :alt="item.name" class="craft-icon" />
                <div class="craft-body">
                  <div class="craft-name">{{ item.name }}</div>
                  <div class="craft-recipe">
                    <template v-for="cid in item.composition" :key="cid">
                      <img
                        v-if="itemMap[cid]?.icon_url"
                        :src="itemMap[cid].icon_url"
                        :title="itemMap[cid].name"
                        class="recipe-icon"
                      />
                      <span v-else class="recipe-text">{{ itemMap[cid]?.name || cid }}</span>
                    </template>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </template>

        <!-- 英雄 -->
        <template v-else-if="dataTab === 'champions'">
          <div class="champ-list">
            <div v-for="c in filteredChampions" :key="c.id" class="champ-card">
              <div class="champ-head">
                <img v-if="c.icon_url" :src="c.icon_url" :alt="c.name" class="champ-icon" />
                <div>
                  <div class="champ-name">
                    <span class="cost" :style="{ color: costColor(c.cost) }">{{ "★".repeat(c.cost) || "—" }}</span>
                    {{ c.name }}
                  </div>
                  <div class="champ-traits">{{ c.traits.join(" / ") }}</div>
                </div>
              </div>
              <div class="champ-stats">
                HP {{ c.hp }} · 攻击 {{ c.damage }} · 护甲 {{ c.armor }} · 魔抗 {{ c.magic_resist }} · 蓝 {{ c.mana }}
              </div>
              <div class="champ-ability">
                <span class="ability-name">{{ c.ability_name }}</span>
                <span class="ability-desc">{{ c.ability_desc }}</span>
              </div>
            </div>
          </div>
        </template>

        <!-- 羁绊 -->
        <template v-else-if="dataTab === 'traits'">
          <div class="trait-list">
            <div v-for="t in filteredTraits" :key="t.id" class="trait-card">
              <div class="champ-head">
                <img v-if="t.icon_url" :src="t.icon_url" :alt="t.name" class="champ-icon" />
                <div class="champ-name">{{ t.name }}</div>
              </div>
              <div class="trait-tiers">
                <span
                  v-for="(tier, i) in t.tiers"
                  :key="i"
                  class="tier-pill"
                >
                  {{ tier.min_units }}{{ tier.max_units > tier.min_units ? "-" + tier.max_units : "" }}
                </span>
              </div>
              <div class="ability-desc">{{ t.desc }}</div>
              <div v-if="t.units.length > 0" class="trait-units">
                <span v-for="u in t.units" :key="u.id" class="trait-unit" :title="u.name">
                  <img v-if="u.icon_url" :src="u.icon_url" :alt="u.name" />
                </span>
              </div>
            </div>
          </div>
        </template>

        <!-- 海克斯 -->
        <template v-else-if="dataTab === 'augments'">
          <div class="augment-list">
            <div v-for="a in filteredAugments" :key="a.id" class="augment-card">
              <img v-if="a.icon_url" :src="a.icon_url" :alt="a.name" class="craft-icon" />
              <div class="craft-body">
                <div class="craft-name">{{ a.name }}</div>
                <div class="ability-desc">{{ a.desc || "—" }}</div>
              </div>
            </div>
          </div>
          <p v-if="filteredAugments.length === 0" class="empty">没有匹配的海克斯</p>
        </template>

        <!-- 掉落球 -->
        <template v-else-if="dataTab === 'loot'">
          <div class="odds-note">按回合掉落的战利品球内容（各版本可能微调）。</div>
          <div v-for="sec in lootTables" :key="sec.title" class="rule-section">
            <h3>{{ sec.title }}</h3>
            <table class="odds-table">
              <tbody>
                <tr v-for="(row, ri) in sec.rows" :key="ri">
                  <td v-for="(cell, ci) in row" :key="ci" :class="{ 'rule-key': ci === 0 }">{{ cell }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </template>

        <!-- 金币利息 -->
        <template v-else-if="dataTab === 'gold'">
          <div class="odds-note">金币获取 / 利息规则（各版本可能微调）。</div>
          <div v-for="sec in goldTables" :key="sec.title" class="rule-section">
            <h3>{{ sec.title }}</h3>
            <table class="odds-table">
              <tbody>
                <tr v-for="(row, ri) in sec.rows" :key="ri">
                  <td v-for="(cell, ci) in row" :key="ci" :class="{ 'rule-key': ci === 0 }">{{ cell }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </template>

        <!-- 刷新订阅 -->
        <template v-else-if="dataTab === 'reroll'">
          <div class="odds-note">商店刷新机制（各版本可能微调）。</div>
          <div v-for="sec in rerollTables" :key="sec.title" class="rule-section">
            <h3>{{ sec.title }}</h3>
            <table class="odds-table">
              <tbody>
                <tr v-for="(row, ri) in sec.rows" :key="ri">
                  <td v-for="(cell, ci) in row" :key="ci" :class="{ 'rule-key': ci === 0 }">{{ cell }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </template>

        <!-- 商店概率 -->
        <template v-else-if="dataTab === 'odds'">
          <div class="odds-note">各等级商店刷新英雄的概率（%）</div>
          <table class="odds-table">
            <thead>
              <tr>
                <th>等级</th>
                <th>1 费</th>
                <th>2 费</th>
                <th>3 费</th>
                <th>4 费</th>
                <th>5 费</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="l in shopOdds" :key="l.level">
                <td>{{ l.level }}</td>
                <td v-for="(o, i) in l.odds" :key="i">{{ o }}%</td>
              </tr>
            </tbody>
          </table>
        </template>

        <!-- 机制 -->
        <template v-else-if="dataTab === 'rules'">
          <div class="odds-note">基础机制规则，具体数值可能随补丁微调，以游戏内为准。</div>
          <div v-for="sec in gameRules" :key="sec.title" class="rule-section">
            <h3>{{ sec.title }}</h3>
            <table class="odds-table">
              <tbody>
                <tr v-for="(row, ri) in sec.rows" :key="ri">
                  <td v-for="(cell, ci) in row" :key="ci" :class="{ 'rule-key': ci === 0 }">
                    {{ cell }}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </template>
      </template>

      <!-- 阵容一览 + 阵容详情 -->
      <template v-else-if="view === 'comps'">
        <header class="topbar">
          <h2>阵容一览</h2>
          <button @click="getMeta" :disabled="metaLoading">
            {{ metaLoading ? "加载中…" : "刷新" }}
          </button>
        </header>
        <div class="odds-note">当前版本 {{ tftData?.patch }} · 高分段常用阵容，按强度排序。</div>

        <template v-if="selectedCompDetail && selectedCard">
          <div class="comp-detail">
            <header class="topbar">
              <button class="ghost" @click="closeCompDetail">← 返回阵容列表</button>
              <h2 class="detail-title">
                <img :src="tfImg(selectedCard.compIcon)" :alt="selectedCard.display_name_cn" class="top-icon" />
                {{ selectedCard.display_name_cn }}
                <span class="muted en-name">{{ selectedCard.display_name_en }}</span>
              </h2>
            </header>

            <!-- 概览统计 -->
            <div class="comp-stat-cards">
              <div class="stat-card">
                <div class="stat-value">{{ selectedCompDetail.composition.overview.stats.avgPlacement }}</div>
                <div class="stat-label">平均名次</div>
              </div>
              <div class="stat-card">
                <div class="stat-value">{{ selectedCompDetail.composition.overview.stats.winRate }}</div>
                <div class="stat-label">吃鸡率</div>
              </div>
              <div class="stat-card">
                <div class="stat-value">{{ selectedCompDetail.composition.summary.sampleSize.toLocaleString() }}</div>
                <div class="stat-label">样本</div>
              </div>
              <div class="stat-card">
                <div class="stat-value">{{ selectedCompDetail.composition.dataSource.patch }}</div>
                <div class="stat-label">数据版本</div>
              </div>
            </div>
            <div v-if="selectedCompDetail.composition.metatftLink" class="ext-link-wrap">
              <a :href="selectedCompDetail.composition.metatftLink" target="_blank" rel="noopener" class="ext-link">在 MetaTFT 查看该阵容 →</a>
            </div>

            <!-- 核心阵容 -->
            <section class="board-section">
              <h3>核心弈子与出装</h3>
              <div class="detail-board">
                <div v-for="u in selectedCompDetail.composition.overview.coreUnits" :key="u.id" class="detail-unit">
                  <div class="detail-unit-icon-wrap" :class="'cost' + u.cost">
                    <img :src="unitIcon(u.id)" :alt="u.nameCn" class="unit-icon" />
                  </div>
                  <div class="detail-unit-name">{{ u.nameCn }}</div>
                  <div class="detail-unit-en muted">{{ u.nameEn }}</div>
                  <div class="detail-builds">
                    <div v-for="b in u.builds" :key="b.index" class="detail-build" :title="'出装 ' + b.index">
                      <img v-for="it in b.items" :key="it.id" :src="itemIcon(it.id)" :alt="it.id" class="mini-item" />
                    </div>
                    <div v-if="!u.builds.length" class="muted tiny">无核心出装</div>
                  </div>
                </div>
              </div>
            </section>

            <!-- 羁绊 -->
            <section class="board-section">
              <h3>关键羁绊</h3>
              <div class="traits">
                <div v-for="t in selectedCompDetail.composition.overview.traits" :key="t.id" class="trait">
                  <img v-if="t.icon" :src="tfImg(t.icon)" :alt="traitName(t.id)" class="trait-icon" />
                  <span class="trait-name">{{ traitName(t.id) }}</span>
                  <span class="trait-tier">{{ t.unitsRequired }} 单位</span>
                </div>
              </div>
            </section>

            <!-- 变体 -->
            <section v-if="selectedCompDetail.composition.variants.length" class="board-section">
              <h3>阵容变体</h3>
              <div class="variant-list">
                <div v-for="(v, i) in selectedCompDetail.composition.variants" :key="v.variant_id" class="variant-card">
                  <div class="variant-head">
                    <span class="variant-name">变体 {{ i + 1 }}<span v-if="v.is_primary" class="badge">主力</span></span>
                    <span class="muted">占比 {{ (v.clustered_stats.share * 100).toFixed(1) }}% · 均 {{ v.clustered_stats.avg_placement.toFixed(2) }}</span>
                  </div>
                  <div class="variant-units">
                    <img v-for="u in v.units" :key="u.id" :src="unitIcon(u.id)" :alt="unitName(u.id)" class="mini-unit" :title="unitName(u.id)" />
                  </div>
                  <div v-if="v.traits.length" class="variant-traits">
                    <span v-for="t in v.traits" :key="t.id" class="trait-chip">
                      <img v-if="t.icon" :src="tfImg(t.icon)" :alt="traitName(t.id)" class="trait-icon-sm" />
                      {{ t.unitsRequired }} {{ traitName(t.id) }}
                    </span>
                  </div>
                </div>
              </div>
            </section>

            <!-- 出装优先级矩阵 -->
            <section class="board-section">
              <h3>出装优先级矩阵</h3>
              <div v-for="r in selectedCompDetail.composition.priorityMatrixRows" :key="r.unitId" class="prio-row">
                <div class="prio-head">
                  <img :src="unitIcon(r.unitId)" class="top-icon" :alt="unitName(r.unitId)" />
                  <span class="prio-unit">{{ unitName(r.unitId) }}</span>
                  <span class="muted tiny" style="margin-left:4px; font-size:10px">{{ unitNameEn(r.unitId) }}</span>
                  <span v-if="r.unitPriority != null" class="muted tiny" style="margin-left:4px">优先度 {{ r.unitPriority.toFixed(2) }}</span>
                </div>
                <div class="prio-buckets">
                  <div v-if="r.buckets.Core.length" class="prio-bucket">
                    <span class="prio-bucket-lbl">核心</span>
                    <div class="prio-items">
                      <div v-for="it in r.buckets.Core" :key="it.itemId" class="prio-item">
                        <img :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId) + ' (' + it.necessity.toFixed(2) + ')'" />
                        <span class="prio-item-nec">{{ it.necessity.toFixed(2) }}</span>
                      </div>
                    </div>
                  </div>
                  <div v-if="r.buckets.Important.length" class="prio-bucket">
                    <span class="prio-bucket-lbl">重要</span>
                    <div class="prio-items">
                      <div v-for="it in r.buckets.Important" :key="it.itemId" class="prio-item">
                        <img :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId) + ' (' + it.necessity.toFixed(2) + ')'" />
                        <span class="prio-item-nec">{{ it.necessity.toFixed(2) }}</span>
                      </div>
                    </div>
                  </div>
                  <div v-if="r.buckets.Optional.length" class="prio-bucket">
                    <span class="prio-bucket-lbl">可选</span>
                    <div class="prio-items">
                      <div v-for="it in r.buckets.Optional" :key="it.itemId" class="prio-item">
                        <img :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId) + ' (' + it.necessity.toFixed(2) + ')'" />
                        <span class="prio-item-nec">{{ it.necessity.toFixed(2) }}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </section>

            <!-- 特殊装备：光明神器 + 纹章 -->
            <section v-if="specialGroupsVisible" class="board-section">
              <h3>特殊装备推荐</h3>
              <div v-if="selectedCompDetail.composition.specialGroups.artifacts.length" class="special-block">
                <h4 class="special-lbl">神器</h4>
                <div class="artifact-picks">
                  <div v-for="g in selectedCompDetail.composition.specialGroups.artifacts" :key="g.unitId" class="artifact-pick">
                    <div class="artifact-pick-main">
                      <img :src="unitIcon(g.unitId)" :alt="unitName(g.unitId)" class="mini-unit" />
                      <span class="artifact-carrier">{{ unitName(g.unitId) }}</span>
                      <img v-for="it in g.items" :key="it.id" :src="itemIcon(it.id)" class="mini-item" :title="itemName(it.id) + ' 均 ' + it.avgPlacement.toFixed(2)" />
                    </div>
                  </div>
                </div>
              </div>
              <div v-if="selectedCompDetail.composition.specialGroups.radiants.length" class="special-block">
                <h4 class="special-lbl">光明装备</h4>
                <div class="artifact-picks">
                  <div v-for="g in selectedCompDetail.composition.specialGroups.radiants" :key="g.unitId" class="artifact-pick">
                    <div class="artifact-pick-main">
                      <img :src="unitIcon(g.unitId)" :alt="unitName(g.unitId)" class="mini-unit" />
                      <span class="artifact-carrier">{{ unitName(g.unitId) }}</span>
                      <img v-for="it in g.items" :key="it.id" :src="itemIcon(it.id)" class="mini-item" :title="itemName(it.id) + ' 均 ' + it.avgPlacement.toFixed(2)" />
                    </div>
                  </div>
                </div>
              </div>
              <div v-if="selectedCompDetail.composition.specialGroups.emblems.length" class="special-block">
                <h4 class="special-lbl">纹章</h4>
                <div class="artifact-picks">
                  <div v-for="g in selectedCompDetail.composition.specialGroups.emblems" :key="g.itemId" class="artifact-pick">
                    <div class="artifact-pick-main">
                      <img :src="itemIcon(g.itemId)" :alt="itemName(g.itemId)" class="mini-item" />
                      <span class="artifact-name">{{ itemName(g.itemId) }}</span>
                      <span class="muted tiny">均 {{ g.avgPlacement.toFixed(2) }} · 出现率 {{ (g.appearanceRate * 100).toFixed(1) }}%</span>
                      <img v-for="c in g.carriers" :key="c.unit_id" :src="unitIcon(c.unit_id)" class="mini-unit" :title="unitName(c.unit_id) + (c.best ? ' ★' : '')" />
                    </div>
                  </div>
                </div>
              </div>
            </section>

            <!-- 互害 / 受益 -->
            <section v-if="selectedCompDetail.composition.matchups" class="board-section">
              <h3>克制关系</h3>
              <div class="matchup-list">
                <div v-for="b in selectedCompDetail.composition.matchups.benefits" :key="b.key" class="matchup-row good">
                  <span>受益对阵</span>
                  <span>{{ compName(b.key) }}</span>
                  <span class="muted">delta {{ b.delta.toFixed(3) }}</span>
                </div>
                <div v-for="c in selectedCompDetail.composition.matchups.counteredBy" :key="c.key" class="matchup-row bad">
                  <span>被克制</span>
                  <span>{{ compName(c.key) }}</span>
                  <span class="muted">delta {{ c.delta.toFixed(3) }}</span>
                </div>
                <div v-for="m in selectedCompDetail.composition.matchups.mutualConstraints" :key="m.key" class="matchup-row neutral">
                  <span>互相挤压</span>
                  <span>{{ compName(m.key) }}</span>
                  <span class="muted">delta {{ m.delta.toFixed(3) }}</span>
                </div>
                <p v-if="!selectedCompDetail.composition.matchups.benefits.length && !selectedCompDetail.composition.matchups.counteredBy.length && !selectedCompDetail.composition.matchups.mutualConstraints.length" class="empty" style="padding: 8px 0; text-align: left">无克制数据。</p>
              </div>
            </section>

            <!-- 神器推荐 -->
            <section v-if="selectedCompDetail.composition.artifactPicks && selectedCompDetail.composition.artifactPicks.picks.length" class="board-section">
              <h3>神器推荐</h3>
              <div class="artifact-picks">
                <div v-for="p in selectedCompDetail.composition.artifactPicks.picks" :key="p.artifactId" class="artifact-pick">
                  <div class="artifact-pick-main">
                    <img :src="tfImg(p.artifactIcon)" :alt="p.artifactName" class="mini-item" />
                    <span class="artifact-name">{{ p.artifactName }}</span>
                    <span class="muted tiny">→</span>
                    <img :src="tfImg(p.carrierIcon)" :alt="p.carrierName" class="mini-unit" />
                    <span class="artifact-carrier">{{ p.carrierName }}</span>
                    <span class="muted tiny">强度 {{ p.strength.toFixed(3) }}</span>
                  </div>
                  <div v-if="p.topPair" class="artifact-pair">
                    <img v-for="it in p.topPair.items" :key="it.itemId" :src="tfImg(it.icon)" :alt="it.name" class="mini-item" :title="it.name" />
                  </div>
                </div>
              </div>
            </section>
          </div>
        </template>

        <template v-else>
          <div v-if="tftData && tftData.compAtlas.compositionSummaries.length" class="comp-grid">
            <div
              v-for="c in sortedComps"
              :key="c.slug"
              class="comp-card"
              @click="showCompDetail(c.slug)"
            >
              <div class="comp-card-head">
                <img :src="cardOf(c.slug).compIcon ? tfImg(cardOf(c.slug).compIcon) : unitIcon(compCarryIcon(c.slug))" :alt="cardOf(c.slug).display_name_cn" class="comp-card-icon" />
                <div class="comp-card-title">
                  <span class="comp-card-name">{{ cardOf(c.slug).display_name_cn }}</span>
                  <span class="comp-card-en muted">{{ cardOf(c.slug).display_name_en }}</span>
                </div>
                <span class="comp-cost-tag">${{ cardOf(c.slug).compCost }}</span>
              </div>
              <div class="comp-card-stats">
                <div class="comp-card-stat">
                  <span class="comp-card-val">{{ c.avgPlacementValue.toFixed(2) }}</span>
                  <span class="comp-card-lbl">平均名次</span>
                </div>
                <div class="comp-card-stat">
                  <span class="comp-card-val">{{ (statOf(c.slug).strength * 100).toFixed(1) }}%</span>
                  <span class="comp-card-lbl">强度</span>
                </div>
                <div class="comp-card-stat">
                  <span class="comp-card-val">{{ (statOf(c.slug).playRate * 100).toFixed(2) }}%</span>
                  <span class="comp-card-lbl">出场率</span>
                </div>
              </div>
              <div class="comp-card-delta" :class="{ up: c.trendDelta < 0, down: c.trendDelta > 0 }">
                {{ c.trendDelta < 0 ? "▲" : (c.trendDelta > 0 ? "▼" : "—") }} 名次变化 {{ Math.abs(c.trendDelta).toFixed(2) }}
              </div>
              <div class="comp-card-share">
                <span class="tiny muted">登顶占比变化 {{ fmtSigned(c.winShareDelta, "%") }}</span>
                <span class="tiny muted">| 出场变化 {{ fmtSigned(c.playRateDelta, "%") }}</span>
              </div>
            </div>
          </div>
          <p v-else-if="!metaLoading" class="empty">暂无阵容数据。</p>
        </template>
      </template>

      <!-- Meta 视图 -->
      <template v-else-if="view === 'meta'">
        <header class="topbar">
          <h2>数据</h2>
          <button @click="getMeta" :disabled="metaLoading">
            {{ metaLoading ? "加载中…" : "刷新" }}
          </button>
        </header>
        <p v-if="lastSync" class="hint" style="margin: 0 0 8px">{{ lastSync }}</p>
        <p v-if="tftData" class="hint" style="margin: 0 0 8px; font-size:11px; color:var(--faint)">更新时间: {{ new Date(tftData.timestamp).toLocaleString("zh-CN") }}</p>
        <p v-if="error" class="error">{{ error }}</p>

        <template v-if="tftData">
          <div class="tabs tabs-group">
            <span class="tabs-group-label">数据</span>
            <button
              v-for="t in [
                { key: 'overview', label: 'Meta总览' },
                { key: 'trends', label: '阵容趋势' },
                { key: 'daily', label: '每日数据' },
                { key: 'tierlist', label: '装备梯队' },
                { key: 'unitItems', label: '按弈子' },
                { key: 'itemGuide', label: '按装备' },
              ]"
              :key="t.key"
              class="tab"
              :class="{ active: metaTab === t.key }"
              @click="metaTab = t.key as any"
            >
              {{ t.label }}
            </button>
          </div>

          <!-- Meta 总览 -->
          <div v-if="metaTab === 'overview'">
            <div class="comp-stat-cards">
              <div class="stat-card">
                <div class="stat-value">{{ (tftData.metaStats.balance.involution_index * 100).toFixed(1) }}</div>
                <div class="stat-label">内卷指数</div>
              </div>
              <div class="stat-card">
                <div class="stat-value">{{ (tftData.metaStats.balance.diversity_index * 100).toFixed(1) }}</div>
                <div class="stat-label">多样性指数</div>
              </div>
              <div class="stat-card">
                <div class="stat-value">{{ tftData.sampleSize.toLocaleString() }}</div>
                <div class="stat-label">样本对局</div>
              </div>
              <div class="stat-card">
                <div class="stat-value">{{ tftData.patch }}</div>
                <div class="stat-label">版本</div>
              </div>
            </div>

            <!-- 方法论说明（对应网站 Show details） -->
            <section class="board-section">
              <button class="ghost small" @click="showDetails = !showDetails">
                {{ showDetails ? "收起方法论说明 ▲" : "指数计算方法说明 ▼" }}
              </button>
              <div v-if="showDetails" class="notes-block">
                <p class="note-line"><b>内卷指数：</b>{{ tftData.metaStats.balance.notes.involution }}</p>
                <p class="note-line"><b>多样性指数：</b>{{ tftData.metaStats.balance.notes.diversity }}</p>
              </div>
            </section>

            <!-- 每日内卷/多样性趋势 -->
            <section class="board-section">
              <h3>环境指数趋势（近 6 日）</h3>
              <div class="trend-legend">
                <span class="trend-legend-item"><span class="trend-legend-dot" style="background:#f0716f"></span>内卷指数</span>
                <span class="trend-legend-item"><span class="trend-legend-dot" style="background:#45d483"></span>多样性指数</span>
              </div>
              <svg class="trend-svg" viewBox="0 0 640 220" preserveAspectRatio="xMidYMid meet">
                <line v-for="n in 5" :key="'b'+n" x1="0" :y1="n*40" x2="640" :y2="n*40" stroke="#2a2a3a" stroke-width="0.5"/>
                <polyline
                  :points="balanceTrendLines[0]"
                  stroke="#f0716f"
                  stroke-width="2"
                  fill="none"
                />
                <polyline
                  :points="balanceTrendLines[1]"
                  stroke="#45d483"
                  stroke-width="2"
                  fill="none"
                />
              </svg>
            </section>

            <div class="two-col">
              <section class="board-section">
                <h3>登顶费用分布（Win Share）</h3>
                <div class="cost-bars">
                  <div v-for="(v, i) in costShareArr(tftData.metaStats.core_cost_share.win_share)" :key="'w' + i" class="cost-bar-row">
                    <span class="cost-bar-label" :style="{ color: costColors[i] }">{{ i + 1 }}费</span>
                    <div class="cost-bar-bg">
                      <div class="cost-bar" :style="{ width: (v * 100) + '%', background: costColors[i] }"></div>
                    </div>
                    <span class="cost-bar-val">{{ (v * 100).toFixed(1) }}%</span>
                  </div>
                </div>
              </section>
              <section class="board-section">
                <h3>前四费用分布（Top4 Share）</h3>
                <div class="cost-bars">
                  <div v-for="(v, i) in costShareArr(tftData.metaStats.core_cost_share.top4_share)" :key="'t' + i" class="cost-bar-row">
                    <span class="cost-bar-label" :style="{ color: costColors[i] }">{{ i + 1 }}费</span>
                    <div class="cost-bar-bg">
                      <div class="cost-bar" :style="{ width: (v * 100) + '%', background: costColors[i] }"></div>
                    </div>
                    <span class="cost-bar-val">{{ (v * 100).toFixed(1) }}%</span>
                  </div>
                </div>
              </section>
            </div>

            <div class="two-col">
              <section class="board-section">
                <h3>登顶占比 · 阵容（Win Share · 样本 {{ tftData.metaStats.winShare.sample_size.toLocaleString() }}）</h3>
                <div class="meta-list">
                  <div class="meta-row meta-head">
                    <span class="meta-rank">#</span>
                    <span class="meta-name">阵容</span>
                    <span class="meta-num">登顶</span>
                    <span class="meta-num">占比</span>
                  </div>
                  <div
                    v-for="(c, i) in winShareComps"
                    :key="c.slug"
                    class="meta-row comp-row"
                    @click="showCompDetail(c.slug)"
                  >
                    <span class="meta-rank" :class="{ top3: i < 3 }">{{ i + 1 }}</span>
                    <span class="meta-name">
                      <img :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
                      {{ compName(c.slug) }}
                    </span>
                    <span class="meta-num">{{ c.wins }}</span>
                    <span class="meta-num accent">{{ ((c.wins / tftData.metaStats.winShare.sample_size) * 100).toFixed(2) }}%</span>
                  </div>
                </div>
              </section>
              <section class="board-section">
                <h3>前四占比 · 阵容（Top4 Share · 样本 {{ tftData.metaStats.top4Share.sample_size.toLocaleString() }}）</h3>
                <div class="meta-list">
                  <div class="meta-row meta-head">
                    <span class="meta-rank">#</span>
                    <span class="meta-name">阵容</span>
                    <span class="meta-num">前四</span>
                    <span class="meta-num">占比</span>
                  </div>
                  <div
                    v-for="(c, i) in top4ShareComps"
                    :key="c.slug"
                    class="meta-row comp-row"
                    @click="showCompDetail(c.slug)"
                  >
                    <span class="meta-rank" :class="{ top3: i < 3 }">{{ i + 1 }}</span>
                    <span class="meta-name">
                      <img :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
                      {{ compName(c.slug) }}
                    </span>
                    <span class="meta-num">{{ c.wins }}</span>
                    <span class="meta-num accent">{{ ((c.wins / tftData.metaStats.top4Share.sample_size) * 100).toFixed(2) }}%</span>
                  </div>
                </div>
              </section>
            </div>

            <!-- 阵容强度趋势 -->
            <section class="board-section">
              <h3>阵容强度趋势（近 6 日）</h3>
              <div class="trend-legend">
                <span v-for="(name, i) in trendTopNames" :key="i" class="trend-legend-item">
                  <span class="trend-legend-dot" :style="{ background: trendColors[i % trendColors.length] }"></span>
                  {{ name }}
                </span>
              </div>
              <svg class="trend-svg" viewBox="0 0 640 220" preserveAspectRatio="xMidYMid meet">
                <line v-for="n in 5" :key="'g'+n" x1="0" :y1="n*40" x2="640" :y2="n*40" stroke="#2a2a3a" stroke-width="0.5"/>
                <g v-for="(line, i) in trendLines" :key="i">
                  <polyline
                    :points="line.map((v, j) => (40 + j * ((600) / Math.max(1, trendDates.length - 1))) + ',' + (205 - v * 180)).join(' ')"
                    :stroke="trendColors[i % trendColors.length]"
                    stroke-width="2"
                    fill="none"
                  />
                </g>
              </svg>
            </section>

            <div class="two-col">
              <section class="board-section">
                <h3>登顶占比 · 羁绊（Win Share）</h3>
                <div class="meta-list">
                  <div class="meta-row meta-head">
                    <span class="meta-rank">#</span>
                    <span class="meta-name">羁绊</span>
                    <span class="meta-num">层级</span>
                    <span class="meta-num">占比</span>
                  </div>
                  <div
                    v-for="(t, i) in tftData.metaStats.winShare.traits.slice(0, 15)"
                    :key="t.trait_id"
                    class="meta-row"
                  >
                    <span class="meta-rank" :class="{ top3: i < 3 }">{{ i + 1 }}</span>
                    <span class="meta-name">
                      <img :src="traitIcon(t.trait_id)" :alt="t.trait_name_cn" class="top-icon" />
                      {{ t.trait_name_cn }}
                      <span class="muted tiny">{{ t.trait_name_en }}</span>
                    </span>
                    <span class="meta-num">{{ t.tier != null ? ('T' + t.tier) : '—' }}</span>
                    <span class="meta-num accent">{{ ((t.wins / tftData.metaStats.winShare.sample_size) * 100).toFixed(2) }}%</span>
                  </div>
                </div>
              </section>
              <section class="board-section">
                <h3>前四占比 · 羁绊（Top4 Share）</h3>
                <div class="meta-list">
                  <div class="meta-row meta-head">
                    <span class="meta-rank">#</span>
                    <span class="meta-name">羁绊</span>
                    <span class="meta-num">层级</span>
                    <span class="meta-num">占比</span>
                  </div>
                  <div
                    v-for="(t, i) in tftData.metaStats.top4Share.traits.slice(0, 15)"
                    :key="t.trait_id"
                    class="meta-row"
                  >
                    <span class="meta-rank" :class="{ top3: i < 3 }">{{ i + 1 }}</span>
                    <span class="meta-name">
                      <img :src="traitIcon(t.trait_id)" :alt="t.trait_name_cn" class="top-icon" />
                      {{ t.trait_name_cn }}
                      <span class="muted tiny">{{ t.trait_name_en }}</span>
                    </span>
                    <span class="meta-num">{{ t.tier != null ? ('T' + t.tier) : '—' }}</span>
                    <span class="meta-num accent">{{ ((t.wins / tftData.metaStats.top4Share.sample_size) * 100).toFixed(2) }}%</span>
                  </div>
                </div>
              </section>
            </div>
          </div>

          <!-- 阵容趋势 -->
          <div v-if="metaTab === 'trends'">
            <section class="board-section">
              <h3>阵容 AVP 完整表格（当前补丁）</h3>
              <div class="meta-list">
                <div class="meta-row meta-head">
                  <span class="meta-rank">#</span>
                  <span class="meta-name">阵容</span>
                  <span class="meta-num">当前补丁 AVP</span>
                  <span class="meta-num">Pick Rate</span>
                  <span class="meta-num">± 名次</span>
                  <span class="meta-num">± 出场率</span>
                </div>
                <div
                  v-for="(c, i) in sortedTrendRows"
                  :key="c.slug"
                  class="meta-row comp-row"
                  @click="showCompDetail(c.slug)"
                >
                  <span class="meta-rank" :class="{ top3: i < 3 }">{{ i + 1 }}</span>
                  <span class="meta-name">
                    <img :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
                    {{ compName(c.slug) }}
                  </span>
                  <span class="meta-num">{{ c.avgPlacement }}</span>
                  <span class="meta-num">{{ c.playRate }}</span>
                  <span class="meta-num" :class="c.versionDeltaAvg ? (c.versionDeltaAvg.positive ? 'accent green' : 'accent bad') : ''">{{ c.versionDeltaAvg?.text ?? '—' }}</span>
                  <span class="meta-num" :class="c.versionDeltaPlay ? (c.versionDeltaPlay.positive ? 'accent green' : 'accent bad') : ''">{{ c.versionDeltaPlay?.text ?? '—' }}</span>
                </div>
              </div>
            </section>
            <section class="board-section">
              <h3>平均名次变化 · 上升</h3>
              <div class="meta-list">
                <div class="meta-row meta-head">
                  <span class="meta-rank">#</span>
                  <span class="meta-name">阵容</span>
                  <span class="meta-num">均名次</span>
                  <span class="meta-num">变化</span>
                  <span class="meta-num">出场率</span>
                </div>
                <div v-for="(c, i) in tftData.trends.compositionTrendUp" :key="c.slug" class="meta-row comp-row" @click="showCompDetail(c.slug)">
                  <span class="meta-rank" :class="{ top3: i < 3 }">{{ i + 1 }}</span>
                  <span class="meta-name">
                    <img :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
                    {{ compName(c.slug) }}
                  </span>
                  <span class="meta-num">{{ c.avgPlacement }}</span>
                  <span class="meta-num accent green">{{ c.avgPlacementDelta?.text }}</span>
                  <span class="meta-num">{{ c.playRate }}</span>
                </div>
              </div>
            </section>
            <section class="board-section">
              <h3>平均名次变化 · 下降</h3>
              <div class="meta-list">
                <div class="meta-row meta-head">
                  <span class="meta-rank">#</span>
                  <span class="meta-name">阵容</span>
                  <span class="meta-num">均名次</span>
                  <span class="meta-num">变化</span>
                  <span class="meta-num">出场率</span>
                </div>
                <div v-for="(c, i) in tftData.trends.compositionTrendDown" :key="c.slug" class="meta-row comp-row" @click="showCompDetail(c.slug)">
                  <span class="meta-rank">{{ i + 1 }}</span>
                  <span class="meta-name">
                    <img :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
                    {{ compName(c.slug) }}
                  </span>
                  <span class="meta-num">{{ c.avgPlacement }}</span>
                  <span class="meta-num accent bad">{{ c.avgPlacementDelta?.text }}</span>
                  <span class="meta-num">{{ c.playRate }}</span>
                </div>
              </div>
            </section>
            <section class="board-section">
              <h3>登顶占比变化 · 上升</h3>
              <div class="meta-list">
                <div class="meta-row meta-head">
                  <span class="meta-rank">#</span>
                  <span class="meta-name">阵容</span>
                  <span class="meta-num">登顶</span>
                  <span class="meta-num">前四</span>
                  <span class="meta-num">±登顶</span>
                  <span class="meta-num">±前四</span>
                </div>
                <div v-for="(c, i) in tftData.trends.winshareTrendUp" :key="c.slug" class="meta-row comp-row" @click="showCompDetail(c.slug)">
                  <span class="meta-rank" :class="{ top3: i < 3 }">{{ i + 1 }}</span>
                  <span class="meta-name">
                    <img :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
                    {{ compName(c.slug) }}
                  </span>
                  <span class="meta-num accent">{{ c.winShare }}</span>
                  <span class="meta-num accent">{{ c.top4Share }}</span>
                  <span class="meta-num accent green">{{ c.winShareDelta?.text }}</span>
                  <span class="meta-num accent green">{{ c.top4ShareDelta?.text }}</span>
                </div>
              </div>
            </section>
            <section class="board-section">
              <h3>登顶占比变化 · 下降</h3>
              <div class="meta-list">
                <div class="meta-row meta-head">
                  <span class="meta-rank">#</span>
                  <span class="meta-name">阵容</span>
                  <span class="meta-num">登顶</span>
                  <span class="meta-num">前四</span>
                  <span class="meta-num">±登顶</span>
                  <span class="meta-num">±前四</span>
                </div>
                <div v-for="(c, i) in tftData.trends.winshareTrendDown" :key="c.slug" class="meta-row comp-row" @click="showCompDetail(c.slug)">
                  <span class="meta-rank">{{ i + 1 }}</span>
                  <span class="meta-name">
                    <img :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
                    {{ compName(c.slug) }}
                  </span>
                  <span class="meta-num accent">{{ c.winShare }}</span>
                  <span class="meta-num accent">{{ c.top4Share }}</span>
                  <span class="meta-num accent bad">{{ c.winShareDelta?.text }}</span>
                  <span class="meta-num accent bad">{{ c.top4ShareDelta?.text }}</span>
                </div>
              </div>
            </section>
          </div>

          <!-- 每日数据 -->
          <div v-if="metaTab === 'daily'">
            <div class="tabs tabs-group">
              <span class="tabs-group-label">模式</span>
              <button
                v-for="t in [{ key: 'playRate', label: '24h 出场率' }, { key: 'avp', label: '24h 均名次' }, { key: 'patch', label: '补丁变化' }]"
                :key="t.key"
                class="tab"
                :class="{ active: dailyMode === t.key }"
                @click="dailyMode = t.key as any"
              >
                {{ t.label }}
              </button>
            </div>
            <section class="board-section">
              <h3>每日数据（{{ tftData.dailyStats.versionDeltaWindowHint }}）</h3>
              <div class="meta-list">
                <div class="meta-row meta-head">
                  <span class="meta-name">阵容</span>
                  <span class="meta-num" v-if="dailyMode === 'playRate'">出场率</span>
                  <span class="meta-num" v-if="dailyMode === 'playRate'">版本变化</span>
                  <span class="meta-num" v-if="dailyMode === 'avp'">均名次</span>
                  <span class="meta-num" v-if="dailyMode === 'avp'">名次变化</span>
                  <span class="meta-num" v-if="dailyMode === 'patch'">AVP</span>
                  <span class="meta-num" v-if="dailyMode === 'patch'">出场率</span>
                  <span class="meta-num" v-for="d in tftData.dailyStats.dateColumns" :key="d">{{ d }}</span>
                </div>
                <div v-for="c in tftData.dailyStats.compositionRows" :key="c.slug" class="meta-row comp-row" @click="showCompDetail(c.slug)">
                  <span class="meta-name">
                    <img :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
                    {{ compName(c.slug) }}
                  </span>
                  <span class="meta-num" v-if="dailyMode === 'playRate'">{{ c.playRate }}</span>
                  <span class="meta-num" v-if="dailyMode === 'playRate'" :class="c.versionDeltaPlay ? (c.versionDeltaPlay.positive ? 'accent green' : 'accent bad') : ''">{{ c.versionDeltaPlay?.text }}</span>
                  <span class="meta-num" v-if="dailyMode === 'avp'">{{ c.avgPlacement }}</span>
                  <span class="meta-num" v-if="dailyMode === 'avp'" :class="c.versionDeltaAvg ? (c.versionDeltaAvg.positive ? 'accent green' : 'accent bad') : ''">{{ c.versionDeltaAvg?.text }}</span>
                  <span class="meta-num" v-if="dailyMode === 'patch'">{{ c.avgPlacement }}</span>
                  <span class="meta-num" v-if="dailyMode === 'patch'">{{ c.playRate }}</span>
                  <span v-for="(m, mi) in c.dateMetrics" :key="mi" class="meta-num">
                    {{ m.avgPlacement }}
                  </span>
                </div>
              </div>
            </section>
          </div>

          <!-- 装备梯队 -->
          <div v-if="metaTab === 'tierlist'">
            <div class="tabs tabs-group">
              <span class="tabs-group-label">梯队</span>
              <button class="tab" :class="{ active: tierlistTab === 'artifacts' }" @click="tierlistTab = 'artifacts'">神器</button>
              <button class="tab" :class="{ active: tierlistTab === 'emblems' }" @click="tierlistTab = 'emblems'">纹章</button>
            </div>
            <div class="tierlist-meta">
              <span class="tiny muted">数据范围: 168小时 · 样本≥50 · 段位 Diamond+ · 更新 {{ tftData.itemTierlist.generated_at ? new Date(tftData.itemTierlist.generated_at).toLocaleString("zh-CN") : "—" }}</span>
            </div>

            <section v-if="tierlistTab === 'artifacts'" class="board-section">
              <h3>神器梯队（按强度）</h3>
              <div class="tierlist-list">
                <div v-for="r in tftData.itemTierlist.tabs.artifacts.rows" :key="r.artifact_id" class="tierlist-row">
                  <div class="tierlist-item">
                    <img :src="itemIcon(r.artifact_id)" :alt="r.artifact_id" class="mini-item" />
                    <span class="tierlist-name">{{ itemName(r.artifact_id) }}</span>
                  </div>
                  <div class="tierlist-comps">
                    <div v-for="cc in r.comps" :key="cc.comp_id" class="tierlist-comp" @click="showCompDetail(cc.comp_id)">
                      <img :src="compCardIcon(cc.comp_id)" :alt="compName(cc.comp_id)" class="mini-unit" />
                      <span class="tierlist-comp-name">{{ compName(cc.comp_id) }}</span>
                      <span class="tierlist-comp-strength">{{ cc.item_strength.toFixed(2) }}</span>
                      <img v-if="cc.top_pair" v-for="it in cc.top_pair.item_ids" :key="it" :src="itemIcon(it)" class="mini-item" :title="itemName(it)" />
                    </div>
                  </div>
                </div>
              </div>
            </section>

            <section v-if="tierlistTab === 'emblems'" class="board-section">
              <h3>纹章梯队（按强度）</h3>
              <div class="tierlist-list">
                <div v-for="r in tftData.itemTierlist.tabs.emblems.rows" :key="r.artifact_id" class="tierlist-row">
                  <div class="tierlist-item">
                    <img :src="itemIcon(r.artifact_id)" :alt="r.artifact_id" class="mini-item" />
                    <span class="tierlist-name">{{ itemName(r.artifact_id) }}</span>
                  </div>
                  <div class="tierlist-comps">
                    <div v-for="cc in r.comps" :key="cc.comp_id" class="tierlist-comp" @click="showCompDetail(cc.comp_id)">
                      <img :src="compCardIcon(cc.comp_id)" :alt="compName(cc.comp_id)" class="mini-unit" />
                      <span class="tierlist-comp-name">{{ compName(cc.comp_id) }}</span>
                      <span class="tierlist-comp-strength">{{ cc.item_strength.toFixed(2) }}</span>
                    </div>
                  </div>
                </div>
              </div>
              <div v-if="tftData.itemTierlist.tabs.emblems.essentials?.length" class="tierlist-essentials">
                <h4 class="special-lbl">必需纹章（Essentials）</h4>
                <div class="tierlist-list">
                  <div v-for="e in tftData.itemTierlist.tabs.emblems.essentials" :key="e.artifact_id" class="tierlist-row essentials">
                    <div class="tierlist-item">
                      <img :src="itemIcon(e.artifact_id)" :alt="e.artifact_id" class="mini-item" />
                      <span class="tierlist-name">{{ itemName(e.artifact_id) }}</span>
                    </div>
                    <div class="tierlist-comps">
                      <div v-for="cc in e.comps" :key="cc.comp_id" class="tierlist-comp" @click="showCompDetail(cc.comp_id)">
                        <img :src="compCardIcon(cc.comp_id)" :alt="compName(cc.comp_id)" class="mini-unit" />
                        <span class="tierlist-comp-name">{{ compName(cc.comp_id) }}</span>
                        <span class="tierlist-comp-strength">{{ cc.item_strength.toFixed(4) }}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </section>
          </div>

          <!-- 按弈子 -->
          <div v-if="metaTab === 'unitItems'">
            <section class="board-section">
              <h3>按弈子 · 最佳出装</h3>
              <div class="unit-item-list">
                <div v-for="u in tftData.unitItems" :key="u.unitId" class="unit-item-card">
                  <div class="champ-head">
                    <img :src="unitIcon(u.unitId)" :alt="unitName(u.unitId)" class="champ-icon" />
                    <span class="champ-name">{{ unitName(u.unitId) }}</span>
                    <span class="muted tiny" style="font-size:10px">{{ unitNameEn(u.unitId) }}</span>
                    <span class="muted tiny">${{ u.unitCost }}</span>
                    <span v-if="u.unitPriority != null" class="muted tiny" style="margin-left:auto">投资优先度 {{ u.unitPriority.toFixed(2) }}</span>
                  </div>
                  <div v-if="u.bisItemIds && u.bisItemIds.length" class="unit-bis">
                    <span class="unit-bucket-lbl">BiS</span>
                    <img v-for="it in u.bisItemIds" :key="it" :src="itemIcon(it)" class="mini-item" :title="itemName(it)" />
                  </div>
                  <div class="unit-item-buckets">
                    <div v-if="u.buckets.Core.length" class="unit-bucket">
                      <span class="unit-bucket-lbl">核心</span>
                      <img v-for="it in u.buckets.Core" :key="it.itemId" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId) + ' 必需度 ' + it.necessity.toFixed(2)" />
                    </div>
                    <div v-if="u.buckets.Important.length" class="unit-bucket">
                      <span class="unit-bucket-lbl">重要</span>
                      <img v-for="it in u.buckets.Important" :key="it.itemId" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId)" />
                    </div>
                    <div v-if="u.buckets.Optional.length" class="unit-bucket">
                      <span class="unit-bucket-lbl">可选</span>
                      <img v-for="it in u.buckets.Optional" :key="it.itemId" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId)" />
                    </div>
                    <div v-if="u.buckets.HighFrequency.length" class="unit-bucket">
                      <span class="unit-bucket-lbl">高频</span>
                      <img v-for="it in u.buckets.HighFrequency" :key="it.itemId" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId) + ' 需求度 ' + it.necessity.toFixed(2)" />
                    </div>
                  </div>
                  <div v-if="u.artifacts.length" class="unit-artifacts">
                    <span class="muted tiny">神器:</span>
                    <img v-for="a in u.artifacts" :key="a.itemId" :src="itemIcon(a.itemId)" class="mini-item" :title="itemName(a.itemId) + ' 均 ' + a.avgPlacement.toFixed(2)" />
                  </div>
                  <div v-if="u.coreCompStrengths.length" class="unit-comps">
                    <span v-for="cc in u.coreCompStrengths" :key="cc.comp_key" class="trait-chip" @click="showCompDetail(cc.comp_key)">{{ compName(cc.comp_key) }}</span>
                  </div>
                </div>
              </div>
            </section>
          </div>

          <!-- 按装备 -->
          <div v-if="metaTab === 'itemGuide'">
            <div class="tabs tabs-group">
              <span class="tabs-group-label">装备</span>
              <button class="tab" :class="{ active: itemGuideTab === 'items' }" @click="itemGuideTab = 'items'">合成</button>
              <button class="tab" :class="{ active: itemGuideTab === 'artifacts' }" @click="itemGuideTab = 'artifacts'">神器</button>
              <button class="tab" :class="{ active: itemGuideTab === 'emblems' }" @click="itemGuideTab = 'emblems'">纹章</button>
            </div>

            <section class="board-section">
              <h3>按装备 · 适用弈子</h3>
              <div class="item-search">
                <input v-model="itemSearch" class="search-input" placeholder="搜索装备名称…" />
                <span class="muted tiny">{{ currentItemGuide.length }} 件</span>
              </div>
              <div class="item-guide-list">
                <div v-for="g in currentItemGuide" :key="g.itemId" class="item-guide-card">
                  <div class="champ-head">
                    <img :src="itemIcon(g.itemId)" :alt="itemName(g.itemId)" class="champ-icon" />
                    <span class="champ-name">{{ itemName(g.itemId) }}</span>
                    <span class="muted tiny" style="font-size:10px">{{ g.itemName }}</span>
                  </div>
                  <div class="item-guide-buckets">
                    <div v-for="(units, cost) in sortedBuckets(g.costBuckets)" :key="cost" class="guide-bucket">
                      <span class="unit-bucket-lbl">{{ cost }}费</span>
                      <img v-for="u in units" :key="u.unitId" :src="unitIcon(u.unitId)" class="mini-unit" :title="unitName(u.unitId) + ' 必需度 ' + u.necessity.toFixed(3)" />
                    </div>
                  </div>
                  <div v-if="g.compositions && g.compositions.length" class="unit-comps">
                    <span v-for="cc in g.compositions" :key="cc.slug" class="trait-chip" @click="showCompDetail(cc.slug)">{{ compName(cc.slug) }}</span>
                  </div>
                </div>
              </div>
            </section>
          </div>
        </template>
        <p v-else-if="!metaLoading" class="empty">暂无数据。</p>
      </template>

      <!-- 列表视图 -->
      <template v-else-if="!selected">
        <header class="topbar">
          <h2>近期战绩</h2>
          <button @click="sync" :disabled="loading">
            {{ loading ? "同步中…" : "同步战绩" }}
          </button>
        </header>

        <p v-if="lastSync" class="hint">{{ lastSync }}</p>
        <p v-if="error" class="error">{{ error }}</p>

        <p v-if="matches.length === 0 && !loading" class="empty">
          还没有记录对局。打开客户端登录后，点击「同步战绩」。
        </p>

        <table v-if="matches.length > 0" class="match-table">
          <thead>
            <tr>
              <th>名次</th>
              <th>模式</th>
              <th>日期</th>
              <th>等级</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="m in matches" :key="m.game_id" @click="openMatch(m.game_id)">
              <td class="placement" :class="`p${m.placement}`">#{{ m.placement }}</td>
              <td>{{ m.game_mode ?? "—" }}</td>
              <td>{{ fmtDate(m.game_datetime) }}</td>
              <td>{{ m.level ?? "—" }}</td>
            </tr>
          </tbody>
        </table>
      </template>

      <!-- 详情视图 -->
      <template v-else>
        <header class="topbar">
          <button class="ghost" @click="back">← 返回</button>
          <h2 class="detail-title">
            <span class="placement" :class="`p${selected.summary.placement}`">
              #{{ selected.summary.placement }}
            </span>
            {{ selected.summary.game_mode ?? "对局" }}
          </h2>
          <span class="muted">{{ fmtDate(selected.summary.game_datetime) }}</span>
        </header>

        <div v-if="detailLoading" class="empty">加载中…</div>

        <template v-else>
          <section class="board-section">
            <h3>羁绊</h3>
            <div class="traits">
              <div v-for="t in selected.traits" :key="t.id" class="trait">
                <img v-if="t.icon_url" :src="t.icon_url" :alt="t.name" class="trait-icon" />
                <span class="trait-name">{{ t.name || t.id }}</span>
                <span class="trait-tier">{{ t.tier_current ?? 0 }}/{{ t.tier_total ?? 0 }}</span>
              </div>
            </div>
          </section>

          <section class="board-section">
            <h3>最终阵容</h3>
            <div class="board">
              <div
                v-for="u in selected.units"
                :key="u.character_id + '-' + u.tier + '-' + u.items.map((i) => i.id).join(',')"
                class="unit-card"
              >
                <div
                  class="unit-icon-wrap"
                  :style="{ borderColor: costColor(u.cost) }"
                >
                  <img v-if="u.icon_url" :src="u.icon_url" :alt="u.name" class="unit-icon" />
                  <div v-if="u.tier" class="stars">
                    {{ "★".repeat(u.tier) }}
                  </div>
                </div>
                <div class="unit-name">{{ u.name || u.character_id }}</div>
                <div class="unit-items">
                  <div v-for="it in u.items" :key="it.id" class="item" :title="it.name">
                    <img v-if="it.icon_url" :src="it.icon_url" :alt="it.name" />
                    <span v-else class="item-fallback">{{ it.name || it.id }}</span>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </template>
      </template>
    </main>
    </div>
  </div>
</template>

<style>
:root {
  --bg: #0e0e15;
  --bg-side: #13131c;
  --card: #1b1b26;
  --card-hover: #222230;
  --border: #2a2a3c;
  --text: #e7e7f0;
  --dim: #9a9aad;
  --faint: #6c6c80;
  --gold: #f0c24b;
  --gold-bright: #f7d878;
  --purple: #8b6cf0;
  --green: #45d483;
  --red: #f0716f;
  --blue: #4d9de0;
  --radius: 10px;
}

* {
  box-sizing: border-box;
}

html,
body {
  margin: 0;
  height: 100%;
}

body {
  font-family: "Segoe UI", "Microsoft YaHei", Inter, system-ui, sans-serif;
  font-size: 14px;
  color: var(--text);
  background:
    radial-gradient(1200px 600px at 20% -10%, rgba(139, 108, 240, 0.08), transparent),
    radial-gradient(1000px 500px at 100% 0%, rgba(240, 194, 75, 0.05), transparent),
    var(--bg);
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
}

::-webkit-scrollbar {
  width: 10px;
  height: 10px;
}

::-webkit-scrollbar-thumb {
  background: #2e2e40;
  border-radius: 6px;
  border: 2px solid transparent;
  background-clip: content-box;
}

::-webkit-scrollbar-thumb:hover {
  background: #3c3c52;
  background-clip: content-box;
}

.window {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.titlebar {
  height: 38px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px 0 14px;
  background: #12121a;
  border-bottom: 1px solid var(--border);
  -webkit-app-region: drag;
  user-select: none;
}

.titlebar-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.titlebar-logo {
  color: var(--gold);
  font-size: 14px;
}

.titlebar-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--dim);
  letter-spacing: 0.3px;
}

.titlebar-controls {
  display: flex;
  gap: 2px;
  -webkit-app-region: no-drag;
}

.titlebar-btn {
  width: 36px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: 6px;
  color: var(--dim);
  font-size: 13px;
  cursor: pointer;
  padding: 0;
}

.titlebar-btn:hover {
  background: rgba(255, 255, 255, 0.06);
  color: var(--text);
}

.titlebar-btn.close:hover {
  background: #c0392b;
  color: #fff;
}

.shell {
  display: flex;
  flex: 1;
  min-height: 0;
}

.sidebar {
  width: 212px;
  flex-shrink: 0;
  background: var(--bg-side);
  display: flex;
  flex-direction: column;
  padding: 20px 14px;
  border-right: 1px solid var(--border);
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 4px;
}

.brand-logo {
  width: 34px;
  height: 34px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  color: var(--gold);
  background: linear-gradient(135deg, #2a2414, #1c1c28);
  border: 1px solid rgba(240, 194, 75, 0.35);
  border-radius: 9px;
  box-shadow: 0 0 14px rgba(240, 194, 75, 0.15);
}

.brand h1 {
  font-size: 16px;
  margin: 0;
  font-weight: 700;
  letter-spacing: 0.2px;
}

.brand-accent {
  color: var(--gold);
}

.brand .version {
  color: var(--faint);
  font-size: 11px;
}

nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 26px;
}

.nav-group {
  position: relative;
}

.nav-group:hover > .nav-sub {
  display: block;
  animation: navFade 0.15s ease-out;
}

.nav-group:hover .nav-caret {
  transform: rotate(180deg);
  color: var(--gold);
}

.nav-caret {
  margin-left: auto;
  font-size: 10px;
  color: var(--faint);
  transition: transform 0.2s, color 0.2s;
}

.nav-sub {
  display: none;
  position: absolute;
  left: calc(100% + 6px);
  top: 0;
  min-width: 150px;
  background: #1a1a25;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 6px;
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.5);
  z-index: 200;
}

.nav-sub-item {
  display: block;
  padding: 8px 12px;
  border-radius: 6px;
  color: var(--dim);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
}

.nav-sub-item:hover {
  background: rgba(240, 194, 75, 0.12);
  color: var(--gold-bright);
}

@keyframes navFade {
  from {
    opacity: 0;
    transform: translateX(-4px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

.nav-item {
  position: relative;
  display: flex;
  align-items: center;
  padding: 9px 12px;
  border-radius: 8px;
  color: var(--dim);
  cursor: pointer;
  font-size: 14px;
  transition: background 0.15s, color 0.15s;
}

.nav-ico {
  display: inline-block;
  width: 20px;
  margin-right: 8px;
  color: var(--faint);
  font-size: 13px;
  text-align: center;
}

.nav-item.active .nav-ico {
  color: var(--gold);
}

.nav-item:hover {
  background: rgba(255, 255, 255, 0.04);
  color: var(--text);
}

.nav-item.active {
  background: linear-gradient(90deg, rgba(240, 194, 75, 0.14), rgba(240, 194, 75, 0.04));
  color: var(--gold-bright);
  font-weight: 600;
}

.nav-item.active::before {
  content: "";
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 18px;
  border-radius: 2px;
  background: var(--gold);
}

.sidebar-footer {
  margin-top: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.lcu-status {
  font-size: 12px;
  padding: 7px 9px;
  border-radius: 8px;
  line-height: 1.3;
}

.lcu-status.ok {
  background: rgba(69, 212, 131, 0.12);
  color: var(--green);
  border: 1px solid rgba(69, 212, 131, 0.25);
}

.lcu-status.off {
  background: rgba(240, 113, 111, 0.1);
  color: var(--red);
  border: 1px solid rgba(240, 113, 111, 0.2);
}

.lcu-status em {
  font-style: normal;
  opacity: 0.7;
  margin-left: 6px;
}

.lcu-source {
  font-size: 11px;
  color: var(--faint);
}

.content {
  flex: 1;
  overflow-y: auto;
  padding: 28px 32px;
  min-width: 0;
}

.topbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 20px;
}

.topbar h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 700;
  letter-spacing: 0.2px;
}

.detail-title {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 10px;
}

.muted {
  color: var(--faint);
  font-size: 13px;
}

button {
  background: linear-gradient(135deg, var(--gold), #d9a92e);
  color: #1c1504;
  border: none;
  border-radius: 8px;
  padding: 9px 16px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
  transition: filter 0.15s, transform 0.05s;
}

button:hover {
  filter: brightness(1.08);
}

button:active {
  transform: translateY(1px);
}

button:disabled {
  opacity: 0.5;
  cursor: default;
  filter: none;
}

button.ghost {
  background: var(--card);
  color: var(--dim);
  border: 1px solid var(--border);
  font-weight: 500;
}

button.ghost:hover {
  background: var(--card-hover);
  color: var(--text);
  filter: none;
}

button.small {
  padding: 7px 10px;
  font-size: 12px;
}

.hint {
  color: var(--green);
  font-size: 13px;
}

.error {
  color: var(--red);
  font-size: 13px;
}

.empty {
  color: var(--faint);
  font-size: 14px;
  padding: 40px 0;
  text-align: center;
}

.match-table {
  width: 100%;
  border-collapse: collapse;
}

.match-table th,
.match-table td {
  text-align: left;
  padding: 12px 14px;
  border-bottom: 1px solid var(--border);
}

.match-table tbody tr {
  cursor: pointer;
  transition: background 0.12s;
}

.match-table tbody tr:hover {
  background: var(--card-hover);
}

.match-table th {
  color: var(--faint);
  font-weight: 600;
  font-size: 12px;
  letter-spacing: 0.5px;
  text-transform: uppercase;
}

.placement {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 34px;
  height: 26px;
  padding: 0 8px;
  border-radius: 7px;
  font-weight: 700;
  font-size: 13px;
  background: #2a2a3a;
  color: var(--dim);
}

.placement.p1 {
  background: rgba(240, 194, 75, 0.16);
  color: var(--gold-bright);
}

.placement.p2 {
  background: rgba(200, 205, 215, 0.12);
  color: #c9cfda;
}

.placement.p3 {
  background: rgba(205, 127, 50, 0.16);
  color: #e0a469;
}

.placement.p4 {
  background: rgba(69, 212, 131, 0.14);
  color: var(--green);
}

.board-section {
  margin-bottom: 28px;
}

.board-section h3 {
  font-size: 13px;
  color: var(--faint);
  margin: 0 0 14px;
  font-weight: 600;
  letter-spacing: 0.5px;
  text-transform: uppercase;
}

.traits {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.trait {
  display: flex;
  align-items: center;
  gap: 7px;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 7px 11px;
}

.trait-icon {
  width: 22px;
  height: 22px;
  border-radius: 4px;
}

.trait-name {
  font-size: 13px;
}

.trait-tier {
  color: var(--green);
  font-size: 12px;
  font-weight: 600;
}

.board {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
  gap: 14px;
}

.unit-card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 12px 10px 10px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 7px;
  transition: background 0.15s, transform 0.1s;
}

.unit-card:hover {
  background: var(--card-hover);
  transform: translateY(-2px);
}

.unit-icon-wrap {
  position: relative;
  width: 66px;
  height: 66px;
  border: 2px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
  background: #101018;
}

.unit-icon {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.stars {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  text-align: center;
  font-size: 9px;
  letter-spacing: 0.5px;
  color: var(--gold);
  background: linear-gradient(transparent, rgba(0, 0, 0, 0.85));
  line-height: 1.6;
  text-shadow: 0 0 4px rgba(240, 194, 75, 0.6);
}

.unit-name {
  font-size: 12px;
  text-align: center;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.unit-sub {
  font-size: 11px;
  color: var(--dim);
}

.comp-row {
  cursor: pointer;
  transition: background 0.12s;
}

.comp-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 12px;
}

.comp-card {
  display: flex;
  align-items: center;
  gap: 12px;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 12px;
  cursor: pointer;
  transition: background 0.12s, transform 0.1s;
}

.comp-card:hover {
  background: var(--card-hover);
  transform: translateY(-2px);
}

.comp-card-rank {
  font-size: 16px;
  font-weight: 700;
  color: var(--dim);
  min-width: 28px;
}

.comp-card-rank.top3 {
  color: var(--gold);
}

.comp-card-icon {
  width: 44px;
  height: 44px;
  border-radius: 8px;
  background: #101018;
}

.comp-card-body {
  min-width: 0;
}

.comp-card-name {
  font-size: 14px;
  font-weight: 600;
}

.comp-card-stats {
  display: flex;
  gap: 10px;
  font-size: 12px;
  margin-top: 3px;
}

.comp-card-stats .accent {
  color: var(--gold-bright);
}

.comp-card-stats .green {
  color: var(--green);
}

.cost-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
  gap: 8px;
}

.cost-group {
  margin-bottom: 16px;
}

.cost-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--dim);
  margin-bottom: 8px;
}

.cost-grid .champ-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 8px;
  cursor: pointer;
}

.cost-grid .champ-head {
  flex-direction: column;
  gap: 6px;
  text-align: center;
}

.env-overview .comp-stat-cards {
  margin-bottom: 24px;
}

.comp-sub {
  font-size: 11px;
  color: var(--faint);
  margin-left: 6px;
}

.trend-chart-wrap {
  margin-bottom: 24px;
}

.trend-legend {
  display: flex;
  gap: 14px;
  margin-bottom: 10px;
  flex-wrap: wrap;
}

.trend-legend-item {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: var(--dim);
}

.trend-legend-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}

.trend-svg {
  width: 100%;
  max-height: 220px;
}

.cost-bars {
  display: flex;
  flex-direction: column;
  gap: 7px;
  max-width: 480px;
}

.cost-bar-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.cost-bar-label {
  width: 34px;
  font-size: 12px;
  font-weight: 600;
}

.cost-bar-bg {
  flex: 1;
  height: 14px;
  background: #26262c;
  border-radius: 5px;
  overflow: hidden;
}

.cost-bar {
  height: 100%;
  border-radius: 5px;
  transition: width 0.3s;
}

.cost-bar-val {
  min-width: 46px;
  text-align: right;
  font-size: 12px;
  color: var(--dim);
}

.comp-row:hover {
  background: var(--card-hover);
}

.unit-items {
  display: flex;
  gap: 4px;
  min-height: 22px;
  flex-wrap: wrap;
  justify-content: center;
}

.item {
  width: 22px;
  height: 22px;
  background: #101018;
  border-radius: 4px;
  overflow: hidden;
}

.item img {
  width: 100%;
  height: 100%;
  display: block;
}

.item-fallback {
  font-size: 9px;
  color: var(--faint);
}

.stat-cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 14px;
  margin-bottom: 28px;
}

.stat-card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 18px;
  position: relative;
  overflow: hidden;
}

.stat-card::after {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: linear-gradient(90deg, var(--gold), var(--purple));
  opacity: 0.6;
}

.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: var(--text);
  letter-spacing: 0.5px;
}

.stat-label {
  color: var(--faint);
  font-size: 12px;
  margin-top: 5px;
}

.dist {
  display: flex;
  flex-direction: column;
  gap: 7px;
  max-width: 520px;
}

.dist-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.dist-rank {
  width: 26px;
  color: var(--dim);
  font-size: 12px;
  text-align: right;
}

.dist-bar-bg {
  flex: 1;
  background: #23232f;
  border-radius: 5px;
  height: 16px;
  overflow: hidden;
}

.dist-bar {
  height: 100%;
  background: linear-gradient(90deg, var(--gold), var(--purple));
  border-radius: 5px;
  transition: width 0.4s ease;
}

.dist-count {
  width: 28px;
  color: var(--dim);
  font-size: 12px;
}

.top-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 9px;
}

.top-item {
  display: flex;
  align-items: center;
  gap: 9px;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 9px 11px;
  transition: background 0.12s;
}

.top-item:hover {
  background: var(--card-hover);
}

.top-icon {
  width: 30px;
  height: 30px;
  border-radius: 6px;
  background: #101018;
}

.top-name {
  flex: 1;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.top-count {
  color: var(--faint);
  font-size: 12px;
}

.toast {
  position: fixed;
  top: 18px;
  right: 18px;
  background: rgba(69, 212, 131, 0.14);
  color: var(--green);
  border: 1px solid rgba(69, 212, 131, 0.3);
  border-radius: 10px;
  padding: 12px 16px;
  font-size: 14px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  z-index: 100;
  backdrop-filter: blur(8px);
}

/* 悬浮窗 */
.overlay {
  position: relative;
  height: 100vh;
  background: rgba(19, 19, 28, 0.95);
  color: var(--text);
  padding: 13px 15px;
  border: 1px solid var(--border);
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  gap: 11px;
  overflow: hidden;
  user-select: none;
  backdrop-filter: blur(6px);
}

.overlay-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--green);
  box-shadow: 0 0 8px var(--green);
}

.overlay-title {
  font-weight: 700;
  font-size: 14px;
}

.overlay-avg {
  margin-left: auto;
  color: var(--gold);
  font-size: 12px;
}

.overlay-rank {
  font-size: 12px;
  font-weight: 700;
  color: var(--gold-bright);
}

.overlay-lp {
  margin-left: 6px;
  color: var(--text);
  font-weight: 600;
}

.overlay-winrate {
  margin-left: 6px;
  color: var(--green);
  font-weight: 600;
}

.overlay-close {
  background: transparent;
  color: var(--faint);
  border: none;
  font-size: 14px;
  padding: 0 4px;
  cursor: pointer;
  -webkit-app-region: no-drag;
}

.overlay-close:hover {
  color: var(--text);
}

.overlay-places {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.place-badge {
  width: 26px;
  height: 26px;
  border-radius: 7px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 13px;
  font-weight: 700;
  background: #2a2a3a;
  color: var(--dim);
}

.place-badge.p1 {
  background: rgba(240, 194, 75, 0.18);
  color: var(--gold-bright);
}

.place-badge.p2 {
  background: rgba(200, 205, 215, 0.14);
  color: #c9cfda;
}

.place-badge.p3 {
  background: rgba(205, 127, 50, 0.18);
  color: #e0a469;
}

.overlay-empty {
  color: var(--faint);
  font-size: 12px;
}

.overlay-footer {
  color: var(--faint);
  font-size: 11px;
}

.search {
  background: var(--card);
  border: 1px solid var(--border);
  color: var(--text);
  border-radius: 8px;
  padding: 9px 13px;
  font-size: 13px;
  min-width: 220px;
  outline: none;
  transition: border-color 0.15s;
}

.search:focus {
  border-color: var(--gold);
}

.tabs {
  display: flex;
  gap: 6px;
  margin-bottom: 22px;
}

.tab {
  background: var(--card);
  color: var(--dim);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 16px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  transition: all 0.15s;
}

.tab:hover {
  color: var(--text);
  background: var(--card-hover);
}

.tab.active {
  background: linear-gradient(135deg, var(--gold), #d9a92e);
  color: #1c1504;
  border-color: transparent;
  font-weight: 600;
}

.component-list {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}

.component {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 5px;
  width: 64px;
}

.component img {
  width: 38px;
  height: 38px;
  background: #101018;
  border-radius: 6px;
  padding: 2px;
}

.component-name {
  font-size: 11px;
  color: var(--dim);
  text-align: center;
}

.craft-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
  gap: 10px;
}

.craft-item {
  display: flex;
  gap: 11px;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 11px;
  align-items: center;
  transition: background 0.12s;
}

.craft-item:hover {
  background: var(--card-hover);
}

.craft-icon {
  width: 38px;
  height: 38px;
  background: #101018;
  border-radius: 6px;
}

.craft-body {
  min-width: 0;
}

.craft-name {
  font-size: 13px;
  margin-bottom: 5px;
}

.craft-recipe {
  display: flex;
  gap: 5px;
  align-items: center;
}

.recipe-icon {
  width: 22px;
  height: 22px;
  background: #101018;
  border-radius: 4px;
}

.recipe-text {
  font-size: 11px;
  color: var(--faint);
}

.champ-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 11px;
}

.champ-card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 13px;
  transition: background 0.12s;
}

.champ-card:hover {
  background: var(--card-hover);
}

.champ-head {
  display: flex;
  gap: 11px;
  align-items: center;
  margin-bottom: 9px;
}

.champ-icon {
  width: 46px;
  height: 46px;
  border-radius: 8px;
  background: #101018;
}

.champ-name {
  font-size: 15px;
  font-weight: 600;
}

.cost {
  color: var(--gold);
  margin-right: 4px;
  letter-spacing: 1px;
}

.champ-traits {
  font-size: 12px;
  color: var(--purple);
  margin-top: 3px;
}

.champ-stats {
  font-size: 12px;
  color: var(--dim);
  margin-bottom: 7px;
}

.champ-ability {
  font-size: 12px;
  line-height: 1.5;
}

.ability-name {
  color: var(--purple);
  font-weight: 600;
  margin-right: 4px;
}

.ability-desc {
  color: var(--faint);
}

.augment-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 10px;
}

.augment-card {
  display: flex;
  gap: 11px;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 11px;
  align-items: flex-start;
}

.augment-card:hover {
  background: var(--card-hover);
}

.trait-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 11px;
}

.trait-card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 13px;
  transition: background 0.12s;
}

.trait-card:hover {
  background: var(--card-hover);
}

.trait-tiers {
  display: flex;
  gap: 5px;
  flex-wrap: wrap;
  margin-bottom: 9px;
}

.tier-pill {
  background: rgba(69, 212, 131, 0.12);
  color: var(--green);
  font-size: 11px;
  font-weight: 600;
  border-radius: 5px;
  padding: 2px 7px;
}

.trait-units {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 10px;
}

.trait-unit {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  overflow: hidden;
  background: #101018;
}

.trait-unit img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.odds-note {
  color: var(--faint);
  font-size: 12px;
  margin-bottom: 10px;
}

.odds-table {
  width: 100%;
  max-width: 560px;
  border-collapse: collapse;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
}

.odds-table th,
.odds-table td {
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  text-align: center;
}

.odds-table thead th {
  background: #222230;
  color: var(--faint);
  font-weight: 600;
  font-size: 12px;
}

.odds-table tbody tr:last-child td {
  border-bottom: none;
}

.odds-table tbody tr:hover {
  background: var(--card-hover);
}

.rule-section {
  margin-bottom: 22px;
}

.rule-section h3 {
  font-size: 13px;
  color: var(--faint);
  font-weight: 600;
  letter-spacing: 0.5px;
  margin: 0 0 8px;
  text-transform: none;
}

.odds-table td.rule-key {
  color: var(--gold-bright);
  font-weight: 600;
  text-align: left;
  width: 40%;
  border-right: 1px solid var(--border);
}

.meta-key {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
}

.meta-list {
  display: flex;
  flex-direction: column;
  margin-top: 16px;
}

.meta-row {
  display: grid;
  grid-template-columns: 40px 1fr 70px 90px 90px;
  align-items: center;
  gap: 10px;
  padding: 11px 14px;
  border-bottom: 1px solid var(--border);
}

.meta-row:not(.meta-head):hover {
  background: var(--card-hover);
}

.meta-head {
  color: var(--faint);
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--border);
}

.meta-rank {
  font-weight: 700;
  color: var(--dim);
}

.counter-name {
  font-weight: 600;
}

.counter-delta {
  margin-left: 8px;
  font-size: 12px;
  padding: 2px 7px;
  border-radius: 4px;
}

.counter-delta.bad {
  background: rgba(240, 85, 85, 0.12);
  color: #f55a5a;
}

.daily-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.daily-card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 14px;
}

.daily-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}

.daily-row:last-child {
  border-bottom: none;
}

.trend-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.trend-card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 14px;
}

.trend-bars {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.trend-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.trend-name {
  width: 110px;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 0;
}

.trend-bar-bg {
  flex: 1;
  height: 12px;
  background: #26262c;
  border-radius: 4px;
  overflow: hidden;
}

.trend-bar {
  height: 100%;
  background: linear-gradient(90deg, var(--gold), var(--purple));
  border-radius: 4px;
}

.trend-val {
  min-width: 50px;
  text-align: right;
  font-size: 12px;
  color: var(--dim);
}

.meta-rank.top3 {
  color: var(--gold);
}

.meta-name {
  display: flex;
  align-items: center;
  gap: 9px;
  font-weight: 600;
}

.meta-num {
  text-align: right;
  color: var(--dim);
}

.meta-num.accent {
  color: var(--gold-bright);
  font-weight: 600;
}

.meta-num.accent.green {
  color: var(--green);
}

.hero-item-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 11px;
  margin-top: 8px;
}

.hero-item-card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 13px;
}

.hero-item-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 0;
  border-bottom: 1px solid var(--border);
}

.hero-item-row:last-child {
  border-bottom: none;
}

.hero-item-row .meta-num {
  min-width: 40px;
}

/* 推荐阵容 */
.recommend-list {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.recommend-card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px;
}

.recommend-head {
  margin-bottom: 12px;
  font-size: 16px;
}

.recommend-traits {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 14px;
}

.trait-chip {
  background: rgba(69, 212, 131, 0.12);
  color: var(--green);
  border: 1px solid rgba(69, 212, 131, 0.25);
  padding: 3px 9px;
  border-radius: 6px;
  font-size: 12px;
}

.recommend-units {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.recommend-unit {
  width: 72px;
  text-align: center;
}

.recommend-unit-icon {
  position: relative;
  width: 58px;
  height: 58px;
  margin: 0 auto;
  border-radius: 8px;
  overflow: hidden;
  background: #101018;
  border: 1px solid var(--border);
}

.recommend-unit-icon img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.recommend-star {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  font-size: 9px;
  color: var(--gold);
  background: rgba(0, 0, 0, 0.7);
  line-height: 1.5;
  text-shadow: 0 0 4px rgba(240, 194, 75, 0.6);
}

.recommend-unit-items {
  display: flex;
  justify-content: center;
  gap: 3px;
  margin: 4px 0 2px;
  min-height: 18px;
}

.recommend-item {
  width: 18px;
  height: 18px;
  border-radius: 3px;
  background: #101018;
}

.recommend-unit-name {
  font-size: 11px;
  color: var(--dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ===== 阵容一览 ===== */
.comp-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 12px;
}

.comp-card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 12px;
  cursor: pointer;
  transition: border-color 0.15s, transform 0.15s;
}
.comp-card:hover {
  border-color: var(--gold);
  transform: translateY(-2px);
}

.comp-card-head {
  display: flex;
  align-items: center;
  gap: 10px;
}
.comp-card-icon {
  width: 44px;
  height: 44px;
  border-radius: 8px;
  object-fit: cover;
  background: #101018;
  border: 2px solid #2a2a3a;
}
.comp-card-title {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.comp-card-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}
.comp-card-en {
  font-size: 11px;
}
.comp-cost-tag {
  font-size: 12px;
  font-weight: 700;
  color: #f5b23e;
  background: rgba(245, 178, 62, 0.12);
  border: 1px solid rgba(245, 178, 62, 0.35);
  border-radius: 6px;
  padding: 2px 6px;
  white-space: nowrap;
}

.comp-card-stats {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}
.comp-card-stat {
  flex: 1;
  background: #101018;
  border-radius: 8px;
  padding: 6px 4px;
  text-align: center;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.comp-card-val {
  font-size: 13px;
  font-weight: 600;
  color: var(--green);
}
.comp-card-lbl {
  font-size: 10px;
  color: var(--faint);
}

.comp-card-delta {
  margin-top: 8px;
  font-size: 11px;
  color: var(--faint);
}
.comp-card-delta.up {
  color: var(--green);
}
.comp-card-delta.down {
  color: var(--red);
}

.two-col {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}
@media (max-width: 1000px) {
  .two-col {
    grid-template-columns: 1fr;
  }
}

.en-name {
  font-size: 12px;
  font-weight: 400;
}
.tiny {
  font-size: 10px;
}

/* ===== 阵容详情 ===== */
.detail-board {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}
.detail-unit {
  width: 108px;
  background: #101018;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 8px;
  text-align: center;
}
.detail-unit-icon-wrap {
  width: 64px;
  height: 64px;
  margin: 0 auto;
  border-radius: 50%;
  overflow: hidden;
  border: 3px solid var(--green);
  background: #101018;
}
.detail-unit-icon-wrap.cost1 { border-color: #9aa4af; }
.detail-unit-icon-wrap.cost2 { border-color: #35c66b; }
.detail-unit-icon-wrap.cost3 { border-color: #3fa8f4; }
.detail-unit-icon-wrap.cost4 { border-color: #c26bf1; }
.detail-unit-icon-wrap.cost5 { border-color: #f5b23e; }
.detail-unit-icon-wrap img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.detail-unit-name {
  margin-top: 6px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
}
.detail-unit-en {
  font-size: 10px;
}
.detail-builds {
  margin-top: 6px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.detail-build {
  display: flex;
  justify-content: center;
  gap: 3px;
}

.mini-item {
  width: 22px;
  height: 22px;
  border-radius: 4px;
  object-fit: cover;
  background: #0a0a10;
  border: 1px solid var(--border);
}
.mini-unit {
  width: 30px;
  height: 30px;
  border-radius: 6px;
  object-fit: cover;
  background: #0a0a10;
  border: 1px solid var(--border);
}

.variant-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.variant-card {
  background: #101018;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 10px;
}
.variant-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.variant-name {
  font-size: 13px;
  font-weight: 600;
}
.badge {
  font-size: 10px;
  color: #101018;
  background: var(--gold);
  border-radius: 4px;
  padding: 1px 5px;
  margin-left: 6px;
}
.variant-units {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.variant-traits {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 8px;
}
.trait-icon-sm {
  width: 18px;
  height: 18px;
  border-radius: 4px;
  vertical-align: middle;
  margin-right: 2px;
}

.priority-table .meta-row {
  grid-template-columns: 2fr 1fr 1fr 1fr;
}
.prio-unit {
  font-size: 12px;
}

/* 优先级矩阵明细 */
.prio-row {
  background: #101018;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 10px;
  margin-bottom: 8px;
}
.prio-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.prio-buckets {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
.prio-bucket {
  display: flex;
  align-items: flex-start;
  gap: 6px;
}
.prio-bucket-lbl {
  font-size: 11px;
  color: var(--faint);
  padding-top: 3px;
  width: 32px;
  flex-shrink: 0;
}
.prio-items {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.prio-item {
  position: relative;
  display: inline-flex;
  flex-direction: column;
  align-items: center;
}
.prio-item .mini-item {
  width: 26px;
  height: 26px;
}
.prio-item-nec {
  font-size: 9px;
  color: var(--green);
  line-height: 1.2;
}

.matchup-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.matchup-row {
  display: grid;
  grid-template-columns: 90px 1fr auto;
  gap: 10px;
  align-items: center;
  padding: 8px 10px;
  border-radius: 8px;
  font-size: 12px;
  border: 1px solid var(--border);
}
.matchup-row.good { background: rgba(69, 212, 131, 0.08); color: var(--green); }
.matchup-row.bad { background: rgba(240, 113, 111, 0.08); color: var(--red); }
.matchup-row.neutral { background: rgba(157, 164, 175, 0.08); color: var(--dim); }

.artifact-picks {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.artifact-pick {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
  background: #101018;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 10px;
}
.artifact-pick-main {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.artifact-name {
  font-size: 12px;
  font-weight: 600;
}
.artifact-carrier {
  font-size: 12px;
  color: var(--dim);
}
.artifact-pair {
  display: flex;
  gap: 3px;
}

/* ===== 按弈子 ===== */
.unit-item-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 10px;
}
.unit-item-card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 10px;
}
.unit-item-buckets {
  display: flex;
  flex-direction: column;
  gap: 5px;
  margin-top: 8px;
}
.unit-bucket {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
}
.unit-bucket-lbl {
  font-size: 10px;
  color: var(--faint);
  width: 34px;
  flex-shrink: 0;
}
.unit-artifacts {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
  margin-top: 6px;
}
.unit-comps {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 6px;
}

/* ===== 装备梯队 ===== */
.tierlist-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.tierlist-row {
  background: #101018;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 10px;
}
.tierlist-item {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.tierlist-name {
  font-size: 13px;
  font-weight: 600;
}
.tierlist-comps {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.tierlist-comp {
  display: flex;
  align-items: center;
  gap: 6px;
  background: #0a0a10;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 4px 8px;
  cursor: pointer;
}
.tierlist-comp:hover {
  border-color: var(--gold);
}
.tierlist-comp-name {
  font-size: 12px;
}
.tierlist-comp-strength {
  font-size: 11px;
  color: var(--green);
  font-weight: 600;
}

/* ===== 按装备 ===== */
.item-guide-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 10px;
}
.item-guide-card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 10px;
}
.item-guide-buckets {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 8px;
}
.guide-bucket {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
}

/* ===== 特殊装备 & 外链 ===== */
.special-block {
  margin-bottom: 10px;
}
.special-lbl {
  font-size: 12px;
  color: var(--dim);
  margin: 0 0 6px;
}
.unit-bis {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
  margin-top: 6px;
}
.ext-link-wrap {
  margin: 6px 0 10px;
}
.ext-link {
  color: var(--blue);
  text-decoration: none;
  font-size: 12px;
}
.ext-link:hover {
  text-decoration: underline;
}
.notes-block {
  margin-top: 8px;
  padding: 10px;
  background: #101018;
  border-radius: 8px;
  border: 1px solid var(--border);
}
.note-line {
  margin: 4px 0;
  font-size: 12px;
  color: var(--dim);
}
.tierlist-essentials {
  margin-top: 16px;
}
.tierlist-row.essentials {
  border: 1px solid var(--green);
}
.tierlist-meta {
  padding: 8px 10px;
  background: #101018;
  border-radius: 8px;
  margin-bottom: 10px;
}
.comp-card-share {
  margin-top: 4px;
  display: flex;
  gap: 8px;
}
.item-search {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}
.search-input {
  background: #101018;
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text);
  padding: 6px 10px;
  font-size: 12px;
  width: 240px;
}
</style>

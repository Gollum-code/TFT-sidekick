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

interface MetaComp {
  id: string;
  name: string;
  icon_url: string;
  appearances: number;
  win_rate: number;
  top4_rate: number;
  win_share: number;
  top4_share: number;
}

interface MetaItem {
  id: string;
  name: string;
  icon_url: string;
  appearances: number;
  win_rate: number;
  top4_rate: number;
}

interface HeroItem {
  id: string;
  name: string;
  icon_url: string;
  appearances: number;
  win_rate: number;
  top4_rate: number;
}

interface HeroItemRow {
  id: string;
  name: string;
  icon_url: string;
  items: HeroItem[];
}

interface CompDetail {
  id: string;
  name: string;
  icon_url: string;
  appearances: number;
  win_rate: number;
  top4_rate: number;
  avg_placement: number;
  units: CompUnit[];
  traits: CompTraitRow[];
}

interface CompUnit {
  id: string;
  name: string;
  icon_url: string;
  count: number;
  rate: number;
  avg_tier: number;
  items: HeroItem[];
}

interface CompTraitRow {
  id: string;
  name: string;
  count: number;
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

interface DailyComp {
  id: string;
  name: string;
  icon_url: string;
  appearances: number;
  win_share: number;
  top4_share: number;
}

interface DailyRow {
  date: string;
  comps: DailyComp[];
}

interface CounterRow {
  a_id: string;
  a_name: string;
  b_id: string;
  b_name: string;
  appearances: number;
  a_avg_placement: number;
  a_win_rate: number;
}

interface NamedComp {
  id: string;
  name: string;
  primary_name: string;
  carry_name: string;
  icon_url: string;
  appearances: number;
  win_rate: number;
  top4_rate: number;
  win_share: number;
  top4_share: number;
}

interface MetaResult {
  comps: MetaComp[];
  named_comps: NamedComp[];
  items: MetaItem[];
  hero_items: HeroItemRow[];
  comp_details: CompDetail[];
  daily: DailyRow[];
  counters: CounterRow[];
  cost_win: number[];
  cost_top4: number[];
  sample_count: number;
  added_count: number;
}

interface RulesSection {
  title: string;
  rows: string[][];
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
  "overview" | "champions" | "traits" | "items" | "augments" | "wisps" | "odds" | "rules"
>("overview");
const dataLoading = ref(false);
const items = ref<ItemData[]>([]);
const champions = ref<ChampionData[]>([]);
const traits = ref<TraitData[]>([]);
const augments = ref<ItemData[]>([]);
const shopOdds = ref<ShopOddsLevel[]>([]);
const gameRules = ref<RulesSection[]>([]);
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

const filteredItems = computed(() =>
  craftable.value.filter((i) => matchName(search.value)(i.name))
);
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

// 反查：每件装备 → 适合哪些弈子（从 hero_items 统计）
const itemHeroGroups = computed(() => {
  if (!metaData.value) return [];
  const groups: Record<string, { itemId: string; itemName: string; itemIcon: string; heroes: HeroItem[] }> = {};
  for (const h of metaData.value.hero_items) {
    for (const it of h.items) {
      const g = (groups[it.id] ??= {
        itemId: it.id,
        itemName: it.name,
        itemIcon: it.icon_url,
        heroes: [],
      });
      g.heroes.push({
        id: h.id,
        name: h.name,
        icon_url: h.icon_url,
        appearances: it.appearances,
        win_rate: it.win_rate,
        top4_rate: it.top4_rate,
      });
    }
  }
  const out = Object.values(groups);
  for (const g of out) {
    g.heroes.sort((a, b) => b.appearances - a.appearances);
  }
  out.sort((a, b) => {
    const ta = a.heroes.reduce((s, h) => s + h.appearances, 0);
    const tb = b.heroes.reduce((s, h) => s + h.appearances, 0);
    return tb - ta;
  });
  return out;
});

// 当前阵容的互害数据（以 selectedComp 为主羁绊，作为 A 的对手列表）
const compCounters = computed(() => {
  if (!selectedComp.value || !metaData.value) return [];
  return metaData.value.counters
    .filter((c) => c.a_id === selectedComp.value!.id)
    .slice(0, 10);
});

// 内卷指数（辛普森集中度）与多样性指数
const concentration = computed(() => {
  if (!metaData.value || !metaData.value.named_comps.length) return 0;
  const shares = metaData.value.named_comps.map((c) => c.win_share);
  const sum = shares.reduce((a, b) => a + b, 0);
  if (sum <= 0) return 0;
  return shares.reduce((acc, s) => acc + (s / sum) * (s / sum), 0);
});
const diversity = computed(() => 1 - concentration.value);

const costColors = ["#9aa4af", "#35c66b", "#3fa8f4", "#c26bf1", "#f5b23e"];

// 阵容趋势图数据整合
const trendColors = ["#f0c24b", "#4d9de0", "#45d483", "#c26bf1", "#f0716f"];
const trendDates = computed(() => metaData.value?.daily.map((d) => d.date) ?? []);
const trendTopNames = computed(() => {
  if (!metaData.value) return [];
  const names: string[] = [];
  for (const d of metaData.value.daily) {
    for (const c of d.comps.slice(0, 5)) {
      if (!names.includes(c.name)) names.push(c.name);
    }
    if (names.length >= 5) break;
  }
  return names.slice(0, 5);
});
// 每个 Top 阵容在每天的 win_share 折线（缺的天补 0）
const trendLines = computed(() => {
  if (!metaData.value) return [];
  return trendTopNames.value.map((name) =>
    metaData.value!.daily.map((d) => d.comps.find((c) => c.name === name)?.win_share ?? 0)
  );
});
const filteredAugments = computed(() =>
  augments.value.filter((a) => matchName(search.value)(a.name))
);

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

const metaData = ref<MetaResult | null>(null);
const metaTab = ref<"comps" | "trend" | "daily" | "items" | "heroes" | "item_heroes">("comps");
const metaLoading = ref(false);
const selectedComp = ref<CompDetail | null>(null);

function showCompDetail(compId: string) {
  selectedComp.value = metaData.value?.comp_details.find((d) => d.id === compId) ?? null;
  if (!selectedComp.value) {
    // 数据里没有详情时，用 comps 条目的简单信息兜底
    const c = metaData.value?.comps.find((x) => x.id === compId);
    if (c) {
      selectedComp.value = {
        id: c.id,
        name: c.name,
        icon_url: c.icon_url,
        appearances: c.appearances,
        win_rate: c.win_rate,
        top4_rate: c.top4_rate,
        avg_placement: 0,
        units: [],
        traits: [],
      };
    }
  }
}

function closeCompDetail() {
  selectedComp.value = null;
}

// 具名阵容 key = "主羁绊|C位" → 跳转到主羁绊的 comp_detail
function showNamedComp(key: string) {
  const primary = key.split("|")[0];
  showCompDetail(primary);
}

async function getMeta() {
  metaLoading.value = true;
  error.value = "";
  try {
    const r = await invoke<MetaResult>("get_meta");
    metaData.value = r;
    lastSync.value = r.sample_count > 0 ? `样本 ${r.sample_count} 局` : "";
  } catch (e) {
    error.value = String(e);
    metaData.value = null;
  } finally {
    metaLoading.value = false;
  }
}

const navGroups: {
  key: "matches" | "stats" | "comps" | "data" | "meta";
  label: string;
  icon: string;
  children?: { label: string; go: () => void }[];
}[] = [
  { key: "matches", label: "战绩", icon: "◉" },
  { key: "stats", label: "统计", icon: "▤" },
  { key: "comps", label: "阵容", icon: "♜" },
  {
    key: "data",
    label: "数据表",
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
      { label: "环境总览", go: () => (metaTab.value = "comps") },
      { label: "阵容趋势", go: () => (metaTab.value = "trend") },
      { label: "每日统计", go: () => (metaTab.value = "daily") },
      { label: "神器纹章", go: () => (metaTab.value = "items") },
      { label: "按弈子", go: () => (metaTab.value = "heroes") },
      { label: "按装备", go: () => (metaTab.value = "item_heroes") },
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

      <!-- 阵容列表（Comp List） -->
      <template v-else-if="view === 'comps'">
        <header class="topbar">
          <h2>阵容列表</h2>
          <button @click="getMeta" :disabled="metaLoading">
            {{ metaLoading ? "加载中…" : "刷新" }}
          </button>
        </header>
        <div class="odds-note">高分段常用阵容（来源：国际服王者/宗师/大师对局）。点击查看详情。</div>

        <div v-if="selectedComp">
          <!-- 复用阵容详情 -->
          <div class="comp-detail">
            <header class="topbar">
              <button class="ghost" @click="closeCompDetail">← 返回</button>
              <h2 class="detail-title">
                <img v-if="selectedComp.icon_url" :src="selectedComp.icon_url" :alt="selectedComp.name" class="top-icon" />
                {{ selectedComp.name }}
              </h2>
              <span class="muted">{{ selectedComp.appearances }} 局</span>
            </header>
            <div class="comp-stat-cards">
              <div class="stat-card"><div class="stat-value">{{ (selectedComp.win_rate * 100).toFixed(1) }}%</div><div class="stat-label">吃鸡率</div></div>
              <div class="stat-card"><div class="stat-value">{{ (selectedComp.top4_rate * 100).toFixed(1) }}%</div><div class="stat-label">前四率</div></div>
              <div class="stat-card"><div class="stat-value">{{ selectedComp.avg_placement.toFixed(2) }}</div><div class="stat-label">平均名次</div></div>
            </div>
            <section class="board-section">
              <h3>阵容构成</h3>
              <div class="board">
                <div v-for="u in selectedComp.units" :key="u.id" class="unit-card" :style="{ opacity: 0.35 + u.rate }">
                  <div class="unit-icon-wrap">
                    <img v-if="u.icon_url" :src="u.icon_url" :alt="u.name" class="unit-icon" />
                    <div v-if="u.avg_tier" class="stars">{{ "★".repeat(Math.max(1, Math.round(u.avg_tier))) }}</div>
                  </div>
                  <div class="unit-name">{{ u.name || u.id }}</div>
                  <div class="unit-sub">{{ (u.rate * 100).toFixed(0) }}% · {{ u.count }} 局</div>
                  <div class="unit-items">
                    <div v-for="it in u.items.slice(0, 3)" :key="it.id" class="item" :title="it.name">
                      <img v-if="it.icon_url" :src="it.icon_url" :alt="it.name" />
                    </div>
                  </div>
                </div>
              </div>
            </section>
            <section class="board-section">
              <h3>常见羁绊</h3>
              <div class="traits">
                <div v-for="t in selectedComp.traits" :key="t.id" class="trait">
                  <span class="trait-name">{{ t.name || t.id }}</span>
                  <span class="trait-tier">{{ t.count }} 局</span>
                </div>
              </div>
            </section>
            <section class="board-section">
              <h3>互害 / 受益</h3>
              <div v-if="compCounters.length > 0" class="counter-list">
                <div class="meta-row meta-head">
                  <span class="meta-name">对阵阵容</span>
                  <span class="meta-num">同场局</span>
                  <span class="meta-num">本阵容均名次</span>
                  <span class="meta-num">吃鸡率</span>
                </div>
                <div v-for="c in compCounters" :key="c.b_id" class="meta-row">
                  <span class="meta-name">
                    <span class="counter-name">{{ c.b_name }}</span>
                    <span class="counter-delta" :class="{ bad: c.a_avg_placement >= 4.5 }">
                      {{ Math.round(c.a_avg_placement) >= 5 ? "克制" : (c.a_avg_placement <= 3.5 ? "被克" : "") }}
                    </span>
                  </span>
                  <span class="meta-num">{{ c.appearances }}</span>
                  <span class="meta-num">{{ c.a_avg_placement.toFixed(2) }}</span>
                  <span class="meta-num accent">{{ (c.a_win_rate * 100).toFixed(0) }}%</span>
                </div>
              </div>
              <p v-else class="empty" style="padding: 12px 0; text-align: left">样本不足，暂未统计互害数据。</p>
            </section>
          </div>
        </div>

        <template v-else>
          <div v-if="metaData && metaData.comps.length" class="comp-grid">
            <div
              v-for="(c, i) in metaData.comps"
              :key="c.id"
              class="comp-card"
              @click="showCompDetail(c.id)"
            >
              <div class="comp-card-rank" :class="{ top3: i < 3 }">#{{ i + 1 }}</div>
              <img v-if="c.icon_url" :src="c.icon_url" :alt="c.name" class="comp-card-icon" />
              <div class="comp-card-body">
                <div class="comp-card-name">{{ c.name }}</div>
                <div class="comp-card-stats">
                  <span class="accent">登顶 {{ (c.win_share * 100).toFixed(2) }}%</span>
                  <span class="green">前四 {{ (c.top4_share * 100).toFixed(2) }}%</span>
                </div>
              </div>
            </div>
          </div>
          <p v-else class="empty">暂无阵容数据，运行 crawl_meta 生成 meta.json 后刷新。</p>
        </template>
      </template>

      <!-- Meta 视图 -->
      <template v-else-if="view === 'meta'">
        <header class="topbar">
          <h2>高分段 Meta</h2>
          <button @click="getMeta" :disabled="metaLoading">
            {{ metaLoading ? "加载中…" : "刷新" }}
          </button>
        </header>
        <div class="odds-note">数据来源：国际服王者/宗师/大师段位（跨服通用）。</div>
        <p v-if="lastSync" class="hint" style="margin: 0 0 8px">{{ lastSync }}</p>

        <p v-if="error" class="error">{{ error }}</p>

        <template v-if="metaData">
          <div class="tabs tabs-group">
            <span class="tabs-group-label">环境</span>
            <button
              v-for="t in [
                { key: 'comps', label: '环境总览' },
                { key: 'trend', label: '阵容趋势' },
                { key: 'daily', label: '每日统计' },
              ]"
              :key="t.key"
              class="tab"
              :class="{ active: metaTab === t.key }"
              @click="metaTab = t.key as any"
            >
              {{ t.label }}
            </button>
          </div>
          <div class="tabs tabs-group">
            <span class="tabs-group-label">装备</span>
            <button
              v-for="t in [
                { key: 'items', label: '神器纹章' },
                { key: 'heroes', label: '按弈子' },
                { key: 'item_heroes', label: '按装备' },
              ]"
              :key="t.key"
              class="tab"
              :class="{ active: metaTab === t.key }"
              @click="metaTab = t.key as any"
            >
              {{ t.label }}
            </button>
          </div>

          <!-- 阵容详情 -->
          <template v-if="selectedComp">
            <div class="comp-detail">
              <header class="topbar">
                <button class="ghost" @click="closeCompDetail">← 返回</button>
                <h2 class="detail-title">
                  <img v-if="selectedComp.icon_url" :src="selectedComp.icon_url" :alt="selectedComp.name" class="top-icon" />
                  {{ selectedComp.name }}
                </h2>
                <span class="muted">{{ selectedComp.appearances }} 局</span>
              </header>

              <div class="comp-stat-cards">
                <div class="stat-card">
                  <div class="stat-value">{{ (selectedComp.win_rate * 100).toFixed(1) }}%</div>
                  <div class="stat-label">吃鸡率</div>
                </div>
                <div class="stat-card">
                  <div class="stat-value">{{ (selectedComp.top4_rate * 100).toFixed(1) }}%</div>
                  <div class="stat-label">前四率</div>
                </div>
                <div class="stat-card">
                  <div class="stat-value">{{ selectedComp.avg_placement.toFixed(2) }}</div>
                  <div class="stat-label">平均名次</div>
                </div>
              </div>

              <section class="board-section">
                <h3>阵容构成</h3>
                <div class="board">
                  <div
                    v-for="u in selectedComp.units"
                    :key="u.id"
                    class="unit-card"
                    :style="{ opacity: 0.35 + u.rate }"
                  >
                    <div class="unit-icon-wrap">
                      <img v-if="u.icon_url" :src="u.icon_url" :alt="u.name" class="unit-icon" />
                      <div v-if="u.avg_tier" class="stars">
                        {{ "★".repeat(Math.max(1, Math.round(u.avg_tier))) }}
                      </div>
                    </div>
                    <div class="unit-name">{{ u.name || u.id }}</div>
                    <div class="unit-sub">{{ (u.rate * 100).toFixed(0) }}% · {{ u.count }} 局</div>
                    <div class="unit-items">
                      <div v-for="it in u.items.slice(0, 3)" :key="it.id" class="item" :title="it.name">
                        <img v-if="it.icon_url" :src="it.icon_url" :alt="it.name" />
                      </div>
                    </div>
                  </div>
                </div>
              </section>

              <section class="board-section">
                <h3>常见羁绊</h3>
                <div class="traits">
                  <div v-for="t in selectedComp.traits" :key="t.id" class="trait">
                    <span class="trait-name">{{ t.name || t.id }}</span>
                    <span class="trait-tier">{{ t.count }} 局</span>
                  </div>
                </div>
              </section>

              <section class="board-section">
                <h3>互害 / 受益</h3>
                <div v-if="compCounters.length > 0" class="counter-list">
                  <div class="meta-row meta-head">
                    <span class="meta-name">对阵阵容</span>
                    <span class="meta-num">同场局</span>
                    <span class="meta-num">本阵容均名次</span>
                    <span class="meta-num">吃鸡率</span>
                  </div>
                  <div v-for="c in compCounters" :key="c.b_id" class="meta-row">
                    <span class="meta-name">
                      <span class="counter-name">{{ c.b_name }}</span>
                      <span class="counter-delta" :class="{ bad: c.a_avg_placement >= 4.5 }">
                        {{ Math.round(c.a_avg_placement) >= 5 ? "克制" : (c.a_avg_placement <= 3.5 ? "被克" : "") }}
                      </span>
                    </span>
                    <span class="meta-num">{{ c.appearances }}</span>
                    <span class="meta-num">{{ c.a_avg_placement.toFixed(2) }}</span>
                    <span class="meta-num accent">{{ (c.a_win_rate * 100).toFixed(0) }}%</span>
                  </div>
                </div>
                <p v-else class="empty" style="padding: 12px 0; text-align: left">
                  样本不足，暂未统计互害数据。
                </p>
              </section>
            </div>
          </template>

          <!-- 环境总览：指数 + 费用分布 + 具名阵容占比榜 -->
          <div v-if="metaTab === 'comps'" class="env-overview">
            <!-- 指数卡 -->
            <div class="comp-stat-cards">
              <div class="stat-card">
                <div class="stat-value">{{ (concentration * 100).toFixed(1) }}</div>
                <div class="stat-label">内卷指数</div>
              </div>
              <div class="stat-card">
                <div class="stat-value">{{ (diversity * 100).toFixed(1) }}</div>
                <div class="stat-label">多样性指数</div>
              </div>
              <div class="stat-card">
                <div class="stat-value">{{ metaData.sample_count }}</div>
                <div class="stat-label">样本对局</div>
              </div>
            </div>

            <!-- 费用分布 -->
            <section class="board-section">
              <h3>登顶费用分布</h3>
              <div class="cost-bars">
                <div v-for="(v, i) in metaData.cost_win" :key="'w' + i" class="cost-bar-row">
                  <span class="cost-bar-label" :style="{ color: costColors[i] }">{{ i + 1 }}费</span>
                  <div class="cost-bar-bg">
                    <div class="cost-bar" :style="{ width: (v * 100) + '%', background: costColors[i] }"></div>
                  </div>
                  <span class="cost-bar-val">{{ (v * 100).toFixed(1) }}%</span>
                </div>
              </div>
            </section>

            <section class="board-section">
              <h3>前四费用分布</h3>
              <div class="cost-bars">
                <div v-for="(v, i) in metaData.cost_top4" :key="'t' + i" class="cost-bar-row">
                  <span class="cost-bar-label" :style="{ color: costColors[i] }">{{ i + 1 }}费</span>
                  <div class="cost-bar-bg">
                    <div class="cost-bar" :style="{ width: (v * 100) + '%', background: costColors[i] }"></div>
                  </div>
                  <span class="cost-bar-val">{{ (v * 100).toFixed(1) }}%</span>
                </div>
              </div>
            </section>

            <!-- 具名阵容登顶占比榜 -->
            <section class="board-section">
              <h3>登顶占比 · 阵容</h3>
              <div v-if="metaData.named_comps.length" class="meta-list">
                <div class="meta-row meta-head">
                  <span class="meta-rank">#</span>
                  <span class="meta-name">阵容</span>
                  <span class="meta-num">场次</span>
                  <span class="meta-num">登顶占比</span>
                  <span class="meta-num">前四占比</span>
                </div>
                <div
                  v-for="(c, i) in metaData.named_comps"
                  :key="c.id"
                  class="meta-row comp-row"
                  @click="showNamedComp(c.id)"
                >
                  <span class="meta-rank" :class="{ top3: i < 3 }">{{ i + 1 }}</span>
                  <span class="meta-name">
                    <img v-if="c.icon_url" :src="c.icon_url" :alt="c.carry_name" class="top-icon" />
                    {{ c.name }}
                    <span class="comp-sub">{{ c.primary_name }} · {{ c.carry_name }}</span>
                  </span>
                  <span class="meta-num">{{ c.appearances }}</span>
                  <span class="meta-num accent">{{ (c.win_share * 100).toFixed(2) }}%</span>
                  <span class="meta-num accent green">{{ (c.top4_share * 100).toFixed(2) }}%</span>
                </div>
              </div>
              <p v-else class="empty">暂无数据</p>
            </section>
          </div>

          <!-- 装备榜 -->
          <div v-if="metaTab === 'items' && metaData.items.length > 0" class="meta-list">
            <div class="meta-row meta-head">
              <span class="meta-rank">#</span>
              <span class="meta-name">装备</span>
              <span class="meta-num">出现</span>
              <span class="meta-num">吃鸡率</span>
              <span class="meta-num">前四率</span>
            </div>
            <div v-for="(it, i) in metaData.items" :key="it.id" class="meta-row">
              <span class="meta-rank" :class="{ top3: i < 3 }">{{ i + 1 }}</span>
              <span class="meta-name">
                <img v-if="it.icon_url" :src="it.icon_url" :alt="it.name" class="top-icon" />
                {{ it.name }}
              </span>
              <span class="meta-num">{{ it.appearances }}</span>
              <span class="meta-num accent">{{ (it.win_rate * 100).toFixed(1) }}%</span>
              <span class="meta-num accent green">{{ (it.top4_rate * 100).toFixed(1) }}%</span>
            </div>
          </div>

          <!-- 按弈子 -->
          <div v-if="metaTab === 'heroes' && metaData.hero_items.length > 0" class="hero-item-list">
            <div v-for="h in metaData.hero_items" :key="h.id" class="hero-item-card">
              <div class="champ-head">
                <img v-if="h.icon_url" :src="h.icon_url" :alt="h.name" class="champ-icon" />
                <span class="champ-name">{{ h.name || h.id }}</span>
              </div>
              <div class="hero-item-row" v-for="it in h.items" :key="it.id">
                <img v-if="it.icon_url" :src="it.icon_url" :alt="it.name" class="top-icon" />
                <span class="top-name">{{ it.name || it.id }}</span>
                <span class="meta-num">{{ it.appearances }}</span>
                <span class="meta-num accent green">{{ (it.top4_rate * 100).toFixed(1) }}%</span>
              </div>
            </div>
          </div>

          <!-- 按装备：反查每件装备适合哪些弈子 -->
          <div v-if="metaTab === 'item_heroes'" class="hero-item-list">
            <div v-for="g in itemHeroGroups" :key="g.itemId" class="hero-item-card">
              <div class="champ-head">
                <img v-if="g.itemIcon" :src="g.itemIcon" :alt="g.itemName" class="champ-icon" />
                <span class="champ-name">{{ g.itemName || g.itemId }}</span>
              </div>
              <div class="hero-item-row" v-for="h in g.heroes.slice(0, 6)" :key="h.id">
                <img v-if="h.icon_url" :src="h.icon_url" :alt="h.name" class="top-icon" />
                <span class="top-name">{{ h.name || h.id }}</span>
                <span class="meta-num">{{ h.appearances }}</span>
                <span class="meta-num accent green">{{ (h.top4_rate * 100).toFixed(1) }}%</span>
              </div>
            </div>
          </div>

          <!-- 每日统计 -->
          <div v-if="metaTab === 'daily' && metaData.daily.length > 0" class="daily-list">
            <div v-for="d in metaData.daily" :key="d.date" class="daily-card">
              <div class="champ-head">
                <span class="champ-name">{{ d.date }}</span>
                <span class="muted">{{ d.comps.length }} 阵容</span>
              </div>
              <div class="daily-row" v-for="(c, i) in d.comps.slice(0, 8)" :key="c.id">
                <span class="meta-rank" :class="{ top3: i < 3 }">{{ i + 1 }}</span>
                <span class="meta-name">
                  <img v-if="c.icon_url" :src="c.icon_url" :alt="c.name" class="top-icon" />
                  {{ c.name }}
                </span>
                <span class="meta-num">{{ c.appearances }}</span>
                <span class="meta-num accent">{{ (c.win_share * 100).toFixed(1) }}%</span>
              </div>
            </div>
          </div>
          <div v-else-if="metaTab === 'daily' && !metaLoading" class="empty">暂无每日数据。</div>

          <!-- 阵容趋势：跨日占比变化 -->
          <div v-if="metaTab === 'trend' && metaData.daily.length > 0" class="trend-chart-wrap">
            <div class="trend-legend">
              <span v-for="(name, i) in trendTopNames" :key="i" class="trend-legend-item">
                <span class="trend-legend-dot" :style="{ background: trendColors[i % trendColors.length] }"></span>
                {{ name }}
              </span>
            </div>
            <svg class="trend-svg" :viewBox="'0 0 ' + trendDates.length * 80 + ' 200'" preserveAspectRatio="xMidYMid meet">
              <line v-for="n in 5" :key="'g'+n" :x1="0" :y1="n*40" :x2="trendDates.length*80" :y2="n*40" stroke="#2a2a3a" stroke-width="0.5"/>
              <text v-for="(name, i) in trendTopNames" :key="'l'+i" x="4" :y="205" fill="#888" font-size="11">{{ name.slice(0, 4) }}</text>
              <g v-for="(line, i) in trendLines" :key="i">
                <polyline
                  :points="line.map((v, j) => j * 80 + ',' + (200 - v * 400)).join(' ')"
                  :stroke="trendColors[i % trendColors.length]"
                  stroke-width="2"
                  fill="none"
                />
              </g>
            </svg>
          </div>
          <div v-else-if="metaTab === 'trend' && !metaLoading" class="empty">暂无趋势数据。</div>
        </template>
        <p v-else-if="!metaLoading" class="empty">
          暂无 Meta 数据。请在项目根目录运行
          <code>cargo run --bin crawl_meta</code> 生成 meta.json 后刷新。
        </p>
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
</style>

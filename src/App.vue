<script setup lang="ts">
import { NAME_MAP, unitName, itemName, traitName, unitNameEn, unitCost } from './lib/tftable-helpers';
import type {
  ChampionData,
  ItemData,
  RulesSection,
  ShopOddsLevel,
  TraitData,
  TFCompStat,
  TFCompositionCard,
  TFItemGuideEntry,
  TFTtableData,
  TFUnitItem,
} from './composables/useTftData';
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface MatchSummary {
  game_id: string;
  game_mode: string | null;
  game_version: string | null;
  game_datetime: string;
  placement: number;
  level: number | null;
  augments: string[];
  queue_id: number | null;
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
  position_x?: number | null;
  position_y?: number | null;
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

const view = ref<
  | "matches"
  | "stats"
  | "comps"
  | "data"
  | "meta"
  | "builder"
  | "explorer"
  | "early"
  | "traits"
  | "augments"
  | "compList"
  | "settings"
>("matches");
const matches = ref<MatchSummary[]>([]);
// 战绩筛选：模式（队列）/ 名次范围 / 文本搜索
const QUEUE_LABELS: Record<number, string> = {
  1090: "排位",
  1130: "匹配",
  1100: "狂暴",
  1110: "双人",
  1160: "疾驰",
  1165: "赛季回归",
  1200: "四人对战",
};
function queueLabel(id: number | null | undefined): string {
  if (id == null || id === 0) return "";
  return QUEUE_LABELS[id] ?? `队列 ${id}`;
}
const matchQueueFilter = ref<number | "all">("all");
const matchQueues = computed(() => {
  const set = new Set<number>();
  for (const m of matches.value) if (m.queue_id) set.add(m.queue_id);
  return [...set].sort((a, b) => a - b);
});
const matchPlaceFilter = ref<"all" | "win" | "top4" | "out">("all");
const matchSearch = ref("");
const filteredMatches = computed(() => {
  let rows = matches.value;
  if (matchQueueFilter.value !== "all") {
    rows = rows.filter((m) => m.queue_id === matchQueueFilter.value);
  }
  if (matchPlaceFilter.value === "win") rows = rows.filter((m) => m.placement === 1);
  else if (matchPlaceFilter.value === "top4") rows = rows.filter((m) => m.placement <= 4);
  else if (matchPlaceFilter.value === "out") rows = rows.filter((m) => m.placement > 4);
  const q = matchSearch.value.trim().toLowerCase();
  if (q) {
    rows = rows.filter((m) =>
      (m.game_mode ?? "").toLowerCase().includes(q) || fmtDate(m.game_datetime).toLowerCase().includes(q)
    );
  }
  return rows;
});

// ===== 游戏内辅助悬浮窗（overlay 窗口专用） =====
const isOverlay = getCurrentWindow().label === "overlay";
const overlayPhase = ref("");
const overlaySummoner = ref("");
const overlayRank = ref("");
const overlayDivision = ref("");
const overlayLp = ref(0);

// 进入新对局时重置商店历史（抢牌检测基于本局数据）
watch(overlayPhase, (n, o) => {
  if (n === "InProgress" && o !== "InProgress") resetShopHistory();
});

// 对局实时状态（2999 Game Client API）
const live = ref<{
  available: boolean;
  gold: number | null;
  level: number | null;
  health: number | null;
  max_health: number | null;
  stage: string;
  game_time: number;
  players: {
    name: string;
    health: number;
    max_health: number;
    level: number | null;
    rank: number | null;
    is_me: boolean;
  }[];
}>({
  available: false,
  gold: null,
  level: null,
  health: null,
  max_health: null,
  stage: "",
  game_time: 0,
  players: [],
});

async function refreshOverlay() {
  try {
    const r = await invoke<any>("set_overlay_state");
    overlayPhase.value = r.phase || "";
    if (r.summoner) overlaySummoner.value = r.summoner;
    if (r.rank_tier) {
      overlayRank.value = r.rank_tier;
      overlayDivision.value = r.rank_division || "";
      overlayLp.value = r.rank_lp || 0;
    }
  } catch {
    /* 静默 */
  }
}

async function toggleOverlay() {
  try {
    const visible = await invoke<boolean>("toggle_overlay");
    if (!visible) showToast("游戏内悬浮窗已隐藏");
  } catch (e) {
    showToast(`悬浮窗操作失败: ${e}`);
  }
}

function applyLive(payload: any) {
  if (!payload) {
    live.value = { available: false, gold: null, level: null, health: null, max_health: null, stage: "", game_time: 0, players: [] };
    return;
  }
  live.value = {
    available: !!payload.available,
    gold: payload.gold ?? null,
    level: payload.level ?? null,
    health: payload.health ?? null,
    max_health: payload.max_health ?? null,
    stage: payload.stage ?? "",
    game_time: payload.game_time ?? 0,
    players: payload.players ?? [],
  };
}

function fmtGameTime(s: number) {
  if (!s) return "0:00";
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${sec.toString().padStart(2, "0")}`;
}

/** 是否开局阶段（阶段 1，即选装环/早期） */
const isEarlyStage = computed(() => live.value.available && parseStage(live.value.stage).major <= 1);

function hpWidth(p: { health: number; max_health: number }) {
  const max = p.max_health > 0 ? p.max_health : 100;
  const pct = Math.max(0, Math.min(100, (p.health / max) * 100));
  return { width: `${pct}%` };
}

function oppHpWidth(p: { health: number }) {
  return hpWidth({ health: p.health, max_health: 100 });
}

/** 对手面板：血量/等级/威胁标记排序（威胁 = 血量最高 & 等级领先） */
interface OpponentRow {
  name: string;
  health: number;
  level: number | null;
  rank: number | null;
  is_me: boolean;
  threat: "none" | "warn" | "bad";
  threatText: string;
}
const opponentPanel = computed<OpponentRow[]>(() => {
  const my = live.value.players.find((p) => p.is_me);
  const myHp = my?.health ?? 0;
  const myLv = my?.level ?? 0;
  return [...live.value.players]
    .map((p) => {
      // 威胁判定：对手血量远高于我 / 等级领先我
      const hpLead = p.health - myHp;
      const lvLead = (p.level ?? 0) - myLv;
      let threat: OpponentRow["threat"] = "none";
      let threatText = "";
      if (!p.is_me && p.health > 0) {
        if (hpLead >= 40 && lvLead >= 1) {
          threat = "bad";
          threatText = `血量+${Math.round(hpLead)} 等级+${lvLead} · 威胁`;
        } else if (hpLead >= 20) {
          threat = "warn";
          threatText = `血量+${Math.round(hpLead)} · 需提防`;
        }
      }
      return {
        name: p.is_me ? "我" : p.name || "对手",
        health: p.health,
        level: p.level,
        rank: p.rank,
        is_me: p.is_me,
        threat,
        threatText,
      };
    })
    // 排序：我排最前，其余按血量降序（血量最高 = 最有威胁）
    .sort((a, b) => {
      if (a.is_me) return -1;
      if (b.is_me) return 1;
      return b.health - a.health;
    })
    .slice(0, 8);
});
/** 头部汇总：谁最威胁 + 场上最高血量 */
const opponentSummary = computed(() => {
  const rivals = opponentPanel.value.filter((p) => !p.is_me && p.health > 0);
  if (!rivals.length) return null;
  const top = rivals[0];
  return { top, count: rivals.length };
});

// ===== 商店 OCR 识别 =====
const ocrScanning = ref(false);
const ocrText = ref("");
const ocrError = ref("");
interface OcrUnit {
  id: string;
  name: string;
  icon: string;
  cost: number;
  score: number;
}
const ocrMatched = ref<OcrUnit[]>([]);

/** 商店 5 格逐格判定结果 */
interface ShopSlot {
  index: number;
  unit: OcrUnit | null;
  verdict: "BUY" | "CONSIDER" | "SKIP" | null;
  reason: string;
}
const shopSlots = ref<ShopSlot[]>([]);

async function scanShopOcr() {
  ocrScanning.value = true;
  ocrError.value = "";
  ocrText.value = "";
  shopSlots.value = [];
  try {
    // 逐格识别 5 个格子（更精准），失败则退回整条识别
    const slots = await invoke<string[]>("ocr_shop_slots");
    const units = slots.map((t) => matchOcrToUnits(t)[0] ?? null);
    shopSlots.value = units.map((u, i) => ({
      index: i,
      unit: u,
      verdict: null,
      reason: "",
    }));
    ocrMatched.value = units.filter((u): u is OcrUnit => !!u);
    ocrText.value = slots.join(" ");
    judgeShopSlots();
    recordShopHistory(units.filter((u): u is OcrUnit => !!u));
  } catch (e) {
    // 退回整条商店 OCR
    try {
      const text = await invoke<string>("ocr_shop_screen");
      ocrText.value = text;
      ocrMatched.value = matchOcrToUnits(text);
    } catch (e2) {
      ocrError.value = String(e);
      ocrMatched.value = [];
    }
  } finally {
    ocrScanning.value = false;
  }
}

// ===== 抢牌 / 牌池检测（基于商店历史推断） =====
// 只读自己的商店历史：某核心在最近 N 次扫描里一直不出现 → 可能被其他玩家抢。
interface ShopHistoryEntry {
  stage: string;
  unitIds: string[];
}
const shopHistory = ref<ShopHistoryEntry[]>([]);
/** 记录一次商店扫描（滚动保留最近 12 次） */
function recordShopHistory(units: OcrUnit[]) {
  shopHistory.value.push({ stage: live.value.stage, unitIds: units.map((u) => u.id) });
  if (shopHistory.value.length > 12) shopHistory.value.shift();
}
/** 清空商店历史（新一局开始时） */
function resetShopHistory() {
  shopHistory.value = [];
}

/** 抢牌检测：对当前目标阵容（Pin/OCR 命中）的核心 carry，
 * 统计最近扫描中它出现的次数；连续 N 次不出 + 该阶段本应出现 → 提示可能被抢。 */
const contestAlerts = computed<{ unitId: string; name: string; icon: string; cost: number; text: string }[]>(() => {
  if (!effectivePin.value || shopHistory.value.length < 4) return [];
  const d = tftData.value?.compDetails[effectivePin.value];
  if (!d) return [];
  const prio = new Map(d.composition.priorityMatrixRows.map((p) => [p.unitId, p.unitPriority ?? 0]));
  // 取优先级最高、且费用 ≥ 3 的核心 carry（低费没人抢那么狠，后期高费才是关键）
  const carries = [...d.composition.overview.coreUnits]
    .filter((u) => u.cost >= 3)
    .sort((a, b) => (prio.get(b.id) ?? 0) - (prio.get(a.id) ?? 0))
    .slice(0, 3);
  const stageMajor = parseStage(live.value.stage).major || 0;
  const out: { unitId: string; name: string; icon: string; cost: number; text: string }[] = [];
  for (const c of carries) {
    const seen = shopHistory.value.filter((h) => h.unitIds.includes(c.id)).length;
    const total = shopHistory.value.length;
    // 阶段 3+ 才值得提示（前期可能只是没刷到）
    if (stageMajor < 3) continue;
    // 连续多次扫描一次都没出现
    if (seen === 0 && total >= 5) {
      out.push({
        unitId: c.id,
        name: c.nameCn || c.name,
        icon: unitIcon(c.id),
        cost: c.cost,
        text: `最近 ${total} 次扫描都没出现「${c.nameCn || c.name}」，可能被抢，考虑转阵容或提前 D`,
      });
    } else if (seen / total < 0.15 && total >= 6) {
      out.push({
        unitId: c.id,
        name: c.nameCn || c.name,
        icon: unitIcon(c.id),
        cost: c.cost,
        text: `「${c.nameCn || c.name}」出现率偏低（${seen}/${total}），留意是否被抢`,
      });
    }
  }
  return out.slice(0, 2);
});

/** 逐格判定 BUY/CONSIDER/SKIP：
 * 结合「当前命中阵容」（ocrRecommendations 的 coreUnits 交集最大者）
 * 与当前阶段（前期优先低费过渡，后期优先高费核心）。
 */
function judgeShopSlots() {  if (!shopSlots.value.length) return;
  // 目标阵容：Pin 优先，其次 OCR 命中交集最大的阵容
  const targetSlug =
    effectivePin.value ??
    ocrRecommendations.value?.comps?.[0]?.slug ??
    overlaySelectedComp.value;
  const core = new Set<string>();
  if (targetSlug && tftData.value) {
    for (const u of tftData.value.compDetails[targetSlug].composition.overview.coreUnits) {
      core.add(u.id);
    }
  }
  const stageMajor = parseStage(live.value.stage).major || 0;
  for (const slot of shopSlots.value) {
    const u = slot.unit;
    if (!u) {
      slot.verdict = null;
      slot.reason = "未识别";
      continue;
    }
    if (core.has(u.id)) {
      slot.verdict = "BUY";
      slot.reason = "目标阵容核心";
    } else if (u.cost <= 2 && stageMajor >= 1 && stageMajor <= 2) {
      slot.verdict = "CONSIDER";
      slot.reason = "前期低费过渡可拿";
    } else if (u.cost >= 4 && stageMajor >= 4) {
      slot.verdict = "CONSIDER";
      slot.reason = "后期高费核心候选";
    } else if (u.cost >= 4 && stageMajor <= 3) {
      slot.verdict = "SKIP";
      slot.reason = "前期太贵不划算";
    } else {
      slot.verdict = "SKIP";
      slot.reason = "偏离目标阵容";
    }
  }
}

/** OCR 文本 → 匹配棋子（模糊匹配名字） */
function matchOcrToUnits(text: string): { id: string; name: string; icon: string; cost: number; score: number }[] {
  if (!text.trim()) return [];
  const units = (NAME_MAP.units as Record<string, any>);
  const out: { id: string; name: string; icon: string; cost: number; score: number }[] = [];
  // 按名字长度从长到短匹配，避免"阿狸"先命中"阿木木"
  const entries = Object.entries(units).sort(
    (a, b) => (b[1].cn?.length ?? 0) - (a[1].cn?.length ?? 0)
  );
  const cleaned = text.replace(/[\s·。，,]*/g, "");
  const used = new Set<string>();
  for (const [id, info] of entries) {
    const cn = info.cn ?? "";
    const en = info.en ?? "";
    if (!cn && !en) continue;
    if (used.has(id)) continue;
    const hitCn = cn.length >= 2 && cleaned.includes(cn);
    const hitEn = en.length >= 3 && text.toLowerCase().includes(en.toLowerCase());
    if (hitCn || hitEn) {
      out.push({
        id,
        name: cn || en,
        icon: unitIcon(id),
        cost: info.c ?? 0,
        score: hitCn ? cn.length : 1,
      });
      used.add(id);
    }
  }
  return out.sort((a, b) => b.score - a.score).slice(0, 5);
}

/** 商店 OCR 推荐：根据识别出的棋子 → 关联的强势阵容 + 同阵容关键棋子 */
const ocrRecommendations = computed((): {
  comps: { slug: string; name: string; icon: string; matched: number; total: number }[];
  suggest: { id: string; name: string; icon: string; cost: number }[];
} | null => {
  if (!ocrMatched.value.length || !tftData.value) return null;
  const unitSet = new Set(ocrMatched.value.map((u) => u.id));
  // 统计这些棋子覆盖的强势阵容（compDetails coreUnits 交集）
  const compHits: { slug: string; name: string; icon: string; matched: number; total: number; missing: string[] }[] = [];
  for (const slug of Object.keys(tftData.value.compDetails)) {
    const core = tftData.value.compDetails[slug].composition.overview.coreUnits;
    const ids = core.map((u) => u.id);
    const matched = ids.filter((id) => unitSet.has(id)).length;
    if (matched >= 2) {
      compHits.push({
        slug,
        name: compName(slug),
        icon: compCardIcon(slug),
        matched,
        total: ids.length,
        missing: ids.filter((id) => !unitSet.has(id)).slice(0, 3),
      });
    }
  }
  const top = compHits.sort((a, b) => b.matched - a.matched).slice(0, 3);
  if (!top.length) return null;
  const best = top[0];
  return {
    comps: top,
    // 补充建议拿哪些缺失棋子（最高优先级：当前能拿的）
    suggest: best.missing.slice(0, 3).map((id) => ({
      id,
      name: unitName(id),
      icon: unitIcon(id),
      cost: unitCost(id),
    })),
  };
});

// ===== 强化符文 OCR 识别与推荐 =====
const augScanning = ref(false);
const augText = ref("");
const augError = ref("");
interface OcrAugment {
  id: string;
  name: string;
  icon: string;
  score: number;
  desc: string;
}
const augMatched = ref<OcrAugment[]>([]);

async function scanAugmentsOcr() {
  augScanning.value = true;
  augError.value = "";
  augText.value = "";
  try {
    const text = await invoke<string>("ocr_augments_screen");
    augText.value = text;
    augMatched.value = matchOcrToAugments(text);
  } catch (e) {
    augError.value = String(e);
    augMatched.value = [];
  } finally {
    augScanning.value = false;
  }
}

/** OCR 文本 → 匹配强化符文（按中文名长度从长到短，避免短名先命中） */
function matchOcrToAugments(text: string): OcrAugment[] {
  if (!text.trim()) return [];
  const cleaned = text.replace(/[\s·。，,]*/g, "");
  const out: OcrAugment[] = [];
  const used = new Set<string>();
  const entries = [...augments.value].sort((a, b) => b.name.length - a.name.length);
  for (const a of entries) {
    if (!a.name || used.has(a.id)) continue;
    const name = a.name;
    if (name.length < 2) continue;
    if (cleaned.includes(name)) {
      out.push({
        id: a.id,
        name,
        icon: a.icon_url,
        score: name.length,
        desc: a.desc,
      });
      used.add(a.id);
    }
  }
  return out.sort((a, b) => b.score - a.score).slice(0, 4);
}

/** 强化符文评分：关键词分类 + 与当前命中阵容的契合度。
 * 参考主流 overlay（MetaTFT/TFT Compass）的做法——把每个选项对照
 * 经济/等级/装备/羁绊这几类通用价值打分，再用当前阵容方向微调。
 * 返回每个识别的符文 { level: good/warn/bad, text: 理由, score }。
 */
function scoreAugments(matched: OcrAugment[]): {
  level: "good" | "warn" | "bad";
  text: string;
  score: number;
}[] {
  if (!matched.length) return [];
  // 当前阵容方向：Pin 的阵容优先，其次商店 OCR 命中的阵容 → 其核心羁绊名集合（中文）
  const compTraits = new Set<string>();
  const targetSlugs =
    (effectivePin.value ? [effectivePin.value] : []) as string[];
  if (!targetSlugs.length && ocrRecommendations.value) {
    for (const c of ocrRecommendations.value.comps) targetSlugs.push(c.slug);
  }
  if (tftData.value) {
    for (const slug of targetSlugs) {
      const d = tftData.value.compDetails[slug];
      if (!d) continue;
      for (const t of d.composition.overview.traits) {
        const cn = (NAME_MAP.traits as Record<string, any>)[t.id]?.cn;
        if (cn) compTraits.add(cn);
      }
    }
  }
  // 羁绊增强类关键词（纹章/羁绊名）— 命中当前阵容方向加分量
  const econKw = /金币|利息|经济|存钱|理财|投资|财富|储蓄|提款|购物|大买特买|对冲基金|存款/;
  const lvlKw = /升级|经验|人口|等级|遥遥领先|扩展包|宝宝学院/;
  const itemKw = /装备|散件|组件|合成|武器|大剑|铁匠|宝藏|自助餐|打捞|整备|修缮/;
  const emblemKw = /纹章|烙印/;
  return matched.map((m) => {
    const s = m.name + " " + m.desc;
    let score = 55; // 中性基准
    const reasons: string[] = [];
    // 通用价值：经济/等级/装备优先级高于纯战斗增强（多数版本如此）
    if (econKw.test(s)) { score += 20; reasons.push("经济类，前期滚雪球价值高"); }
    if (lvlKw.test(s)) { score += 12; reasons.push("等级/经验类，节奏领先"); }
    if (itemKw.test(s)) { score += 10; reasons.push("装备类，成型快"); }
    if (emblemKw.test(s)) { score += 6; reasons.push("纹章类，可补羁绊"); }
    // 与当前命中阵容的羁绊契合
    if (compTraits.size) {
      for (const t of compTraits) {
        if (s.includes(t)) { score += 15; reasons.push(`契合当前阵容「${t}」`); break; }
      }
    }
    const level: "good" | "warn" | "bad" = score >= 75 ? "good" : score >= 60 ? "warn" : "bad";
    return { level, score, text: reasons.length ? reasons.join("；") : "通用型强化" };
  });
}

// ===== overlay 页签与扩展内容 =====
const overlayTab = ref<"live" | "comps" | "items">("live");
const overlaySelectedComp = ref<string | null>(null);

/** 装备推荐：从 unitItems 汇总全阵容高频装备 + 对应弈子 */
const overlayItemRecs = computed(() => {
  if (!tftData.value) return [];
  // 收集每个弈子的 Core/HighFrequency 装备（带必要性）
  const recs: Record<string, { itemId: string; need: number; units: { id: string; need: number }[] }> = {};
  for (const u of tftData.value.unitItems) {
    const bids = u.buckets?.Core ?? [];
    const freq = u.buckets?.HighFrequency ?? [];
    for (const b of [...bids, ...freq]) {
      if (!b.itemId) continue;
      const need = b.necessity ?? 0;
      if (!recs[b.itemId]) recs[b.itemId] = { itemId: b.itemId, need: 0, units: [] };
      recs[b.itemId].need += need;
      recs[b.itemId].units.push({ id: u.unitId, need });
    }
  }
  return Object.values(recs)
    .sort((a, b) => b.need - a.need)
    .slice(0, 5)
    .map((r) => ({
      itemId: r.itemId,
      need: r.need,
      units: r.units.sort((a, b) => b.need - a.need).slice(0, 3),
    }));
});

/** 阵容详情（overlay 内展开）：核心棋子 + 出装 + 站位 + 羁绊 */
const overlayCompDetail = computed(() => {
  const slug = overlaySelectedComp.value;
  if (!slug || !tftData.value) return null;
  const d = tftData.value.compDetails[slug];
  if (!d) return null;
  const ov = d.composition.overview;
  // teamCode → 位置数组 → 28 格棋盘
  const positions = parseTeamCode(ov.teamCode);
  const { grid, bench } = buildBoardGrid(positions);
  // 序号 → 单位 id（对照 coreUnits 顺序）
  const unitById: Record<number, string> = {};
  ov.coreUnits.forEach((u, i) => (unitById[i + 1] = u.id));
  const unitByPos: Record<number, string> = {};
  for (let i = 0; i < 28; i++) {
    const n = grid[i];
    if (n && unitById[n]) unitByPos[i] = unitById[n];
  }
  return {
    slug,
    name: compName(slug),
    icon: compCardIcon(slug),
    stats: ov.stats,
    units: ov.coreUnits,
    traits: ov.traits,
    teamCodeGrid: grid,
    bench,
    coreUnitByPos: unitByPos,
    variants: d.composition.variants,
    priority: d.composition.priorityMatrixRows,
  };
});

/** 单位 id → 该单位所属羁绊的中文名集合（CDragon champions.value 提供 traits 中文名） */
const unitTraitNames = computed(() => {
  const m: Record<string, string[]> = {};
  for (const c of champions.value) m[c.id] = c.traits ?? [];
  return m;
});

/** 按当前等级给出「这一阶段该上的棋子」与已激活羁绊。
 * 逻辑：priorityMatrixRows 按 unitPriority 降序 → 优先选费用 ≤ 当前等级的
 * 核心棋子 → 取前 N 个（N = 当前等级，即能上场的数量）→ 计算这些棋子的羁绊激活。
 * 参考主流 overlay「该等级该上的阵容板」思路。
 */
const overlayLevelBoard = computed(() => {
  const slug = overlaySelectedComp.value;
  if (!slug || !tftData.value) return null;
  const d = tftData.value.compDetails[slug];
  if (!d) return null;
  const ov = d.composition.overview;
  const level = live.value.level ?? ov.coreUnits.length;
  if (!level || level <= 0) return null;

  // 核心棋子按优先级（unitPriority）降序；无 priorityMatrix 时用原顺序
  const prio = new Map(d.composition.priorityMatrixRows.map((p) => [p.unitId, p.unitPriority ?? 0]));
  const ordered = [...ov.coreUnits].sort(
    (a, b) => (prio.get(b.id) ?? 0) - (prio.get(a.id) ?? 0) || b.cost - a.cost
  );
  // 只取费用 ≤ 等级的棋子（等级不够拿不到高费），再按上场上限截断
  const affordable = ordered.filter((u) => u.cost <= level);
  const take = affordable.slice(0, level);

  // 计算已激活羁绊
  const tmap = unitTraitNames.value;
  const activeTraits = ov.traits
    .map((t) => {
      const cn = traitName(t.id);
      const count = take.filter((u) => (tmap[u.id] ?? []).includes(cn)).length;
      return { id: t.id, name: cn, icon: t.icon, count, required: t.unitsRequired, active: count >= t.unitsRequired };
    })
    .filter((t) => t.count > 0)
    .sort((a, b) => b.count - a.count);

  return {
    level,
    units: take,
    // 尚未到齐但值得补的羁绊（已激活优先）
    nextTraits: activeTraits.filter((t) => !t.active).slice(0, 3),
    activeTraits,
  };
});

// ===== 散件 → 合成建议（针对当前目标阵容） =====
// 手动点选你手上有的散件（装备栏小件），据此推荐该合哪件装备。
const heldComponents = ref<string[]>([]);
// 目标阵容：Pin 的阵容 > overlay 当前展开的阵容 > 商店 OCR 命中阵容 > 版本第一
const slamTargetComp = computed(() => {
  if (effectivePin.value && tftData.value?.compDetails[effectivePin.value]) {
    return effectivePin.value;
  }
  if (overlaySelectedComp.value && tftData.value?.compDetails[overlaySelectedComp.value]) {
    return overlaySelectedComp.value;
  }
  const slug = ocrRecommendations.value?.comps?.[0]?.slug;
  if (slug && tftData.value?.compDetails[slug]) return slug;
  const top = overlayCompRecs.value[0]?.slug;
  return top && tftData.value?.compDetails[top] ? top : null;
});
// 目标阵容的成装优先级：unitItems 的 buckets.Core/Important → 全阵容汇总必要性
const slamPriority = computed(() => {
  if (!tftData.value || !slamTargetComp.value) return new Map<string, number>();
  const d = tftData.value.compDetails[slamTargetComp.value];
  const m = new Map<string, number>();
  for (const row of d.composition.priorityMatrixRows) {
    for (const b of [...(row.buckets?.Core ?? []), ...(row.buckets?.Important ?? []), ...(row.buckets?.HighFrequency ?? [])]) {
      if (!b.itemId) continue;
      m.set(b.itemId, (m.get(b.itemId) ?? 0) + (b.necessity ?? 0));
    }
  }
  return m;
});
// 根据当前持有的散件 → 推荐可合成的成装（全部组件都在手上），按优先级排序
const slamRecs = computed(() => {
  const held = new Set(heldComponents.value);
  if (!held.size || !slamTargetComp.value) return [];
  const prio = slamPriority.value;
  const out: { id: string; name: string; icon: string; need: number; recipe: string[]; hasRecipe: boolean }[] = [];
  for (const item of items.value) {
    if (item.composition.length < 2) continue; // 只看成装
    const recipe = item.composition;
    const hasAll = recipe.every((c) => held.has(c));
    const need = prio.get(item.id) ?? 0;
    // 全部凑齐 或 差一件且是该阵容高优先装备 → 都值得提示
    const missing = recipe.filter((c) => !held.has(c));
    const relevance = need > 0 ? need : missing.length === 0 ? 0.05 : 0;
    if (hasAll || missing.length === 1) {
      if (hasAll || relevance > 0) {
        out.push({
          id: item.id,
          name: item.name,
          icon: item.icon_url,
          need: need,
          recipe,
          hasRecipe: hasAll,
        });
      }
    }
  }
  return out
    .sort((a, b) => b.need - a.need || Number(b.hasRecipe) - Number(a.hasRecipe))
    .slice(0, 6);
});
function toggleHeld(id: string) {
  const i = heldComponents.value.indexOf(id);
  if (i >= 0) heldComponents.value.splice(i, 1);
  else heldComponents.value.push(id);
}

/** 合成优先级条宽度：相对最高必要度归一到 15%-100% */
function slamPct(need: number): number {
  const max = Math.max(...slamRecs.value.map((r) => r.need), 0.001);
  const ratio = need / max;
  return 15 + ratio * 85;
}

// ===== 开局选装助手：每个散件的价值分 =====
// 散件价值 = 携带它的核心棋子（Core bucket）归属阵容的 AVP 加权之和。
// 用于选装环 3 选 1（开局第一件散件），以及判断手牌方向。
interface ScatterScore {
  id: string;
  name: string;
  icon: string;
  value: number; // 0-100
  bestComps: { slug: string; name: string }[]; // 最适配的阵容
  bestCarriers: { id: string; name: string }[]; // 常见 carry
}
const scatterScores = computed<ScatterScore[]>(() => {
  if (!tftData.value) return [];
  // comp_slug → avp（用于加权，越强势阵容权重越高）
  const avpOf: Record<string, number> = {};
  for (const s of tftData.value.compAtlas.compositionSummaries) avpOf[s.slug] = s.avgPlacementValue;
  // unitId → { comps: [{slug, avp}], cost }
  const byUnit: Record<string, { comps: { slug: string; w: number }[] }> = {};
  for (const u of tftData.value.unitItems) {
    const comps = (u.coreCompStrengths ?? []).map((c) => ({
      slug: c.comp_key,
      // AVP 越低（越好）→ 权重越高
      w: Math.max(0.1, (6 - (avpOf[c.comp_key] ?? 5)) ),
    }));
    byUnit[u.unitId] = { comps };
  }
  // 汇总每个散件
  const acc: Record<string, { value: number; comps: Map<string, number>; carriers: Map<string, number> }> = {};
  for (const u of tftData.value.unitItems) {
    const core = u.buckets?.Core ?? [];
    const info = byUnit[u.unitId];
    for (const b of core) {
      if (!b.itemId) continue;
      // 每个散件的价值：它在这棋子核心装里出现次数 × 棋子所属阵容权重（取最优 comp）
      const bestW = info.comps.length ? Math.max(...info.comps.map((c) => c.w)) : 0.5;
      const slotW = b.necessity ?? 0.2;
      const add = bestW * slotW;
      acc[b.itemId] ??= { value: 0, comps: new Map(), carriers: new Map() };
      acc[b.itemId].value += add;
      acc[b.itemId].carriers.set(u.unitId, (acc[b.itemId].carriers.get(u.unitId) ?? 0) + add);
      for (const c of info.comps) {
        acc[b.itemId].comps.set(c.slug, (acc[b.itemId].comps.get(c.slug) ?? 0) + add * 0.3);
      }
    }
  }
  const max = Math.max(...Object.values(acc).map((a) => a.value), 0.001);
  return Object.entries(acc)
    .map(([id, a]) => {
      const topComps = [...a.comps.entries()].sort((x, y) => y[1] - x[1]).slice(0, 3)
        .map(([slug]) => ({ slug, name: compName(slug) }));
      const topCarriers = [...a.carriers.entries()].sort((x, y) => y[1] - x[1]).slice(0, 4)
        .map(([uid]) => ({ id: uid, name: unitName(uid) }));
      const meta = items.value.find((i) => i.id === id);
      return {
        id,
        name: meta?.name ?? id,
        icon: meta?.icon_url ?? "",
        value: Math.round((a.value / max) * 100),
        bestComps: topComps,
        bestCarriers: topCarriers,
      };
    })
    .sort((a, b) => b.value - a.value);
});
/** 开局推荐：选装价值最高的散件 Top3 */
const openingRecommendations = computed(() => scatterScores.value.slice(0, 3));

function overlayOpenComp(slug: string) {
  overlaySelectedComp.value = overlaySelectedComp.value === slug ? null : slug;
}

/** overlay 阵容详情的分阶段攻略（复用 compStageGuide） */
const overlayStageGuide = computed(() =>
  overlaySelectedComp.value ? compStageGuide(overlaySelectedComp.value) : null
);

// ===== 升星检索：持有的核心还差几张到 3 星 + 当前等级刷新期望 =====
// 手动记录已持有的张数（点 +/-），估算还差几张、以及当前等级刷多少次能出。
const upgradeHeld = ref<Record<string, number>>({}); // unitId -> 已持有张数
const upgradeUnit = ref<string | null>(null); // 当前追踪的棋子

function setUpgradeHeld(unitId: string, n: number) {
  const d = n <= 0 ? 0 : n;
  upgradeHeld.value = { ...upgradeHeld.value, [unitId]: d };
}
function bumpUpgrade(unitId: string, delta: number) {
  const cur = upgradeHeld.value[unitId] ?? 0;
  setUpgradeHeld(unitId, Math.max(0, cur + delta));
}

/** 费用池大小（该费用的棋子总数，粗略按第一套阵容的 unitCategories） */
function poolSize(cost: number): number {
  const cat = tftData.value?.compDetails[Object.keys(tftData.value.compDetails)[0]]?.unitCategories;
  return cat?.[`cost_${cost}`]?.length ?? 10;
}

/** 当前等级每次刷新（5 格）中出现目标棋子的期望张数 */
function expectedPerRoll(level: number, cost: number): number {
  const odds = shopOdds.value.find((l) => l.level === level)?.odds;
  if (!odds) return 0;
  const pCost = (odds[cost - 1] ?? 0) / 100; // 单格是该费用的概率
  const pUnit = pCost / poolSize(cost); // 再均分到该费用的每张棋子
  return pUnit * 5; // 每次刷新 5 格
}

/** 升星检索结果：差几张到 3 星 / 期望刷次 / 当前持有 */
const upgradeInfo = computed(() => {
  const uid = upgradeUnit.value;
  if (!uid || !tftData.value) return null;
  const cost = unitCost(uid);
  const held = upgradeHeld.value[uid] ?? 0;
  const to2 = Math.max(0, 3 - held); // 2 星 = 3 张
  const to3 = Math.max(0, 9 - held); // 3 星 = 9 张
  const lv = live.value.level ?? 7;
  const perRoll = expectedPerRoll(lv, cost);
  return {
    unitId: uid,
    name: unitName(uid),
    icon: unitIcon(uid),
    cost,
    held,
    to2,
    to3,
    perRoll,
    // 刷多少次期望凑够 3 星
    rollsTo3: perRoll > 0 ? Math.ceil(to3 / perRoll) : null,
    lv,
  };
});
/** 可追踪的核心候选（Pin 阵容的优先核心前 6） */
const upgradeCandidates = computed(() => {
  const slug = effectivePin.value ?? overlaySelectedComp.value;
  if (!slug || !tftData.value) return [];
  const d = tftData.value.compDetails[slug];
  if (!d) return [];
  const prio = new Map(d.composition.priorityMatrixRows.map((p) => [p.unitId, p.unitPriority ?? 0]));
  return [...d.composition.overview.coreUnits]
    .sort((a, b) => (prio.get(b.id) ?? 0) - (prio.get(a.id) ?? 0))
    .slice(0, 6)
    .map((u) => ({ id: u.id, name: u.nameCn || u.name, cost: u.cost, held: upgradeHeld.value[u.id] ?? 0 }));
});

/** 装备合成路线：热门成型装 → 所需散件（来自 CDragon items.composition） */
const itemRecsById = computed(() => {
  const m: Record<string, { name: string; icon_url: string; composition: string[] }> = {};
  for (const i of items.value) m[i.id] = i;
  return m;
});
const overlayCraftables = computed(() => {
  if (!tftData.value) return [];
  const recs = overlayItemRecs.value;
  const out: { id: string; comps: string[] }[] = [];
  for (const r of recs) {
    const full = itemRecsById.value[r.itemId];
    const comps = full?.composition?.filter((c) => c) ?? [];
    if (comps.length) out.push({ id: r.itemId, comps });
  }
  return out.slice(0, 5);
});

/** 连胜连败：从战绩库最近 N 局推断当前走势 */
const overlayStreak = computed(() => {
  if (!matches.value.length) return null;
  // matches 按时间倒序，取最近 8 局，从最近一局向前数连续同倾向局数
  const places = matches.value.map((m) => m.placement).filter((p): p is number => p != null).slice(0, 8);
  if (!places.length) return null;
  const first = places[0];
  const isWin = first <= 4;
  let count = 0;
  for (const p of places) {
    if ((p <= 4) === isWin) count++;
    else break;
  }
  if (isWin) {
    return count >= places.length
      ? { type: "win", count, text: `近 ${count} 局均前四，状态火热` }
      : { type: "win", count, text: `已连续 ${count} 局前四，可激进些` };
  }
  return count >= places.length
    ? { type: "loss", count, text: `近 ${count} 局未进前四，建议稳节奏` }
    : { type: "loss", count, text: `已连续 ${count} 局未进前四，注意血量与经济` };
});

// ===== 对局内推荐（meta 数据 + 实时状态 → 建议） =====

/** 解码 teamCode（如 "023FC3FB...TFTSet18"）→ 每单位位置数组。
 * Riot 格式：去掉 "TFTSet\d+" 后缀后，
 * - 前 2 nibble = 固定头 (02)
 * - 之后每 3 nibble (12 bits) = 一个单位
 * - 12 bits 的低 5 位 = 位置编码：0..27 = 棋盘 7列×4行（row=pos/7, col=pos%7），28+ = 替补席
 * 返回与 coreUnits 顺序对齐的位置数组（0=棋盘空, -1=替补席）。
 */
function parseTeamCode(code: string | undefined | null): number[] {
  const body = (code ?? "").replace(/TFTSet\d+/i, "");
  const nibbles = body.split("").map((c) => parseInt(c, 16));
  const out: number[] = [];
  if (nibbles.length < 5) return out;
  for (let i = 2; i + 2 < nibbles.length; i += 3) {
    const v = nibbles[i] * 256 + nibbles[i + 1] * 16 + nibbles[i + 2];
    if (v === 0) continue; // padding 0
    out.push(v % 32);
  }
  return out;
}

/** 由位置值生成 28 格棋盘映射：grid[i] = 核心棋子序号（1..n），i = row*7+col；替补席序号为负。 */
function buildBoardGrid(positions: number[]): { grid: number[]; bench: number[] } {
  const grid = new Array(28).fill(0);
  const bench: number[] = [];
  positions.forEach((pos, idx) => {
    if (pos >= 0 && pos < 28) grid[pos] = idx + 1;
    else if (pos >= 28) bench.push(idx + 1);
  });
  return { grid, bench };
}

function parseStage(stage: string): { major: number; minor: number } {
  const m = /^(\d+)-(\d+)$/.exec(stage);
  if (!m) return { major: 0, minor: 0 };
  return { major: parseInt(m[1], 10), minor: parseInt(m[2], 10) };
}

/** 判断 stage 字符串（如 "3-5"）是否落在区间字符串（如 "3-1 ~ 3-7" 或 "6-1+"）内 */
function stageInRange(range: string, stage: string): boolean {
  const cur = parseStage(stage);
  const t = cur.major * 100 + cur.minor;
  if (!t) return false;
  const m = /(\d+)-(\d+)(?:\s*~\s*(\d+)-(\d+)|\s*\+)?/.exec(range);
  if (!m) return false;
  const start = parseInt(m[1], 10) * 100 + parseInt(m[2], 10);
  if (m[4]) {
    const end = parseInt(m[3], 10) * 100 + parseInt(m[4], 10);
    return t >= start && t <= end;
  }
  return t >= start;
}

/** 标准经济节奏下，某 stage 通常应达到的等级（2-1≈4级 → 4-5≈8级 → 5-1≈9级） */
function expectedLevelForStage(stage: string): number {
  const { major, minor } = parseStage(stage);
  if (major <= 1) return 4;
  if (major === 2) return minor >= 5 ? 5 : 4;
  if (major === 3) return minor >= 5 ? 7 : 6;
  if (major === 4) return minor >= 5 ? 8 : 7;
  return 9;
}

/** 根据实时状态生成 3 条以内的经济/血量建议 */
const overlayAdvice = computed(() => {
  if (!live.value.available) return [];
  const tips: { level: "good" | "warn" | "bad"; text: string }[] = [];
  const gold = live.value.gold ?? -1;
  const hp = live.value.health ?? -1;
  const lv = live.value.level ?? -1;
  const stage = live.value.stage;
  const { major } = parseStage(stage);

  // 金币建议（阶段 1 金币少是正常的，不提示）
  if (gold >= 0 && major >= 2) {
    if (gold >= 50) tips.push({ level: "good", text: `金币 ${gold}，利息已拉满，可放心消费` });
    else if (gold >= 40) tips.push({ level: "good", text: `金币 ${gold}，接近 50 利息线，尽量别破 40` });
    else if (gold >= 30) tips.push({ level: "warn", text: `金币 ${gold}，注意保住 30 利息` });
    else tips.push({ level: "bad", text: `金币 ${gold}，经济紧张，避免过度消费` });
  }

  // 等级 vs 标准节奏（阶段 1 跳过，此时等级通常是正常的）
  if (stage && lv > 0 && major >= 2) {
    const exp = expectedLevelForStage(stage);
    if (lv < exp) tips.push({ level: "warn", text: `${stage} 通常已到 ${exp} 级，当前 ${lv} 级略慢` });
    else if (lv > exp) tips.push({ level: "good", text: `${stage} 就到 ${lv} 级，等级节奏领先` });
  }

  // 血量风险（所有阶段）
  if (hp >= 0) {
    if (hp < 20) tips.push({ level: "bad", text: `仅剩 ${Math.round(hp)} HP，必须全力保命` });
    else if (hp < 40) tips.push({ level: "warn", text: `${Math.round(hp)} HP，注意血量，考虑提速` });
  }

  return tips.slice(0, 3);
});

/** 推荐阵容：top3 的 meta 阵容（含图标与 AVP） */
const overlayCompRecs = computed(() => {
  if (!tftData.value) return [];
  return [...tftData.value.compAtlas.compositionSummaries]
    .sort((a, b) => a.avgPlacementValue - b.avgPlacementValue)
    .slice(0, 3)
    .map((c) => ({
      slug: c.slug,
      name: compName(c.slug),
      icon: compCardIcon(c.slug),
      avp: c.avgPlacementValue,
    }));
});

function phaseLabel(p: string) {
  const map: Record<string, string> = {
    Lobby: "大厅",
    Matchmaking: "匹配中",
    ReadyCheck: "对局确认",
    ChampSelect: "选人阶段",
    InProgress: "对局进行中",
    PreEndOfGame: "结算中",
    EndOfGame: "对局结束",
  };
  return map[p] || p || "—";
}

// ===== 对局状态（主窗口也用） =====
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

const statsQueue = ref<number | "all">("all");
async function openStats(queue: number | "all" = statsQueue.value) {
  view.value = "stats";
  statsQueue.value = queue;
  statsLoading.value = true;
  error.value = "";
  try {
    stats.value = await invoke<Stats>(
      "get_stats",
      queue === "all" ? {} : { queue }
    );
  } catch (e) {
    error.value = String(e);
  } finally {
    statsLoading.value = false;
  }
}

const dataTab = ref<
  "overview" | "champions" | "traits" | "items" | "augments" | "wisps" | "loot" | "gold" | "reroll" | "odds" | "rules" | "econ"
>("overview");
const dataLoading = ref(false);
const items = ref<ItemData[]>([]);
const champions = ref<ChampionData[]>([]);
const traits = ref<TraitData[]>([]);
const augments = ref<ItemData[]>([]);
const shopOdds = ref<ShopOddsLevel[]>([]);
const gameRules = ref<RulesSection[]>([]);
// 经济节奏时间轴（标准参考，非实时）。优先取 tftable_data.patchTables（随补丁更新），缺失用内置默认。
const DEFAULT_ECON_TIMELINE: {
  stage: string;
  targetLevel: number;
  phase_label: string;
  items: { label: string; desc: string }[];
}[] = [
  {
    stage: "1-1 ~ 1-4",
    targetLevel: 4,
    phase_label: "法术开局 · 攒钱",
    items: [
      { label: "目标等级", desc: "1-1 升 4 级（免费），正常 1-3 前后到 4 级" },
      { label: "金币", desc: "4 人开局约 0-10 金，主要靠野怪掉落" },
      { label: "拿弈子", desc: "围绕 1-2 费核心过渡，拿能与成型阵容挂钩的棋子" },
      { label: "策略", desc: "不花钱重投商店，尽量自然成型；吃基础利息" },
    ],
  },
  {
    stage: "2-1 ~ 2-4",
    targetLevel: 5,
    phase_label: "蓄力 · 目标 20 金",
    items: [
      { label: "目标等级", desc: "2-1 前后升到 5 级" },
      { label: "金币", desc: "目标攒到 20 金吃 2 利息" },
      { label: "升级", desc: "2-1 或 2-2 升 5（30 金内）；若血量健康可稍等" },
      { label: "拿弈子", desc: "锁定核心棋子，2 费弈子开始出现" },
    ],
  },
  {
    stage: "2-5 ~ 2-7",
    targetLevel: 5,
    phase_label: "冲刺 30 金 · 确定方向",
    items: [
      { label: "目标等级", desc: "保持 5 级，观察是否有人抢牌" },
      { label: "金币", desc: "目标攒到 30 金吃 3 利息" },
      { label: "升级", desc: "2-6/2-7 视血量决定是否抢升 6 级" },
      { label: "策略", desc: "确定成型方向，开始囤核心弈子的 2 星" },
    ],
  },
  {
    stage: "3-1 ~ 3-4",
    targetLevel: 6,
    phase_label: "关键转折 · 30-50 金",
    items: [
      { label: "目标等级", desc: "3-1 前升到 6 级（标准节奏）" },
      { label: "金币", desc: "3-4 前后攒到 40-50 金" },
      { label: "升级", desc: "3-1 升 6（约 40 金时）；升级后 D 牌 2-3 次找核心" },
      { label: "血量", desc: "若 <30 HP，放弃部分经济提前抢节奏" },
    ],
  },
  {
    stage: "3-5 ~ 3-7",
    targetLevel: 7,
    phase_label: "决定存钱 or 提速",
    items: [
      { label: "目标等级", desc: "6 级稳定，开始决定 7 级节奏" },
      { label: "金币", desc: "根据血量和牌的情况目标 50 金" },
      { label: "策略", desc: "血量健康 → 存钱到 50；血量危险 → 立即抢牌提质量" },
    ],
  },
  {
    stage: "4-1 ~ 4-4",
    targetLevel: 7,
    phase_label: "冲 7 级 · 核心成型",
    items: [
      { label: "目标等级", desc: "4-1 到 4-2 升 7 级" },
      { label: "金币", desc: "7 级后 D 牌找核心 3 费/4 费弈子" },
      { label: "策略", desc: "阵容成型关键期，星级做完后再考虑 8 级" },
    ],
  },
  {
    stage: "4-5 ~ 4-7",
    targetLevel: 8,
    phase_label: "决胜负 · 8 级",
    items: [
      { label: "目标等级", desc: "4-5 到 5-1 前后升 8 级" },
      { label: "金币", desc: "8 级后 D 高费核心，2 星 5 费开始出现" },
      { label: "策略", desc: "若血量健康可以慢慢上 9；否则全力 D 卡保名次" },
    ],
  },
  {
    stage: "5-1 ~ 6-1",
    targetLevel: 9,
    phase_label: "决赛圈 · 拉满",
    items: [
      { label: "目标等级", desc: "视经济和血量决定 9 级冲刺或稳住 8 级" },
      { label: "金币", desc: "高费 2 星是关键，5 费核心 2 星决定上限" },
      { label: "策略", desc: "血量/装备偏差时保 4 即可；优势局冲 9 找版本答案" },
    ],
  },
];

const econTimeline = computed(() => {
  const pt = tftData.value?.patchTables?.econ_timeline;
  if (Array.isArray(pt) && pt.length) return pt;
  return DEFAULT_ECON_TIMELINE;
});

// 按等级参考表：每个等级对应标准阶段 / 场上人数 / 该等级该做的事
const DEFAULT_LEVEL_GUIDE: {
  level: number;
  stage: string;
  board: number;
  focus: string;
  odds: number[]; // 1~5 费概率（从 shopOdds 取）
}[] = [
  { level: 1, stage: "1-1", board: 2, focus: "开局拿 1 费过渡牌，不花钱刷新", odds: [100, 0, 0, 0, 0] },
  { level: 2, stage: "1-1 ~ 1-2", board: 3, focus: "补到 3 人，吃基础利息", odds: [100, 0, 0, 0, 0] },
  { level: 3, stage: "1-2 ~ 1-3", board: 4, focus: "上 4 人，自然成型，不花钱", odds: [75, 25, 0, 0, 0] },
  { level: 4, stage: "1-3 ~ 2-1", board: 5, focus: "2-1 前上 5 人；确定阵容方向，拿 1-2 费核心", odds: [55, 30, 15, 0, 0] },
  { level: 5, stage: "2-1 ~ 2-5", board: 6, focus: "2-5 前后升 6 人；2 费核心开始出现", odds: [45, 33, 20, 2, 0] },
  { level: 6, stage: "3-1 ~ 3-5", board: 7, focus: "3-1 升 7 人；找 3 费核心，可小 D 出 2 星", odds: [30, 40, 25, 5, 0] },
  { level: 7, stage: "3-5 ~ 4-2", board: 8, focus: "4-1 前后升 8 人；核心 3/4 费成型，开始搜牌", odds: [19, 35, 35, 10, 1] },
  { level: 8, stage: "4-5 ~ 5-1", board: 9, focus: "4-5 升 9 人；4/5 费核心，保命 D 卡", odds: [18, 25, 36, 18, 3] },
  { level: 9, stage: "5-1+", board: 9, focus: "冲 9 级找 5 费 2 星定上限", odds: [10, 20, 25, 35, 10] },
];

const levelGuide = computed(() => {
  const pt = tftData.value?.patchTables?.level_guide;
  if (Array.isArray(pt) && pt.length) return pt;
  return DEFAULT_LEVEL_GUIDE;
});

/** 对局中：当前 stage 对应的阶段参考行（用于高亮） */
const currentEconRow = computed(() => {
  if (!live.value.available || !live.value.stage) return null;
  const { major, minor } = parseStage(live.value.stage);
  const t = major * 10 + minor;
  for (const row of econTimeline.value) {
    const m = /(\d+)-(\d+)/.exec(row.stage);
    if (!m) continue;
    const start = parseInt(m[1], 10) * 10 + parseInt(m[2], 10);
    const endM = /(\d+)-(\d+)/.exec(row.stage.replace(/.*~\s*/, ""));
    const end = endM ? parseInt(endM[1], 10) * 10 + parseInt(endM[2], 10) : start;
    if (t >= start && t <= end) return row;
  }
  return null;
});
/** 对局中：当前等级对应的等级参考行（用于高亮） */
const currentLevelGuideRow = computed(() => {
  if (!live.value.available || !live.value.level) return null;
  return levelGuide.value.find((g) => g.level === live.value.level) ?? null;
});

// ===== 阵容 × 阶段攻略（每套阵容每个阶段的参考） =====
// 依据 compCost 定运营类型，再按核心棋子费用结构给每阶段"该拿什么/怎么升/怎么搜/装备重点"。

/** 运营类型：compCost 1-2 = 低费重投(reroll)，3 = 中费重投/标准，4 = 标准(fast8)，5 = 高费(fast9/95) */
function compPlaystyle(compCost: number): {
  key: "reroll" | "mid" | "fast8" | "fast9";
  label: string;
  levelPlan: number[]; // 每个阶段建议达到的等级
  econHint: string[];
  rollHint: string[];
  itemHint: string[];
} {
  if (compCost <= 2)
    return {
      key: "reroll",
      label: "低费重投",
      levelPlan: [4, 5, 6, 6, 7, 8],
      econHint: ["开局能拿的核心低费全拿", "攒钱到 20-30，别乱花", "卡 50 利息，只在 6 级时大搜", "3 星核心成型前不冲 8", "核心 3 星后可升 7-8 补质量", "决赛圈冲 9 找 5 费上限"],
      rollHint: ["自然升级，不搜牌", "2-1 前只拿自然出现的核心", "3-1 前后升 6 并开始搜 1-2 费", "6 级持续搜出 2-3 费核心 3 星", "核心 3 星后停止搜，存钱升级", "只搜缺的 5 费补强"],
      itemHint: ["抢核心需要的散件开局", "优先合核心的 Core 装", "多余散件合功能装", "成装给核心，散件留替补", "核心装做完可合前排装", "按优先级矩阵补剩余成装"],
    };
  if (compCost === 3)
    return {
      key: "mid",
      label: "中费重投/标准",
      levelPlan: [4, 5, 6, 7, 7, 9],
      econHint: ["拿 1-2 费过渡，留 3 费核心", "攒钱到 30，找 3 费核心", "3 费核心 2 星后升 7", "7 级小搜 3-4 费质量", "攒 50 利息再升 8", "决赛圈冲 9 或 D 5 费"],
      rollHint: ["不搜，靠自然出现", "2 费核心开始出现，锁定", "3-1 前后少量 D 找 3 费", "3 费核心 2 星即收手", "4-1 升 7 后 D 找 4 费", "8 级后按需搜 5 费"],
      itemHint: ["抢 3 费核心的散件", "优先 3 费核心 Core 装", "成装先给 3 费核心", "2 星后可分给前排", "4 费核心进场再给装", "补前排防御装"],
    };
  if (compCost === 4)
    return {
      key: "fast8",
      label: "标准 8 级成型",
      levelPlan: [4, 5, 6, 7, 8, 9],
      econHint: ["拿能转的 1-2 费过渡", "攒钱到 40-50", "3-1 升 6，保血量", "4-1 升 7 找 4 费核心", "4-5 前后升 8 大搜", "决赛圈 D 5 费或升 9"],
      rollHint: ["过渡为主，不追核心", "1 费核心可顺带 2 星", "少量 D 找 3 费过渡", "7 级找 4 费核心 1-2 张", "8 级全力搜 4 费核心 2 星", "5 费 2 星决定上限"],
      itemHint: ["过渡牌随便给装保血", "成装先给过渡 carry", "4 费核心到手前不给终装", "4 费核心 2 星后全给", "补 5 费功能装", "前排装优先"],
    };
  return {
    key: "fast9",
    label: "高费 9 级冲顶",
    levelPlan: [4, 5, 6, 7, 8, 9],
    econHint: ["过渡牌尽量少花钱", "极限攒钱 50", "尽量不 D，靠自然升级", "4-2 前冲 8，少 D", "8 级小 D 保血再冲 9", "9 级 D 5 费核心 2 星"],
    rollHint: ["低费过渡，不绑定", "2 费过渡拿能转的", "3 费过渡 2 星即可", "7 级只保血不搜", "8 级少量 D 找核心", "9 级全力 D 5 费"],
    itemHint: ["散件先留，不急着合", "过渡装可给但别锁死", "忍住不合成品装", "给保血过渡装", "核心 5 费到位再合成品", "全力 5 费核心装"],
  };
}

/** 每套阵容的阶段攻略：返回 6 个阶段的行（阶段 / 建议等级 / 拿什么 / 升级 / 搜牌 / 装备） */
function compStageGuide(slug: string):
  | {
      playstyle: string;
      coreCosts: string;
      stages: {
        key: string;
        stage: string;
        targetLevel: number;
        take: { id: string; name: string; cost: number; core: boolean }[];
        level: string;
        roll: string;
        item: string;
      }[];
    }
  | null {
  if (!tftData.value) return null;
  const d = tftData.value.compDetails[slug];
  if (!d) return null;
  const ov = d.composition.overview;
  const card = cardOf(slug);
  const ps = compPlaystyle(card.compCost);
  // 优先级排序后的核心棋子
  const prio = new Map(d.composition.priorityMatrixRows.map((p) => [p.unitId, p.unitPriority ?? 0]));
  const units = [...ov.coreUnits].sort(
    (a, b) => (prio.get(b.id) ?? 0) - (prio.get(a.id) ?? 0) || a.cost - b.cost
  );
  // 每阶段的重点费用段（由运营类型与当前等级推导）
  const stageCost: Record<string, [number, number]> = {
    s1: [1, 2],
    s2: [1, 3],
    s3: [1, 3],
    s4: [2, 4],
    s5: [2, 5],
    s6: [3, 5],
  };
  const stageLabels = [
    { key: "s1", stage: "1-1 ~ 1-4", label: "开局" },
    { key: "s2", stage: "2-1 ~ 2-7", label: "定方向" },
    { key: "s3", stage: "3-1 ~ 3-7", label: "关键转折" },
    { key: "s4", stage: "4-1 ~ 4-7", label: "成型期" },
    { key: "s5", stage: "5-1 ~ 5-7", label: "强化期" },
    { key: "s6", stage: "6-1+", label: "决赛圈" },
  ];
  const stages = stageLabels.map((s, i) => {
    const [minC, maxC] = stageCost[s.key];
    const take = units
      .filter((u) => u.cost >= minC && u.cost <= maxC)
      .slice(0, 5)
      .map((u) => ({ id: u.id, name: u.nameCn || u.name, cost: u.cost, core: (prio.get(u.id) ?? 0) >= 3 }));
    return {
      key: s.key,
      stage: s.stage,
      targetLevel: ps.levelPlan[i],
      take,
      level: `${s.label}：建议 ${ps.levelPlan[i]} 级`,
      roll: ps.rollHint[i],
      item: ps.itemHint[i],
    };
  });
  return {
    playstyle: ps.label,
    coreCosts: [...new Set(units.map((u) => u.cost))].sort((a, b) => a - b).join("/"),
    stages,
  };
}
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

// ---- CDN 数据统一缓存加载 ----
let cdnDataLoaded = false;
async function ensureCdnData(force: boolean = false) {
  if (cdnDataLoaded && !force) return;
  const [c, t, a, i, o, r] = await Promise.all([
    invoke<ChampionData[]>("get_champions"),
    invoke<TraitData[]>("get_traits"),
    invoke<ItemData[]>("get_augments"),
    invoke<ItemData[]>("get_items"),
    invoke<ShopOddsLevel[]>("get_shop_odds"),
    invoke<RulesSection[]>("get_game_rules"),
  ]);
  champions.value = c;
  traits.value = t;
  augments.value = a;
  items.value = i;
  shopOdds.value = o;
  gameRules.value = r;
  cdnDataLoaded = true;
}

async function openData(force: boolean = false) {
  view.value = "data";
  dataLoading.value = true;
  error.value = "";
  try {
    await ensureCdnData(force);
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
// 图标路径按当前补丁推导赛季号（如 patch "18.3b" → set18），避免赛季更替后 404。
const setSlug = computed(() => {
  const patch = tftData.value?.patch ?? tftData.value?.metaStats?.metadata?.patch ?? "";
  const m = /^(\d+)/.exec(patch.trim());
  return m ? `set${m[1]}` : "set18";
});
function unitIcon(id: string): string { return id ? `${TF}/${setSlug.value}/avatar-webp/${id}.webp?v=5` : ""; }
function itemIcon(id: string): string { return id ? `${TF}/${setSlug.value}/item-webp/${id}.webp?v=4` : ""; }
function traitIcon(id: string): string { return id ? `${TF}/${setSlug.value}/trait-svg/${id}.svg?v=1` : ""; }

// ===== 设置页 =====
const updateCheck = ref<{
  checking: boolean;
  has_update: boolean;
  local_patch: string | null;
  latest_patch: string;
  sample_size: number;
  updated_at: string;
}>({ checking: false, has_update: false, local_patch: null, latest_patch: "", sample_size: 0, updated_at: "" });

async function checkForUpdate() {
  updateCheck.value.checking = true;
  error.value = "";
  try {
    const r = await invoke<any>("check_tftable_update");
    updateCheck.value = { checking: false, ...r };
  } catch (e) {
    error.value = String(e);
    updateCheck.value.checking = false;
  }
}

// 在线下载并重建 tftable_data.json
const updating = ref(false);
async function downloadUpdate() {
  updating.value = true;
  error.value = "";
  try {
    const r = await invoke<any>("update_tftable_data");
    if (r.success) {
      showToast(`数据已更新到 ${r.patch}（${r.sample_size.toLocaleString()} 样本）`);
      await getMeta(true);
      await checkForUpdate();
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    updating.value = false;
  }
}

// ===== 名称映射（中文优先） =====
function showCompDetail(slug: string) {
  selectedCompSlug.value = slug;
  view.value = "compList";
}
function closeCompDetail() {
  selectedCompSlug.value = null;
}
const selectedCompDetail = computed(() => {
  if (!selectedCompSlug.value || !tftData.value) return null;
  return tftData.value.compDetails[selectedCompSlug.value] ?? null;
});

/** 主窗口阵容详情：站位图（teamCode 解码） */
const compPosCells = computed(() => {
  if (!selectedCompDetail.value) return [];
  const ov = selectedCompDetail.value.composition.overview;
  const { grid } = buildBoardGrid(parseTeamCode(ov.teamCode));
  return grid.map((n) => (n >= 1 && n <= ov.coreUnits.length ? ov.coreUnits[n - 1].id : null));
});
const compPosBench = computed(() => {
  if (!selectedCompDetail.value) return [];
  const ov = selectedCompDetail.value.composition.overview;
  const { bench } = buildBoardGrid(parseTeamCode(ov.teamCode));
  return bench;
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

// 数据缓存：同一会话只加载一次 tftable_data + CDragon，切视图不再重复下载
let metaDataLoaded = false;
const dataFetchedAt = ref<number | null>(null);
let autoRefreshTimer: ReturnType<typeof setInterval> | null = null;
// 数据过期阈值（10 分钟静默刷新一次）
const DATA_STALE_MS = 10 * 60 * 1000;
// 数据是否已过期（用于自动刷新提示）
const dataStale = computed(() => {
  if (!dataFetchedAt.value || !tftData.value) return false;
  return Date.now() - dataFetchedAt.value > DATA_STALE_MS;
});

async function getMeta(force: boolean = false) {
  if (metaDataLoaded && !force && tftData.value) return;
  metaLoading.value = true;
  error.value = "";
  try {
    const [r] = await Promise.all([
      invoke<TFTtableData>("get_tftable_data"),
      ensureCdnData(force),
    ]);
    tftData.value = r;
    dataFetchedAt.value = Date.now();
    metaDataLoaded = true;
    lastSync.value = r.sampleSize > 0 ? `${r.metaStats.metadata.patch} · ${r.sampleSize.toLocaleString()} 样本` : "";
  } catch (e) {
    error.value = String(e);
    tftData.value = null;
  } finally {
    metaLoading.value = false;
  }
}

// 自动刷新：定时静默检查数据是否超过 10 分钟，过期则后台强制刷新
function startAutoRefresh() {
  if (autoRefreshTimer) clearInterval(autoRefreshTimer);
  autoRefreshTimer = setInterval(() => {
    if (dataStale.value && !metaLoading.value) {
      getMeta(true);
    }
  }, 5 * 60 * 1000);
}

// overlay 的实时轮询定时器（对局中每 5s 刷新段位与 2999 实时状态）
let overlayTimer: ReturnType<typeof setInterval> | null = null;

onBeforeUnmount(() => {
  if (autoRefreshTimer) clearInterval(autoRefreshTimer);
  if (overlayTimer) clearInterval(overlayTimer);
  autoRefreshTimer = null;
  overlayTimer = null;
});

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
/** 是否派生变体阵容（主阵容 + "__v{n}" 后缀生成） */
function isVariantComp(slug: string): boolean {
  return /__v\d+$/.test(slug);
}
/** 变体所属的主阵容 slug（去掉 __vN 后缀） */
function variantBaseOf(slug: string): string {
  return slug.replace(/__v\d+$/, "");
}
function compCardIcon(slug: string): string {
  const c = cardOf(slug);
  return c.compIcon ? tfImg(c.compIcon) : unitIcon(slug);
}
function costShareArr(obj: Record<string, number>): number[] {
  return [1, 2, 3, 4, 5].map((k) => obj[String(k)] ?? 0);
}

// ===== 前期（Early）：通用低费过渡英雄池 =====
// 前期不应绑定某个具体成型阵容，而是展示每个费用段可用作过渡的弈子
// 及其核心/重要装备、关联的成型阵容方向，方便玩家开局灵活拿子。
const earlyCost = ref<1 | 2 | 3 | "all">("all");
const earlyUnits = computed(() => {
  if (!tftData.value) return [];
  return tftData.value.unitItems
    .filter((u) => earlyCost.value === "all" || u.unitCost === earlyCost.value)
    .sort((a, b) => a.unitCost - b.unitCost || unitName(a.unitId).localeCompare(unitName(b.unitId)));
});
// 每个低费弈子关联的成型阵容（点击可跳到阵容详情）
function earlyCompsFor(unit: TFUnitItem): string[] {
  return (unit.coreCompStrengths || []).map((c) => c.comp_key);
}
// 低费阵容（1-3费为主力）同样作为早期目标参考
const earlyGoalComps = computed(() => {
  if (!tftData.value) return [];
  return tftData.value.compositionCards
    .filter((c) => c.compCost <= 3)
    .sort((a, b) => a.averagePlacementValue - b.averagePlacementValue);
});

// ===== 阵容构建器（Team Builder） =====
const builderSearch = ref("");
const builderSelected = ref<string[]>([]);
const builderUnits = computed(() => {
  if (!tftData.value) return [];
  const cat = tftData.value.compDetails[Object.keys(tftData.value.compDetails)[0]].unitCategories;
  const ids: string[] = [];
  for (const k of ["cost_1", "cost_2", "cost_3", "cost_4", "cost_5"]) {
    for (const id of cat[k] || []) ids.push(id);
  }
  return ids;
});
const filteredBuilderUnits = computed(() => {
  const q = builderSearch.value.toLowerCase();
  return builderUnits.value.filter(
    (id) => !q || unitName(id).toLowerCase().includes(q) || unitNameEn(id).toLowerCase().includes(q)
  );
});
function toggleBuilderUnit(id: string) {
  const i = builderSelected.value.indexOf(id);
  if (i >= 0) builderSelected.value.splice(i, 1);
  else builderSelected.value.push(id);
}
// 已选弈子的费用统计
const builderCostCount = computed(() => {
  const m: Record<number, number> = { 1: 0, 2: 0, 3: 0, 4: 0, 5: 0 };
  for (const id of builderSelected.value) {
    const u = (NAME_MAP.units as Record<string, any>)[id];
    if (u) m[u.c] = (m[u.c] ?? 0) + 1;
  }
  return m;
});
// 匹配该阵容的关联阵容
const builderMatchingComps = computed(() => {
  if (!tftData.value || builderSelected.value.length === 0) return [];
  const sel = new Set(builderSelected.value);
  const out: { slug: string; matched: number; total: number; units: string[] }[] = [];
  for (const slug of Object.keys(tftData.value.compDetails)) {
    const units = tftData.value.compDetails[slug].composition.overview.coreUnits.map((u) => u.id);
    const matched = units.filter((id) => sel.has(id)).length;
    if (matched >= 2) {
      out.push({ slug, matched, total: units.length, units });
    }
  }
  return out.sort((a, b) => b.matched - a.matched).slice(0, 8);
});

// ===== 数据检索器（Data Explorer） =====
const explorerTab = ref<"units" | "items" | "comps">("units");
const explorerUnitCost = ref<number | "all">("all");
const explorerUnitSearch = ref("");
const explorerUnitRows = computed(() => {
  if (!tftData.value) return [];
  let rows = tftData.value.unitItems;
  if (explorerUnitCost.value !== "all") rows = rows.filter((u) => u.unitCost === explorerUnitCost.value);
  const q = explorerUnitSearch.value.toLowerCase();
  if (q) rows = rows.filter((u) => unitName(u.unitId).toLowerCase().includes(q) || unitNameEn(u.unitId).toLowerCase().includes(q));
  return rows;
});
const explorerItemRows = computed(() => {
  if (!tftData.value) return [];
  return tftData.value.itemGuide.items;
});
const explorerCompSearch = ref("");
const explorerCompRows = computed(() => {
  if (!tftData.value) return [];
  const q = explorerCompSearch.value.toLowerCase();
  return tftData.value.compAtlas.compositionSummaries.filter(
    (c) => !q || compName(c.slug).toLowerCase().includes(q) || c.slug.includes(q)
  );
});

// ===== 羁绊统计页 =====
const traitSearch = ref("");
const _traitSearchQ = ref("");
let _traitTimer = 0;
function setTraitSearch(v: string) {
  traitSearch.value = v;
  clearTimeout(_traitTimer);
  _traitTimer = setTimeout(() => (_traitSearchQ.value = v), 200);
}
const traitCountByCost = computed(() => {
  const m: Record<string, { cost: number[]; count: number }> = {};
  for (const ch of champions.value) {
    for (const tr of ch.traits) {
      (m[tr] ??= { cost: [], count: 0 });
      m[tr].cost.push(ch.cost);
      m[tr].count++;
    }
  }
  return m;
});
const traitWinShareMap = computed(() => {
  const m: Record<string, { win: number; top4: number; tier: number | null; unitsRequired: number | null }> = {};
  if (!tftData.value) return m;
  const ws = tftData.value.metaStats.winShare.traits;
  const t4 = tftData.value.metaStats.top4Share.traits;
  for (const t of ws) m[t.trait_id] = { win: t.wins, top4: 0, tier: t.tier, unitsRequired: t.units_required };
  for (const t of t4) {
    (m[t.trait_id] ??= { win: 0, top4: 0, tier: t.tier, unitsRequired: t.units_required }).top4 = t.wins;
  }
  return m;
});
// 每个羁绊关联的阵容
const traitCompsMap = computed(() => {
  const m: Record<string, string[]> = {};
  if (!tftData.value) return m;
  for (const slug of Object.keys(tftData.value.compDetails)) {
    const trs = tftData.value.compDetails[slug].composition.overview.traits.map((t) => t.id);
    for (const tr of trs) (m[tr] ??= []).push(slug);
  }
  return m;
});
const filteredTraitRows = computed(() => {
  const q = _traitSearchQ.value.toLowerCase();
  return traits.value.filter(
    (t) => !q || t.name.toLowerCase().includes(q) || t.id.toLowerCase().includes(q)
  );
});
function traitCostPill(cost: number[]): string {
  const costs = [...new Set(cost)].sort((a, b) => a - b);
  return costs.map((c) => `${c}费`).join(" ");
}

// ===== 强化符文页 =====
const augmentSearch = ref("");
const _augmentSearchQ = ref("");
let _augmentTimer = 0;
function setAugmentSearch(v: string) {
  augmentSearch.value = v;
  clearTimeout(_augmentTimer);
  _augmentTimer = setTimeout(() => (_augmentSearchQ.value = v), 200);
}
const augmentTier = ref<"all" | "silver" | "gold" | "prismatic">("all");
const filteredAugmentRows = computed(() => {
  const q = _augmentSearchQ.value.toLowerCase();
  return augments.value.filter((a) => {
    // 稀有度筛选（standard 归入「全部」）
    if (augmentTier.value !== "all" && (a.tier || "standard") !== augmentTier.value) {
      return false;
    }
    return !q || a.name.toLowerCase().includes(q) || a.id.toLowerCase().includes(q);
  });
});
/** tier（来自 CDragon tftitems 数据）→ 中文标签 */
function augmentTierLabel(tier: string | null | undefined): string {
  switch (tier) {
    case "silver":
      return "白银";
    case "gold":
      return "黄金";
    case "prismatic":
      return "棱彩";
    default:
      return "常规";
  }
}

// ===== 阵容列表（预设 + 收藏） =====
const savedComps = ref<string[]>([]);
function loadSavedComps() {
  try {
    const raw = localStorage.getItem("tft_saved_comps");
    savedComps.value = raw ? (JSON.parse(raw) as string[]) : [];
  } catch {
    savedComps.value = [];
  }
}
function toggleSavedComp(slug: string) {
  const i = savedComps.value.indexOf(slug);
  if (i >= 0) savedComps.value.splice(i, 1);
  else savedComps.value.push(slug);
  try {
    localStorage.setItem("tft_saved_comps", JSON.stringify(savedComps.value));
  } catch {}
}
const savedCompDetail = computed(() => {
  if (!savedComps.value.length) return [];
  return savedComps.value
    .map((slug) => ({ slug, card: cardOf(slug) }))
    .filter((x) => x.card.slug === x.slug && x.card.display_name_cn !== x.slug);
});

// ===== 当前阵容（Pin）：全局建议优先围绕它跑 =====
const pinnedComp = ref<string | null>(null);
function loadPinnedComp() {
  try {
    pinnedComp.value = localStorage.getItem("tft_pinned_comp") || null;
  } catch {
    pinnedComp.value = null;
  }
}
function setPinnedComp(slug: string | null) {
  pinnedComp.value = slug;
  try {
    if (slug) localStorage.setItem("tft_pinned_comp", slug);
    else localStorage.removeItem("tft_pinned_comp");
  } catch {}
}
/** Pin 的阵容卡片（用于顶部展示 + 建议标题） */
const pinnedCompCard = computed(() => {
  if (!pinnedComp.value) return null;
  const c = cardOf(pinnedComp.value);
  return c && c.display_name_cn !== pinnedComp.value ? c : null;
});
/** 有效 Pin（存在且数据里有） */
const effectivePin = computed(() => {
  const s = pinnedComp.value;
  if (!s || !tftData.value?.compDetails[s]) return null;
  return s;
});

// ===== 阵容码导入（TFT-Planner / 客户端 Team Planner Code） =====
// 格式：01 + 每个英雄 2 位 hex 序号（按 character_id 字母序）+ TFTSetN 后缀。
// hex 序号 = team_planner_code - 1000（来自 get_teamplanner_map）。
const teamplannerMap = ref<Record<string, { id: string; name: string }> | null>(null);
const importCode = ref("");
const importError = ref("");
const importedUnits = ref<{ id: string; name: string; icon: string; cost: number }[]>([]);

async function ensureTeamplannerMap(force = false) {
  if (teamplannerMap.value && !force) return;
  teamplannerMap.value = await invoke<Record<string, { id: string; name: string }>>("get_teamplanner_map");
}

/** 解析阵容码字符串 → 英雄 id 列表（Team Planner Code，01 头 + hex 序号 + TFTSet 后缀） */
function parseImportCode(code: string): string[] {
  const m = teamplannerMap.value;
  if (!m) return [];
  const body = code.trim().replace(/TFTSet\d+/i, "");
  if (!body || body.length < 2) return [];
  // 跳过开头 "01" 标记，之后每 2 位 hex 是一个英雄
  const out: string[] = [];
  for (let i = 2; i + 1 < body.length; i += 2) {
    const hex = body.slice(i, i + 2);
    const codeNum = parseInt(hex, 16);
    if (!codeNum) continue; // 00 = 空
    const e = m[codeNum.toString()];
    if (e) out.push(e.id);
  }
  return out;
}

async function importTeamCode() {
  importError.value = "";
  importedUnits.value = [];
  const code = importCode.value.trim();
  if (!code) return;
  try {
    await ensureTeamplannerMap();
  } catch (e) {
    importError.value = `teamplanner 数据加载失败: ${e}`;
    return;
  }
  const ids = parseImportCode(code);
  if (!ids.length) {
    importError.value = "无法解析该阵容码（格式可能不匹配当前赛季）";
    return;
  }
  importedUnits.value = ids.map((id) => ({
    id,
    name: unitName(id),
    icon: unitIcon(id),
    cost: unitCost(id),
  }));
}

// ===== 阵容构建器增强：羁绊激活 =====
const builderActiveTraits = computed(() => {
  if (builderSelected.value.length === 0) return [];
  const chMap = new Map<string, ChampionData>();
  for (const ch of champions.value) chMap.set(ch.id, ch);
  const traitCount: Record<string, number> = {};
  for (const id of builderSelected.value) {
    const ch = chMap.get(id);
    if (ch) for (const tr of ch.traits) traitCount[tr] = (traitCount[tr] ?? 0) + 1;
  }
  const out: { id: string; name: string; count: number; activeTier: number; totalTiers: number }[] = [];
  for (const tr of traits.value) {
    const cnt = traitCount[tr.id] ?? 0;
    if (cnt === 0) continue;
    let active = 0;
    for (const t of tr.tiers) {
      if (cnt >= t.min_units) active++;
    }
    out.push({
      id: tr.id,
      name: tr.name,
      count: cnt,
      activeTier: active,
      totalTiers: tr.tiers.length,
    });
  }
  return out.sort((a, b) => b.count - a.count);
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
  key: "matches" | "stats" | "comps" | "data" | "meta" | "builder" | "explorer" | "early" | "traits" | "augments" | "compList" | "settings";
  label: string;
  icon: string;
  go?: () => void;
  children?: { label: string; go: () => void }[];
}[] = [
  { key: "matches", label: "对局历史", icon: "◉" },
  { key: "stats", label: "统计", icon: "▤" },
  { key: "compList", label: "阵容列表", icon: "♜" },
  { key: "early", label: "前期", icon: "⚡" },
  { key: "builder", label: "阵容构建器", icon: "🔧" },
  { key: "explorer", label: "数据检索器", icon: "🔍" },
  { key: "traits", label: "羁绊", icon: "🔗" },
  { key: "augments", label: "强化符文", icon: "✨" },
  {
    key: "meta",
    label: "Meta数据",
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
      { label: "经济节奏", go: () => (dataTab.value = "econ") },
    ],
  },
  { key: "settings", label: "设置", icon: "⚙" },
];

function setView(
  v:
    | "matches"
    | "stats"
    | "comps"
    | "data"
    | "meta"
    | "builder"
    | "explorer"
    | "early"
    | "traits"
    | "augments"
    | "compList"
    | "settings",
) {
  view.value = v;
  if (v === "stats") openStats();
  if (v === "data") openData();
  if (v === "meta" || v === "comps" || v === "builder" || v === "explorer" || v === "early" || v === "traits" || v === "augments" || v === "compList") {
    getMeta();
  }
  if (v === "compList") {
    loadSavedComps();
    loadPinnedComp();
  }
}

function navGo(
  v:
    | "matches"
    | "stats"
    | "comps"
    | "data"
    | "meta"
    | "builder"
    | "explorer"
    | "early"
    | "traits"
    | "augments"
    | "compList"
    | "settings",
  child?: { go: () => void },
) {
  setView(v);
  if (child) child.go();
}

function navClick(g: (typeof navGroups)[number]) {
  setView(g.key);
  if (g.go) g.go();
}

// 手风琴式导航：点击分组标题折叠/展开
const expandedGroups = ref<Record<string, boolean>>({ meta: true });
function toggleNav(g: (typeof navGroups)[number]) {
  if (g.children?.length) {
    expandedGroups.value[g.label] = !expandedGroups.value[g.label];
  } else {
    navClick(g);
  }
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

// 对局详情棋盘（7列×4行）。
// 国服 LCU 对局历史不提供真实站位，因此：
// 1) 若未来数据源补齐 position_x/y，优先渲染真实站位；
// 2) 否则按本局 final units 反查最接近的 meta 阵容，渲染其 teamCode「参考预设站位」，
//    局内实际持有的核心棋子叠加星级，未持有的格子标「缺」。
interface MatchBoardCell {
  unit: { id: string; name: string; icon: string; cost: number };
  tier: number | null;
  held: boolean;
}
const matchBoard = computed<{
  hasBoard: boolean;
  isReference: boolean;
  compName: string;
  cells: (MatchBoardCell | null)[];
} | null>(() => {
  if (!selected.value || !selected.value.units.length) return null;
  const cells = new Array<MatchBoardCell | null>(28).fill(null);

  // 真实站位（未来数据源补齐时生效）
  let hasReal = false;
  for (const u of selected.value.units) {
    const x = u.position_x ?? -1;
    const y = u.position_y ?? -1;
    if (x >= 0 && x < 7 && y >= 0 && y < 4) {
      cells[y * 7 + x] = {
        unit: { id: u.character_id, name: u.name || u.character_id, icon: u.icon_url, cost: u.cost ?? 0 },
        tier: u.tier,
        held: true,
      };
      hasReal = true;
    }
  }
  if (hasReal) return { hasBoard: true, isReference: false, compName: "", cells };

  // 参考站位：匹配本局最接近的阵容（核心交集 ≥3）
  if (!tftData.value) return null;
  const byId = new Map(selected.value.units.map((u) => [u.character_id, u]));
  let best: { slug: string; matched: number; total: number } | null = null;
  for (const slug of Object.keys(tftData.value.compDetails)) {
    const ids = tftData.value.compDetails[slug].composition.overview.coreUnits.map((u) => u.id);
    const matched = ids.filter((id) => byId.has(id)).length;
    if (
      matched >= 3 &&
      (!best || matched > best.matched || (matched === best.matched && best.total > ids.length))
    ) {
      best = { slug, matched, total: ids.length };
    }
  }
  if (!best) return null;

  const ov = tftData.value.compDetails[best.slug].composition.overview;
  const { grid } = buildBoardGrid(parseTeamCode(ov.teamCode));
  for (let i = 0; i < 28; i++) {
    const ord = grid[i];
    if (ord && ord >= 1 && ord <= ov.coreUnits.length) {
      const tu = ov.coreUnits[ord - 1];
      const match = byId.get(tu.id);
      cells[i] = {
        unit: {
          id: tu.id,
          name: tu.nameCn || tu.name,
          icon: match?.icon_url || unitIcon(tu.id),
          cost: tu.cost,
        },
        tier: match?.tier ?? null,
        held: !!match,
      };
    }
  }
  return { hasBoard: true, isReference: true, compName: compName(best.slug), cells };
});

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
    // overlay 窗口：加载战绩（连胜连败）+ 实时状态 + 推荐
    loadMatches();
    getMeta();
    await refreshOverlay();
    await listen<any>("overlay-state", (e) => {
      const s = e.payload ?? {};
      overlayPhase.value = s.phase || overlayPhase.value;
      if (s.summoner) overlaySummoner.value = s.summoner;
      if (s.rank_tier) {
        overlayRank.value = s.rank_tier;
        overlayDivision.value = s.rank_division || "";
        overlayLp.value = s.rank_lp || 0;
      }
    });
    await listen<string>("overlay-phase", () => {
      refreshOverlay();
    });
    // 对局结束刷新战绩（连胜连败趋势更新）
    await listen<MatchSummary | null>("match-ended", () => {
      loadMatches();
    });
    await listen<any>("overlay-live", (e) => {
      applyLive(e.payload);
    });
    overlayTimer = setInterval(async () => {
      refreshOverlay();
      try {
        const s = await invoke<any>("get_live_state");
        if (s?.available) applyLive(s);
      } catch {
        /* 静默 */
      }
    }, 5000);
    return;
  }

  loadMatches();
  loadInfo();
  // 后台预加载 meta + CDN 数据，首次进入数据页无需等待
  getMeta();
  startAutoRefresh();
  // 初始化对局流程状态（供主窗口 dot 指示灯）
  refreshOverlay();

  // 对局实时状态（供主窗口显示，不进对局时后端会推 null）
  await listen<any>("overlay-live", (e) => {
    applyLive(e.payload);
  });
  // 对局流程状态（供主窗口 dot 指示灯）
  await listen<any>("overlay-state", (e) => {
    const s = e.payload ?? {};
    if (s.phase) overlayPhase.value = s.phase;
  });
  await listen<string>("overlay-phase", (e) => {
    overlayPhase.value = e.payload || "";
  });
  // 启动后静默检查是否有新补丁（不打扰用户，仅在有更新时提示）。
  // 节流：每天最多主动抓取一次 tftable.cc，避免每次启动都发整页请求。
  const UPDATE_CHECK_KEY = "tft_update_check_day";
  const lastCheckDay = localStorage.getItem(UPDATE_CHECK_KEY);
  const today = new Date().toISOString().slice(0, 10);
  if (lastCheckDay !== today) {
    localStorage.setItem(UPDATE_CHECK_KEY, today);
    setTimeout(() => {
      checkForUpdate().then(() => {
        if (updateCheck.value.has_update && !updating.value) {
          showToast(`检测到新版本 ${updateCheck.value.latest_patch}，可在「设置」中在线更新`);
        }
      });
    }, 3000);
  }

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

  // 托盘菜单「刷新数据」
  await listen<string>("refresh-data", () => {
    getMeta(true);
  });

  // 键盘快捷键：Ctrl/Cmd+数字 快速切换视图
  const onKeydown = (e: KeyboardEvent) => {
    if (!(e.ctrlKey || e.metaKey)) return;
    const map: Record<string, any> = {
      "0": "overlay",
      "1": "matches",
      "2": "stats",
      "3": "compList",
      "4": "early",
      "5": "builder",
      "6": "explorer",
      "7": "traits",
      "8": "augments",
    };
    const v = map[e.key];
    if (v === "overlay") {
      e.preventDefault();
      toggleOverlay();
    } else if (v) {
      e.preventDefault();
      setView(v);
    }
  };
  window.addEventListener("keydown", onKeydown);

  // 全局图片加载失败兜底：统一替换为占位图（事件捕获，覆盖动态渲染的 <img>）
  document.addEventListener(
    "error",
    (e) => {
      const el = e.target as HTMLImageElement;
      if (el.tagName !== "IMG" || el.dataset.imgFallback === "1") return;
      if (!el.src || el.src.includes("data:")) return;
      el.dataset.imgFallback = "1";
      const ic = document.createElement("canvas");
      ic.width = 64;
      ic.height = 64;
      const g = ic.getContext("2d");
      if (g) {
        g.fillStyle = "#1c1c28";
        g.fillRect(0, 0, 64, 64);
        g.strokeStyle = "#3a3a4a";
        g.lineWidth = 3;
        g.strokeRect(8, 8, 48, 48);
        g.fillStyle = "#5a5a6a";
        g.font = "28px sans-serif";
        g.textAlign = "center";
        g.textBaseline = "middle";
        g.fillText("?", 32, 34);
      }
      el.src = ic.toDataURL("image/png");
    },
    true
  );
});
</script>

<template>
  <!-- 游戏内辅助悬浮窗（overlay 窗口） -->
  <div v-if="isOverlay" class="overlay" data-tauri-drag-region>
    <div class="overlay-head">
      <span class="overlay-dot" :class="{ live: overlayPhase === 'InProgress' }"></span>
      <span class="overlay-title">TFT 对战助手</span>
      <span class="overlay-phase">{{ phaseLabel(overlayPhase) }}</span>
    </div>
    <div v-if="overlaySummoner" class="overlay-summoner">{{ overlaySummoner }}</div>
    <div v-if="overlayRank" class="overlay-rank">
      {{ overlayRank }} {{ overlayDivision }} · {{ overlayLp }} LP
    </div>

    <!-- 页签：实时 / 阵容 / 装备 -->
    <div class="overlay-tabs">
      <button
        v-for="t in ([
          { key: 'live', label: '实时' },
          { key: 'comps', label: '阵容' },
          { key: 'items', label: '装备' },
        ] as const)"
        :key="t.key"
        class="overlay-tab"
        :class="{ active: overlayTab === t.key }"
        @click="overlayTab = t.key"
      >{{ t.label }}</button>
    </div>

    <!-- ===== 实时页签 ===== -->
    <template v-if="overlayTab === 'live'">
      <!-- 开局选装助手（仅阶段 1 显示） -->
      <div v-if="live.available && overlayPhase === 'InProgress' && isEarlyStage" class="overlay-opening">
        <div class="comps-title">开局选装建议（价值分）</div>
        <div
          v-for="r in openingRecommendations"
          :key="r.id"
          class="opening-row"
        >
          <img class="mini-item" :src="r.icon" :alt="r.name" />
          <div class="opening-body">
            <div class="opening-head">
              <span class="opening-name">{{ r.name }}</span>
              <span class="opening-score" :style="{ width: r.value + '%' }"></span>
              <span class="opening-val">{{ r.value }}</span>
            </div>
            <div class="opening-comps">
              <span v-for="c in r.bestComps" :key="c.slug" class="trait-chip" @click="overlayTab='comps'; overlayOpenComp(c.slug)">{{ c.name }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 连胜连败趋势 -->
      <div v-if="overlayStreak" class="overlay-streak" :class="overlayStreak.type">
        {{ overlayStreak.text }}
      </div>

      <!-- 对局实时状态 -->
      <div v-if="live.available && overlayPhase === 'InProgress'" class="overlay-live">
        <div class="live-row">
          <span class="live-label">阶段</span><span class="live-val">{{ live.stage || "—" }}</span>
          <span class="live-label">时间</span><span class="live-val">{{ fmtGameTime(live.game_time) }}</span>
        </div>
        <div class="live-row">
          <span class="live-label">等级</span><span class="live-val">{{ live.level ?? "—" }}</span>
          <span class="live-label">金币</span><span class="live-val gold">{{ live.gold ?? "—" }}</span>
        </div>
        <div class="live-row">
          <span class="live-label">生命</span><span class="live-val">{{ Math.round(live.health ?? 0) }}</span>
        </div>
        <div class="live-players">
          <div
            v-for="p in opponentPanel"
            :key="p.name + '_' + p.rank + '_' + p.health"
            class="live-player"
            :class="{ me: p.is_me, threat: p.threat }"
            :title="`${p.name} · ${Math.round(p.health)}HP · ${p.level ?? '?'}级`"
          >
            <span class="lp-rank">{{ p.rank ?? "?" }}</span>
            <span class="lp-name">{{ p.name }}</span>
            <span class="lp-lv">{{ p.level ?? "—" }}</span>
            <span class="lp-hp" :style="oppHpWidth(p)"></span>
            <span v-if="p.threatText" class="lp-threat" :class="p.threat">{{ p.threatText }}</span>
          </div>
        </div>
        <div v-if="opponentSummary" class="live-threat-top">
          <span class="comps-title">威胁提示</span>
          <span class="live-threat-text">
            最高血量 <b>{{ opponentSummary.top.name }}</b>（{{ Math.round(opponentSummary.top.health) }}HP）
          </span>
        </div>
      </div>

      <!-- 抢牌 / 牌池检测提醒 -->
      <div v-if="contestAlerts.length && live.available && overlayPhase === 'InProgress'" class="overlay-contest">
        <div v-for="a in contestAlerts" :key="a.unitId" class="contest-row">
          <img class="comp-icon" :src="a.icon" alt="" />
          <span class="contest-text">{{ a.text }}</span>
        </div>
      </div>

      <!-- 实时建议 -->
      <div v-if="live.available && overlayPhase === 'InProgress'" class="overlay-advice">
        <div
          v-for="(t, i) in overlayAdvice"
          :key="i"
          class="advice-row"
          :class="t.level"
        >{{ t.text }}</div>
      </div>

      <!-- 当前阶段/等级参考卡 -->
      <div v-if="live.available && overlayPhase === 'InProgress'" class="overlay-stage-ref">
        <div v-if="currentEconRow" class="stage-ref-block">
          <div class="comps-title">
            阶段参考 · {{ live.stage }}（标准 {{ currentEconRow.targetLevel }} 级）
          </div>
          <div v-for="(it, i) in currentEconRow.items" :key="i" class="stage-ref-line">
            <span class="sr-lbl">{{ it.label }}</span><span class="sr-desc">{{ it.desc }}</span>
          </div>
        </div>
        <div v-if="currentLevelGuideRow" class="stage-ref-block">
          <div class="comps-title">等级参考 · {{ live.level }} 级</div>
          <div class="stage-ref-line"><span class="sr-lbl">该做的事</span><span class="sr-desc">{{ currentLevelGuideRow.focus }}</span></div>
          <div class="stage-ref-line">
            <span class="sr-lbl">商店概率</span>
            <span class="sr-desc">
              <span
                v-for="(o, i) in currentLevelGuideRow.odds"
                :key="i"
                class="sr-odds"
                :class="{ hot: i === 0 || o > 0 && o >= 25 }"
              >{{ i + 1 }}费{{ o }}%</span>
            </span>
          </div>
        </div>
      </div>

      <!-- 版本推荐阵容（简表，点击跳到阵容页签） -->
      <div v-if="overlayCompRecs.length && overlayPhase === 'InProgress'" class="overlay-comps">
        <div class="comps-title">当前版本推荐（点阵容看详情）</div>
        <div
          v-for="c in overlayCompRecs"
          :key="c.slug"
          class="comp-row"
          @click="overlayTab = 'comps'; overlayOpenComp(c.slug)"
        >
          <img class="comp-icon" :src="c.icon" alt="" />
          <span class="comp-name">{{ c.name }}</span>
          <span class="comp-avp">{{ c.avp.toFixed(2) }}</span>
        </div>
      </div>

      <!-- 商店 OCR 扫描 -->
      <div v-if="overlayPhase === 'InProgress'" class="overlay-ocr">
        <button class="ocr-btn" @click="scanShopOcr" :disabled="ocrScanning">
          {{ ocrScanning ? "扫描中…" : "📷 扫描商店" }}
        </button>
        <p v-if="ocrError" class="ocr-error">{{ ocrError }}</p>

        <!-- 识别到的棋子 + 逐格判定 -->
        <div v-if="shopSlots.length" class="ocr-slots">
          <div
            v-for="s in shopSlots"
            :key="s.index"
            class="ocr-slot"
            :class="s.verdict ? s.verdict.toLowerCase() : ''"
            :title="s.reason"
          >
            <template v-if="s.unit">
              <img class="top-icon" :src="s.unit.icon" :alt="s.unit.name" />
              <span class="ocr-unit-name">{{ s.unit.name }}</span>
              <span class="ocr-cost" :style="{ color: costColors[s.unit.cost - 1] }">{{ s.unit.cost }}费</span>
            </template>
            <template v-else>
              <span class="ocr-unit-name muted">—</span>
            </template>
            <span v-if="s.verdict" class="ocr-verdict">{{ s.verdict }}</span>
          </div>
        </div>
        <div v-else-if="ocrMatched.length" class="ocr-units">
          <div v-for="u in ocrMatched" :key="u.id" class="ocr-unit">
            <img class="top-icon" :src="u.icon" :alt="u.name" :title="u.name" />
            <span class="ocr-unit-name">{{ u.name }}</span>
            <span class="ocr-cost" :style="{ color: costColors[u.cost - 1] }">{{ u.cost }}费</span>
          </div>
        </div>

        <!-- 推荐 -->
        <div v-if="ocrRecommendations" class="ocr-recs">
          <div class="comps-title">命中阵容</div>
          <div
            v-for="c in ocrRecommendations.comps"
            :key="c.slug"
            class="comp-row clickable"
            @click="overlayTab = 'comps'; overlayOpenComp(c.slug)"
          >
            <img class="comp-icon" :src="c.icon" alt="" />
            <span class="comp-name">{{ c.name }}</span>
            <span class="comp-avp">{{ c.matched }}/{{ c.total }}</span>
          </div>
          <div v-if="ocrRecommendations.suggest.length" class="ocr-suggest">
            <span class="comps-title">建议拿：</span>
            <div class="ocr-suggest-units">
              <img
                v-for="s in ocrRecommendations.suggest"
                :key="s.id"
                class="top-icon"
                :src="s.icon"
                :alt="s.name"
                :title="s.name"
              />
            </div>
          </div>
        </div>
        <p v-else-if="ocrText && !ocrError" class="ocr-empty">未识别到已知棋子（可能需要调整窗口位置）</p>
      </div>

      <!-- 强化符文 OCR 扫描 -->
      <div v-if="overlayPhase === 'InProgress'" class="overlay-ocr">
        <button class="ocr-btn" @click="scanAugmentsOcr" :disabled="augScanning">
          {{ augScanning ? "扫描中…" : "✨ 扫描强化符文" }}
        </button>
        <p v-if="augError" class="ocr-error">{{ augError }}</p>

        <div v-if="augMatched.length" class="aug-recs">
          <div class="comps-title">强化符文选择建议</div>
          <div
            v-for="(a, i) in augMatched"
            :key="a.id"
            class="aug-row"
            :class="scoreAugments(augMatched)[i]?.level"
          >
            <img class="comp-icon" :src="a.icon" alt="" />
            <div class="aug-body">
              <div class="aug-name">{{ a.name }}</div>
              <div class="aug-reason">{{ scoreAugments(augMatched)[i]?.text }}</div>
            </div>
          </div>
        </div>
        <p v-else-if="augText && !augError" class="ocr-empty">未识别到强化符文（确认在符文选择界面再扫）</p>
      </div>
    </template>

    <!-- ===== 阵容页签 ===== -->
    <template v-if="overlayTab === 'comps'">
      <div v-if="!tftData" class="overlay-empty">加载数据中…</div>
      <template v-else>
        <!-- 阵容列表 -->
        <div v-if="!overlayCompDetail" class="overlay-comp-list">
          <div v-if="effectivePin && pinnedCompCard" class="ovl-pinned" @click="overlaySelectedComp = effectivePin">
            <span class="ovl-pinned-ico">📌</span>
            <span class="ovl-pinned-name">{{ pinnedCompCard.display_name_cn }}</span>
            <button class="ovl-unpin" @click.stop="setPinnedComp(null)">✕</button>
          </div>
          <div class="comps-title">当前版本强势阵容</div>
          <div
            v-for="(c, i) in overlayCompRecs"
            :key="c.slug"
            class="comp-row clickable"
            :class="{ 'is-pin': effectivePin === c.slug }"
            @click="overlayOpenComp(c.slug)"
          >
            <span class="comp-rank">#{{ i + 1 }}</span>
            <img class="comp-icon" :src="c.icon" alt="" />
            <span class="comp-name">{{ c.name }}</span>
            <span class="comp-avp">{{ c.avp.toFixed(2) }}</span>
            <button
              v-if="effectivePin === c.slug"
              class="ovl-pin-tag"
              title="取消 Pin"
              @click.stop="setPinnedComp(null)"
            >📌</button>
            <button
              v-else
              class="ovl-pin-tag subtle"
              title="设为当前阵容"
              @click.stop="setPinnedComp(c.slug)"
            >📌</button>
          </div>
        </div>

        <!-- 阵容详情 -->
        <div v-else-if="overlayCompDetail" class="overlay-comp-detail">
          <div class="detail-head">
            <button class="detail-back" @click="overlaySelectedComp = null">← 返回</button>
            <span class="detail-title">{{ overlayCompDetail.name }}<span class="detail-avp"> AVP {{ overlayCompDetail.stats.avgPlacementValue.toFixed(2) }}</span></span>
            <button
              class="ovl-pin-tag"
              :class="{ subtle: effectivePin !== overlaySelectedComp }"
              :title="effectivePin === overlaySelectedComp ? '取消 Pin' : '设为当前阵容'"
              @click="setPinnedComp(effectivePin === overlaySelectedComp ? null : overlaySelectedComp)"
            >
              {{ effectivePin === overlaySelectedComp ? "📌 已Pin" : "📌 Pin" }}
            </button>
          </div>

          <!-- 分阶段攻略 -->
          <div v-if="overlayStageGuide" class="ovl-stage-guide">
            <div class="comps-title">分阶段攻略 · {{ overlayStageGuide.playstyle }} / 核心 {{ overlayStageGuide.coreCosts }} 费</div>
            <div
              v-for="g in overlayStageGuide.stages"
              :key="g.key"
              class="ovl-sg-row"
              :class="{ current: live.available && currentEconRow && stageInRange(g.stage, live.stage) }"
            >
              <div class="ovl-sg-head">
                <span class="ovl-sg-stage">{{ g.stage }}</span>
                <span class="ovl-sg-lvl">{{ g.targetLevel }} 级</span>
                <span class="ovl-sg-take">
                  <img
                    v-for="u in g.take"
                    :key="u.id"
                    loading="lazy"
                    :src="unitIcon(u.id)"
                    :alt="u.name"
                    :title="u.name"
                    class="mini-item"
                    :class="{ core: u.core }"
                  />
                </span>
              </div>
              <div class="ovl-sg-line">{{ g.roll }}</div>
              <div class="ovl-sg-line muted">{{ g.item }}</div>
            </div>
          </div>

          <!-- 核心棋子 + 出装 -->
          <div class="detail-units">
            <div v-for="u in overlayCompDetail.units" :key="u.id" class="detail-unit">
              <div class="detail-unit-icon-wrap" :class="'cost' + u.cost">
                <img loading="lazy" :src="unitIcon(u.id)" :alt="u.nameCn" class="unit-icon" />
              </div>
              <div class="detail-unit-name">{{ u.nameCn }}</div>
              <div class="detail-builds">
                <div v-for="b in u.builds.slice(0, 2)" :key="b.index" class="detail-build">
                  <img loading="lazy" v-for="it in b.items" :key="it.id" :src="itemIcon(it.id)" :alt="it.id" class="mini-item" />
                </div>
                <div v-if="!u.builds.length" class="muted tiny">—</div>
              </div>
            </div>
          </div>

          <!-- 关键羁绊 -->
          <div class="detail-traits">
            <span v-for="t in overlayCompDetail.traits" :key="t.id" class="trait-chip">
              <img loading="lazy" v-if="t.icon" :src="tfImg(t.icon)" :alt="traitName(t.id)" class="trait-icon-sm" />
              {{ t.unitsRequired }} {{ traitName(t.id) }}
            </span>
          </div>

          <!-- 按当前等级阶段性阵容板 -->
          <template v-if="overlayLevelBoard">
            <div class="detail-pos-title">
              当前 {{ overlayLevelBoard.level }} 级 · 建议上阵
              <span v-if="live.available" class="muted tiny">（实时等级）</span>
            </div>
            <div class="lvl-board">
              <div
                v-for="u in overlayLevelBoard.units"
                :key="u.id"
                class="lvl-unit"
                :class="'cost' + u.cost"
              >
                <img loading="lazy" :src="unitIcon(u.id)" :alt="u.nameCn" class="unit-icon" />
                <div class="lvl-unit-name">{{ u.nameCn }}</div>
              </div>
            </div>
            <div v-if="overlayLevelBoard.activeTraits.length" class="lvl-traits">
              <span
                v-for="t in overlayLevelBoard.activeTraits"
                :key="t.id"
                class="trait-chip"
                :class="{ on: t.active }"
              >
                <img loading="lazy" v-if="t.icon" :src="tfImg(t.icon)" :alt="t.name" class="trait-icon-sm" />
                {{ t.count }}/{{ t.required }} {{ t.name }}
              </span>
            </div>
            <div v-if="overlayLevelBoard.nextTraits.length" class="lvl-next">
              还需补：<span v-for="t in overlayLevelBoard.nextTraits" :key="t.id" class="lvl-next-chip">{{ t.name }}</span>
            </div>
          </template>

          <!-- 升星检索：核心还差几张 + 当前等级刷新期望 -->
          <div v-if="upgradeCandidates.length" class="ovl-upgrade">
            <div class="detail-pos-title">升星检索（点选追踪持有数）</div>
            <div class="upgrade-picks">
              <div
                v-for="c in upgradeCandidates"
                :key="c.id"
                class="upgrade-pick"
                :class="{ active: upgradeUnit === c.id }"
                @click="upgradeUnit = c.id"
              >
                <img loading="lazy" :src="unitIcon(c.id)" :alt="c.name" class="mini-item" />
                <span class="upgrade-pick-name">{{ c.name }}</span>
                <span class="upgrade-pick-held">{{ c.held }}张</span>
              </div>
            </div>
            <div v-if="upgradeInfo" class="upgrade-info">
              <div class="upgrade-info-head">
                <img loading="lazy" :src="upgradeInfo.icon" :alt="upgradeInfo.name" class="mini-item" />
                <span class="upgrade-name">{{ upgradeInfo.name }}（{{ upgradeInfo.cost }}费）</span>
                <span class="upgrade-held">已持有 {{ upgradeInfo.held }}</span>
              </div>
              <div class="upgrade-row">
                <button class="ghost small" @click="bumpUpgrade(upgradeInfo.unitId, -1)">−</button>
                <span class="upgrade-count">{{ upgradeInfo.held }}</span>
                <button class="ghost small" @click="bumpUpgrade(upgradeInfo.unitId, 1)">＋</button>
              </div>
              <div class="upgrade-meta">
                <span>还差 <b>{{ upgradeInfo.to2 }}</b> 张到 2 星</span>
                <span>还差 <b>{{ upgradeInfo.to3 }}</b> 张到 3 星</span>
              </div>
              <div class="upgrade-odds">
                <span class="muted tiny">{{ upgradeInfo.lv }} 级每次刷新期望出 {{ upgradeInfo.perRoll.toFixed(2) }} 张</span>
                <span v-if="upgradeInfo.rollsTo3" class="upgrade-rolls">凑 3 星约需 {{ upgradeInfo.rollsTo3 }} 次刷新</span>
              </div>
            </div>
          </div>

          <!-- 推荐站位 -->
          <div class="detail-pos-title">最终站位</div>
          <div class="detail-board">
            <div v-for="idx in 28" :key="idx" class="board-cell">
              <div v-if="overlayCompDetail.coreUnitByPos[idx - 1]" class="cell-unit">
                <img
                  loading="lazy"
                  :src="unitIcon(overlayCompDetail.coreUnitByPos[idx - 1])"
                  :alt="unitName(overlayCompDetail.coreUnitByPos[idx - 1])"
                  :title="unitName(overlayCompDetail.coreUnitByPos[idx - 1])"
                  class="cell-icon"
                />
              </div>
            </div>
          </div>
          <div v-if="overlayCompDetail.bench.length" class="detail-bench">
            <span class="detail-bench-lbl">替补</span>
            <img
              v-for="b in overlayCompDetail.bench"
              :key="b"
              loading="lazy"
              :src="unitIcon(overlayCompDetail.units[b - 1]?.id)"
              :alt="unitName(overlayCompDetail.units[b - 1]?.id)"
              :title="unitName(overlayCompDetail.units[b - 1]?.id)"
              class="mini-unit bench-unit"
            />
          </div>

          <!-- 变体 -->
          <div v-if="overlayCompDetail.variants.length" class="detail-variants-title">阵容变体</div>
          <div v-for="v in overlayCompDetail.variants.slice(0, 3)" :key="v.variant_id" class="detail-variant">
            <span class="variant-lbl" v-if="v.is_primary">主力</span>
            <span class="variant-nums">{{ (v.clustered_stats.share * 100).toFixed(0) }}% · {{ v.clustered_stats.avg_placement.toFixed(1) }}</span>
            <div class="variant-icons">
              <img loading="lazy" v-for="u in v.units" :key="u.id" :src="unitIcon(u.id)" :alt="unitName(u.id)" class="mini-unit" :title="unitName(u.id)" />
            </div>
          </div>

          <!-- 出装优先级（简化：仅显示 top 弈子的核心装） -->
          <div v-if="overlayCompDetail.priority.length" class="detail-prio-title">核心弈子出装</div>
          <div v-for="r in overlayCompDetail.priority.slice(0, 4)" :key="r.unitId" class="detail-prio-row">
            <img class="top-icon" :src="unitIcon(r.unitId)" :title="unitName(r.unitId)" />
            <div class="prio-items-row">
              <div v-for="it in (r.buckets.Core || []).slice(0, 3)" :key="it.itemId" class="prio-item">
                <img loading="lazy" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId)" />
              </div>
              <div v-if="!(r.buckets.Core || []).length" class="muted tiny">—</div>
            </div>
          </div>
        </div>
      </template>
    </template>

    <!-- ===== 装备页签 ===== -->
    <template v-if="overlayTab === 'items'">
      <div v-if="!tftData" class="overlay-empty">加载数据中…</div>
      <template v-else>
        <!-- 散件→合成建议（针对当前阵容） -->
        <div v-if="slamTargetComp" class="slam-block">
          <div class="comps-title">点选手上散件 → 合成建议（目标：{{ compName(slamTargetComp) }}）</div>
          <div class="slam-components">
            <button
              v-for="c in components"
              :key="c.id"
              class="slam-comp"
              :class="{ held: heldComponents.includes(c.id) }"
              @click="toggleHeld(c.id)"
              :title="c.name"
            >
              <img class="mini-item" :src="c.icon_url" :alt="c.name" />
            </button>
          </div>
          <div v-if="slamRecs.length" class="slam-recs">
            <div v-for="(r, ri) in slamRecs" :key="r.id" class="slam-prio-row">
              <div class="craft-row" :class="{ ready: r.hasRecipe }">
                <span class="slam-rank">{{ ri + 1 }}</span>
                <img class="mini-item" :src="r.icon" :alt="r.name" />
                <span class="slam-name">{{ r.name }}</span>
                <span class="craft-eq">=</span>
                <img v-for="s in r.recipe" :key="s" class="mini-item" :src="itemIcon(s)" :alt="itemName(s)" />
                <span class="slam-flag">{{ r.hasRecipe ? "可合成" : "差1件" }}</span>
              </div>
              <div class="slam-bar"><span :style="{ width: slamPct(r.need) + '%' }"></span></div>
            </div>
          </div>
          <p v-else-if="heldComponents.length" class="ocr-empty">这些散件暂无可推荐成装（或该阵容不需要）</p>
        </div>

        <div class="comps-title">热门装备 → 适合弈子</div>
        <div v-for="r in overlayItemRecs" :key="r.itemId" class="item-recs">
          <div class="item-recs-head">
            <img class="mini-item" :src="itemIcon(r.itemId)" :alt="itemName(r.itemId)" />
            <span class="item-recs-name">{{ itemName(r.itemId) }}</span>
          </div>
          <div class="item-recs-units">
            <div v-for="u in r.units" :key="u.id" class="item-recs-unit">
              <img class="top-icon" :src="unitIcon(u.id)" :alt="unitName(u.id)" :title="unitName(u.id)" />
              <span class="item-recs-need">{{ (u.need * 100).toFixed(0) }}%</span>
            </div>
          </div>
        </div>

        <div v-if="overlayCraftables.length" class="comps-title" style="margin-top:10px">合成路线</div>
        <div v-for="c in overlayCraftables" :key="c.id" class="craft-row">
          <img class="mini-item" :src="itemIcon(c.id)" :alt="itemName(c.id)" />
          <span class="craft-eq">=</span>
          <img v-for="s in c.comps" :key="s" class="mini-item" :src="itemIcon(s)" :alt="itemName(s)" />
        </div>
      </template>
    </template>

    <div class="overlay-foot">对局结束自动记录战绩</div>
  </div>

  <div v-else class="window">
    <div class="titlebar" data-tauri-drag-region>
      <div class="titlebar-left">
        <span class="titlebar-logo">◈</span>
        <span class="titlebar-title">TFT Sidekick</span>
      </div>
      <div class="titlebar-center" data-tauri-drag-region="false">
        <span class="tb-chip" :class="summoner ? 'ok' : 'off'">
          <span class="tb-dot"></span>{{ summoner ? `${summoner.game_name}` : '未连接' }}
        </span>
        <span v-if="stats && stats.total" class="tb-chip" title="近 N 局战绩">
          {{ stats.total }}局 · 前四 {{ pct(stats.top4_rate) }} · 均名 {{ stats.avg_placement.toFixed(2) }}
        </span>
        <span v-else-if="live.available" class="tb-chip live">
          对局中 {{ live.stage || "" }} · {{ live.level ?? "—" }}级
        </span>
        <button class="tb-btn" title="同步战绩" @click="sync" :disabled="loading">
          {{ loading ? "同步中" : "同步" }}
        </button>
        <button class="tb-btn" title="游戏内悬浮窗 (Ctrl+0)" @click="toggleOverlay">悬浮窗</button>
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
          :key="g.label"
          class="nav-group"
          :class="{ 'has-sub': g.children?.length, expanded: g.children?.length && expandedGroups[g.label] }"
        >
          <a
            class="nav-item"
            :class="{ active: view === g.key }"
            @click="toggleNav(g)"
          >
            <span class="nav-ico">{{ g.icon }}</span>{{ g.label }}
            <span v-if="g.children?.length" class="nav-caret">{{ expandedGroups[g.label] ? '▾' : '▸' }}</span>
          </a>
          <div v-if="g.children?.length" class="nav-sub" :class="{ open: expandedGroups[g.label] }">
            <a
              v-for="c in g.children"
              :key="c.label"
              class="nav-sub-item"
              @click.stop="navGo(g.key as any, c)"
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
      </div>
    </aside>

    <main class="content">
      <div v-if="toast" class="toast">{{ toast }}</div>

      <!-- 全局错误提示（统一入口，关闭可清除） -->
      <div v-if="error" class="error-banner" @click="error = ''">
        <span class="error-banner-ico">⚠</span>
        <span class="error-banner-text">{{ error }}</span>
        <button class="error-banner-close" title="关闭">✕</button>
      </div>

      <!-- 对局实时状态条（主窗口） -->
      <div v-if="live.available" class="live-bar">
        <span class="live-bar-dot" :class="{ live: overlayPhase === 'InProgress' }"></span>
        <span v-if="live.stage" class="live-bar-item">阶段 {{ live.stage }}</span>
        <span class="live-bar-item">金币 {{ live.gold ?? "—" }}</span>
        <span class="live-bar-item">等级 {{ live.level ?? "—" }}</span>
        <span class="live-bar-item">生命 {{ Math.round(live.health ?? 0) }}</span>
        <span class="live-bar-item muted">{{ fmtGameTime(live.game_time) }}</span>
      </div>

      <!-- 统计视图 -->
      <template v-if="view === 'stats'">
        <header class="topbar">
          <h2>统计</h2>
        </header>

        <template v-if="stats">
          <div class="filter-group" style="margin-bottom: 12px">
            <button class="tab" :class="{ active: statsQueue === 'all' }" @click="openStats('all')">全部</button>
            <button
              v-for="q in [1090, 1130, 1100, 1110]"
              :key="q"
              class="tab"
              :class="{ active: statsQueue === q }"
              @click="openStats(q)"
            >{{ queueLabel(q) }}</button>
          </div>

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
                <img loading="lazy" v-if="u.icon_url" :src="u.icon_url" :alt="u.name" class="top-icon" />
                <span class="top-name">{{ u.name || u.id }}</span>
                <span class="top-count">{{ u.count }} 局</span>
              </div>
            </div>
          </section>

          <section class="board-section">
            <h3>常用羁绊</h3>
            <div class="top-list">
              <div v-for="t in stats.top_traits" :key="t.id" class="top-item">
                <img loading="lazy" v-if="t.icon_url" :src="t.icon_url" :alt="t.name" class="top-icon" />
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
                    <img loading="lazy" v-if="c.icon_url" :src="c.icon_url" :alt="c.name" class="champ-icon" />
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
              <img loading="lazy" v-if="w.icon_url" :src="w.icon_url" :alt="w.name" class="craft-icon" />
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
                <img loading="lazy" v-if="c.icon_url" :src="c.icon_url" :alt="c.name" />
                <span class="component-name">{{ c.name }}</span>
              </div>
            </div>
          </section>

          <section class="board-section">
            <h3>成品装备</h3>
            <div class="craft-list">
              <div v-for="item in filteredItems" :key="item.id" class="craft-item">
                <img loading="lazy" v-if="item.icon_url" :src="item.icon_url" :alt="item.name" class="craft-icon" />
                <div class="craft-body">
                  <div class="craft-name">{{ item.name }}</div>
                  <div class="craft-recipe">
                    <template v-for="cid in item.composition" :key="cid">
                      <img loading="lazy"                         v-if="itemMap[cid]?.icon_url"
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
                <img loading="lazy" v-if="c.icon_url" :src="c.icon_url" :alt="c.name" class="champ-icon" />
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
                <img loading="lazy" v-if="t.icon_url" :src="t.icon_url" :alt="t.name" class="champ-icon" />
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
                  <img loading="lazy" v-if="u.icon_url" :src="u.icon_url" :alt="u.name" />
                </span>
              </div>
            </div>
          </div>
        </template>

        <!-- 海克斯 -->
        <template v-else-if="dataTab === 'augments'">
          <div class="augment-list">
            <div v-for="a in filteredAugments" :key="a.id" class="augment-card">
              <img loading="lazy" v-if="a.icon_url" :src="a.icon_url" :alt="a.name" class="craft-icon" />
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

        <!-- 经济节奏（标准节奏参考，非实时） -->
        <template v-else-if="dataTab === 'econ'">
          <div class="odds-note">
            以下为当前补丁的<b>标准经济节奏参考</b>，具体策略因阵容/血量/装备而异，只作节奏参考。
            <span v-if="live.available" class="muted">对局中会自动高亮当前阶段/等级所在行。</span>
          </div>

          <!-- 按等级参考 -->
          <section class="board-section">
            <h3>按等级参考（1~9 级）</h3>
            <table class="odds-table">
              <thead>
                <tr>
                  <th>等级</th>
                  <th>标准阶段</th>
                  <th>场上人数</th>
                  <th>商店概率 (1费/2费/3费/4费/5费 %)</th>
                  <th>该等级该做什么</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="g in levelGuide"
                  :key="g.level"
                  class="ref-row"
                  :class="{ current: currentLevelGuideRow?.level === g.level }"
                >
                  <td class="rule-key">{{ g.level }}</td>
                  <td>{{ g.stage }}</td>
                  <td>{{ g.board }} 人</td>
                  <td class="muted tiny">{{ g.odds.join(" / ") }}</td>
                  <td>{{ g.focus }}</td>
                </tr>
              </tbody>
            </table>
          </section>

          <!-- 按阶段时间轴 -->
          <section class="board-section">
            <h3>按阶段时间轴</h3>
            <div v-for="row in econTimeline" :key="row.stage" class="rule-section" :class="{ current: currentEconRow?.stage === row.stage }">
              <h3>
                {{ row.stage }}
                <span class="lvl-badge">标准等级 {{ row.targetLevel }}</span>
                <span class="muted econ-title-sub">{{ row.phase_label }}</span>
              </h3>
              <table class="odds-table">
                <tbody>
                  <tr v-for="(item, ri) in row.items" :key="ri">
                    <td class="rule-key econ-key">{{ item.label }}</td>
                    <td>{{ item.desc }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>
        </template>
      </template>

      <!-- 阵容一览 + 阵容详情 -->
      <!-- 前期视图 (Early)：通用低费过渡 -->
      <template v-else-if="view === 'early'">
        <header class="topbar">
          <h2>前期过渡</h2>
          <span class="muted">开局通用弈子 · 不绑定具体成型阵容</span>
        </header>
        <div class="tabs tabs-group">
          <button class="tab" :class="{ active: earlyCost === 'all' }" @click="earlyCost = 'all'">全部</button>
          <button v-for="c in [1, 2, 3]" :key="c" class="tab" :class="{ active: earlyCost === c }" @click="earlyCost = c">{{ c }} 费</button>
        </div>

        <template v-if="tftData">
          <!-- 开局选装助手：散件价值分排序 -->
          <section class="board-section">
            <h3>开局选装助手（散件价值分）</h3>
            <p class="muted tiny" style="margin-bottom:8px">选装环 3 选 1 时看这里：价值分越高，该散件能进的强势核心阵容越多。</p>
            <div class="scatter-grid">
              <div v-for="s in scatterScores.slice(0, 8)" :key="s.id" class="scatter-card">
                <div class="scatter-head">
                  <img loading="lazy" :src="s.icon" :alt="s.name" class="mini-item" />
                  <span class="scatter-name">{{ s.name }}</span>
                  <span class="scatter-val">{{ s.value }}</span>
                </div>
                <div class="scatter-bar"><span :style="{ width: s.value + '%' }"></span></div>
                <div class="scatter-meta">
                  <div class="scatter-row">
                    <span class="muted tiny">适配阵容</span>
                    <span v-for="c in s.bestComps" :key="c.slug" class="trait-chip" @click="showCompDetail(c.slug)">{{ c.name }}</span>
                  </div>
                  <div class="scatter-row">
                    <span class="muted tiny">常见 carry</span>
                    <img v-for="cid in s.bestCarriers" :key="cid.id" loading="lazy" :src="unitIcon(cid.id)" :alt="cid.name" :title="cid.name" class="mini-unit" />
                  </div>
                </div>
              </div>
            </div>
          </section>

          <!-- 低费过渡英雄池 -->
          <section class="board-section">
            <h3>过渡弈子池（{{ earlyUnits.length }}）</h3>
            <div class="early-unit-grid">
              <div v-for="u in earlyUnits" :key="u.unitId" class="early-unit-card" @click="earlyCompsFor(u).length && showCompDetail(earlyCompsFor(u)[0])">
                <div class="early-unit-head">
                  <img loading="lazy" :src="unitIcon(u.unitId)" :alt="unitName(u.unitId)" class="early-unit-icon" :class="'cost' + u.unitCost" />
                  <div class="early-unit-info">
                    <span class="early-unit-name">{{ unitName(u.unitId) }}</span>
                    <span class="early-unit-en muted">{{ unitNameEn(u.unitId) }}</span>
                  </div>
                  <span class="early-cost-badge" :style="{ color: costColors[u.unitCost - 1] }">{{ u.unitCost }}费</span>
                </div>
                <div v-if="u.buckets.Core?.length || u.buckets.Important?.length" class="early-items">
                  <span class="muted tiny">核心:</span>
                  <img loading="lazy" v-for="it in (u.buckets.Core || []).concat(u.buckets.Important || []).slice(0, 4)" :key="it.itemId" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId) + (it.necessity ? ' · 必需度 ' + it.necessity.toFixed(2) : '')" />
                </div>
                <div v-if="earlyCompsFor(u).length" class="early-comps">
                  <span class="muted tiny">→</span>
                  <span v-for="slug in earlyCompsFor(u).slice(0, 3)" :key="slug" class="trait-chip" @click.stop="showCompDetail(slug)">{{ compName(slug) }}</span>
                </div>
              </div>
            </div>
          </section>

          <!-- 前期目标阵容（低费成型方向） -->
          <section class="board-section">
            <h3>前期可成型的低费阵容</h3>
            <div class="comp-grid">
              <div v-for="c in earlyGoalComps" :key="c.slug" class="comp-card" @click="showCompDetail(c.slug)">
                <div class="comp-card-head">
                  <img loading="lazy" :src="tfImg(c.compIcon)" :alt="c.display_name_cn" class="comp-card-icon" />
                  <div class="comp-card-title">
                    <span class="comp-card-name">{{ c.display_name_cn }}</span>
                    <span class="comp-card-en muted">{{ c.display_name_en }}</span>
                  </div>
                  <span class="comp-cost-tag">${{ c.compCost }}</span>
                </div>
                <div class="comp-card-stats">
                  <div class="comp-card-stat">
                    <span class="comp-card-val">{{ c.averagePlacementValue.toFixed(2) }}</span>
                    <span class="comp-card-lbl">AVP</span>
                  </div>
                  <div class="comp-card-stat">
                    <span class="comp-card-val">{{ (statOf(c.slug).playRate * 100).toFixed(2) }}%</span>
                    <span class="comp-card-lbl">出场率</span>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </template>
        <p v-else-if="!metaLoading" class="empty">加载中，请稍候…</p>
      </template>

      <!-- 阵容构建器视图 (Team Builder) -->
      <template v-else-if="view === 'builder'">
        <header class="topbar">
          <h2>阵容构建器</h2>
          <span class="muted">点击选择弈子，查看关联阵容</span>
        </header>
        <div class="builder-wrap">
          <div class="builder-sidebar">
            <div class="builder-import">
              <div class="comps-title">导入阵容码</div>
              <textarea v-model="importCode" class="import-code-input" rows="2" placeholder="粘贴 TFT-Planner / 客户端阵容码（如 0101...TFTSet18）" />
              <button class="ghost small" @click="importTeamCode" style="width:100%;margin-top:4px">解码并加入</button>
              <p v-if="importError" class="ocr-error" style="font-size:11px">{{ importError }}</p>
            </div>
            <input v-model="builderSearch" class="search-input" placeholder="搜索英雄…" />
            <div class="builder-unit-list">
              <div v-for="id in filteredBuilderUnits" :key="id" class="builder-unit" :class="{ selected: builderSelected.includes(id) }" @click="toggleBuilderUnit(id)">
                <img loading="lazy" :src="unitIcon(id)" :alt="unitName(id)" class="builder-unit-icon" :class="'cost' + unitCost(id)" />
                <span class="builder-unit-name">{{ unitName(id) }}</span>
              </div>
            </div>
          </div>
          <div class="builder-main">
            <div class="builder-slots">
              <div v-for="(id, i) in builderSelected" :key="i" class="builder-slot" @click="toggleBuilderUnit(id)">
                <img loading="lazy" :src="unitIcon(id)" :alt="unitName(id)" class="builder-slot-icon" />
                <span class="builder-slot-name">{{ unitName(id) }}</span>
                <span class="builder-slot-x">✕</span>
              </div>
              <div v-for="i in Math.max(0, 9 - builderSelected.length)" :key="'e'+i" class="builder-slot empty">+</div>
            </div>
            <div v-if="importedUnits.length" class="builder-imported">
              <div class="comps-title">导入的棋子（点击加入构建）</div>
              <div class="builder-imported-units">
                <div
                  v-for="u in importedUnits"
                  :key="u.id"
                  class="builder-unit imported"
                  :class="{ selected: builderSelected.includes(u.id) }"
                  @click="toggleBuilderUnit(u.id)"
                >
                  <img loading="lazy" :src="u.icon" :alt="u.name" class="builder-unit-icon" :class="'cost' + u.cost" />
                  <span class="builder-unit-name">{{ u.name }}</span>
                </div>
              </div>
            </div>
            <div class="builder-cost-bar">
              <span v-for="c in [1,2,3,4,5]" :key="c" class="builder-cost" :style="{ color: costColors[c-1] }">
                {{ c }}费: <b>{{ builderCostCount[c] ?? 0 }}</b>
              </span>
            </div>
            <section v-if="builderActiveTraits.length" class="board-section">
              <h3>激活羁绊</h3>
              <div class="builder-traits">
                <div v-for="t in builderActiveTraits" :key="t.id" class="builder-trait">
                  <span class="builder-trait-name">{{ t.name }}</span>
                  <span class="builder-trait-count" :class="{ active: t.activeTier > 0 }">
                    {{ t.count }} 位英雄 · {{ t.activeTier }}/{{ t.totalTiers }} 层
                  </span>
                </div>
              </div>
            </section>
            <section v-if="builderMatchingComps.length" class="board-section">
              <h3>匹配阵容</h3>
              <div class="meta-list">
                <div v-for="c in builderMatchingComps" :key="c.slug" class="meta-row comp-row" @click="showCompDetail(c.slug)">
                  <span class="meta-name">
                    <img loading="lazy" :src="compCardIcon(c.slug)" class="top-icon" />
                    {{ compName(c.slug) }}
                  </span>
                  <span class="meta-num accent green">{{ c.matched }}/{{ c.total }} 弈子匹配</span>
                </div>
              </div>
            </section>
            <p v-if="builderSelected.length === 0" class="empty">请在左侧选择弈子开始构建</p>
          </div>
        </div>
        <p v-if="tftData === null && !metaLoading" class="empty">加载中，请稍候…</p>
      </template>

      <!-- 数据检索器视图 (Data Explorer) -->
      <template v-else-if="view === 'explorer'">
        <header class="topbar">
          <h2>数据检索器</h2>
          <span class="muted">交叉检索英雄/装备/阵容</span>
        </header>
        <div class="tabs tabs-group">
          <button class="tab" :class="{ active: explorerTab === 'units' }" @click="explorerTab = 'units'">英雄</button>
          <button class="tab" :class="{ active: explorerTab === 'items' }" @click="explorerTab = 'items'">装备</button>
          <button class="tab" :class="{ active: explorerTab === 'comps' }" @click="explorerTab = 'comps'">阵容</button>
        </div>

        <template v-if="tftData">
          <div v-if="explorerTab === 'units'">
            <div class="explorer-filter">
              <input v-model="explorerUnitSearch" class="search-input" placeholder="搜索英雄…" />
              <div class="tabs tabs-group" style="margin-left:8px">
                <button class="tab" :class="{ active: explorerUnitCost === 'all' }" @click="explorerUnitCost = 'all'">全部</button>
                <button v-for="c in [1,2,3,4,5]" :key="c" class="tab" :class="{ active: explorerUnitCost === c }" @click="explorerUnitCost = c">{{ c }}费</button>
              </div>
            </div>
            <div v-if="explorerUnitRows.length" class="explorer-list">
              <div v-for="u in explorerUnitRows" :key="u.unitId" class="explorer-row">
                <div class="explorer-head">
                  <img loading="lazy" :src="unitIcon(u.unitId)" class="top-icon" />
                  <span class="explorer-name">{{ unitName(u.unitId) }}</span>
                  <span class="muted tiny">{{ u.unitCost }}费</span>
                  <span v-if="u.unitPriority != null" class="muted tiny" style="margin-left:auto">优先度 {{ u.unitPriority.toFixed(2) }}</span>
                </div>
                <div v-if="u.buckets.Core.length" class="explorer-items">
                  <span class="muted tiny">核心:</span>
                  <img loading="lazy" v-for="it in u.buckets.Core" :key="it.itemId" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId)" />
                </div>
                <div v-if="u.buckets.Important.length" class="explorer-items">
                  <span class="muted tiny">重要:</span>
                  <img loading="lazy" v-for="it in u.buckets.Important" :key="it.itemId" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId)" />
                </div>
                <div v-if="u.coreCompStrengths.length" class="unit-comps">
                  <span v-for="cc in u.coreCompStrengths" :key="cc.comp_key" class="trait-chip" @click="showCompDetail(cc.comp_key)">{{ compName(cc.comp_key) }}</span>
                </div>
              </div>
            </div>
          </div>

          <div v-if="explorerTab === 'items'">
            <div v-if="explorerItemRows.length" class="explorer-list">
              <div v-for="g in explorerItemRows" :key="g.itemId" class="explorer-row">
                <div class="explorer-head">
                  <img loading="lazy" :src="itemIcon(g.itemId)" class="top-icon" />
                  <span class="explorer-name">{{ g.itemName }}</span>
                </div>
                <div v-if="g.costBuckets" class="explorer-items">
                  <div v-for="(units, cost) in sortedBuckets(g.costBuckets)" :key="cost" style="display:flex;align-items:center;gap:4px;margin:2px 0">
                    <span class="muted tiny" style="width:24px">{{ cost }}费</span>
                    <img loading="lazy" v-for="u in units.slice(0, 6)" :key="u.unitId" :src="unitIcon(u.unitId)" class="mini-unit" :title="unitName(u.unitId)" />
                  </div>
                </div>
                <div v-if="g.compositions.length" class="unit-comps">
                  <span v-for="cc in g.compositions.slice(0, 4)" :key="cc.slug + cc.unit_id" class="trait-chip" @click="showCompDetail(cc.slug)">{{ compName(cc.slug) }}</span>
                </div>
              </div>
            </div>
          </div>

          <div v-if="explorerTab === 'comps'">
            <div class="explorer-filter">
              <input v-model="explorerCompSearch" class="search-input" placeholder="搜索阵容…" />
            </div>
            <div v-if="explorerCompRows.length" class="explorer-list">
              <div v-for="c in explorerCompRows" :key="c.slug" class="explorer-row comp-row" @click="showCompDetail(c.slug)">
                <div class="explorer-head">
                  <img loading="lazy" :src="compCardIcon(c.slug)" class="top-icon" />
                  <span class="explorer-name">{{ compName(c.slug) }}</span>
                </div>
                <div class="explorer-meta">
                  <span class="tiny muted">AVP {{ c.avgPlacementValue.toFixed(2) }}</span>
                  <span class="tiny muted">强度 {{ (statOf(c.slug).strength * 100).toFixed(1) }}%</span>
                  <span class="tiny muted">出场 {{ (statOf(c.slug).playRate * 100).toFixed(2) }}%</span>
                </div>
              </div>
            </div>
            <p v-if="explorerCompRows.length === 0" class="empty">没有匹配的阵容</p>
          </div>
        </template>
        <p v-else-if="!metaLoading" class="empty">加载中，请稍候…</p>
      </template>

      <!-- 羁绊统计视图 -->
      <template v-else-if="view === 'traits'">
        <header class="topbar">
          <h2>羁绊统计</h2>
          <input @input="(e: any) => setTraitSearch((e.target as HTMLInputElement).value)" class="search-input" placeholder="搜索羁绊…" />
        </header>
        <template v-if="tftData">
          <div class="trait-list">
            <div v-for="t in filteredTraitRows" :key="t.id" class="trait-card">
              <div class="champ-head">
                <img loading="lazy" v-if="t.icon_url" :src="t.icon_url" :alt="t.name" class="champ-icon" />
                <div>
                  <div class="champ-name">{{ t.name }}</div>
                  <div class="champ-traits">{{ t.id }}</div>
                </div>
                <span v-if="traitWinShareMap[t.id]" class="trait-stat">
                  登顶 {{ ((traitWinShareMap[t.id].win / tftData.metaStats.winShare.sample_size) * 100).toFixed(2) }}% ·
                  前四 {{ ((traitWinShareMap[t.id].top4 / tftData.metaStats.top4Share.sample_size) * 100).toFixed(2) }}%
                </span>
              </div>
              <div class="trait-tiers">
                <span v-for="(tier, i) in t.tiers" :key="i" class="tier-pill">
                  {{ tier.min_units }}{{ tier.max_units > tier.min_units ? "-" + tier.max_units : "" }}
                </span>
                <span v-if="traitCountByCost[t.id]" class="tier-pill">{{ traitCountByCost[t.id].count }} 英雄</span>
                <span v-if="traitCountByCost[t.id]" class="tier-pill">{{ traitCostPill(traitCountByCost[t.id].cost) }}</span>
              </div>
              <div class="ability-desc">{{ t.desc }}</div>
              <div v-if="t.units.length > 0" class="trait-units">
                <img loading="lazy" v-for="u in t.units.slice(0, 10)" :key="u.id" :src="u.icon_url" :alt="u.name" class="mini-unit" :title="u.name" />
              </div>
              <div v-if="traitCompsMap[t.id]?.length" class="unit-comps">
                <span class="muted tiny">相关阵容:</span>
                <span v-for="slug in traitCompsMap[t.id].slice(0, 8)" :key="slug" class="trait-chip" @click="showCompDetail(slug)">{{ compName(slug) }}</span>
              </div>
            </div>
          </div>
        </template>
        <p v-else-if="!metaLoading" class="empty">加载中…</p>
      </template>

      <!-- 强化符文视图 -->
      <template v-else-if="view === 'augments'">
        <header class="topbar">
          <h2>强化符文</h2>
          <input @input="(e: any) => setAugmentSearch((e.target as HTMLInputElement).value)" class="search-input" placeholder="搜索海克斯…" />
        </header>
        <div class="tabs tabs-group">
          <button class="tab" :class="{ active: augmentTier === 'all' }" @click="augmentTier = 'all'">全部</button>
          <button
            v-for="t in [
              { k: 'standard', label: '常规' },
              { k: 'silver', label: '白银' },
              { k: 'gold', label: '黄金' },
              { k: 'prismatic', label: '棱彩' },
            ]"
            :key="t.k"
            class="tab"
            :class="{ active: augmentTier === t.k }"
            @click="augmentTier = t.k as any"
          >{{ t.label }}</button>
        </div>
        <div class="augment-list">
          <div v-for="a in filteredAugmentRows" :key="a.id" class="augment-card">
            <img loading="lazy" v-if="a.icon_url" :src="a.icon_url" :alt="a.name" class="craft-icon" />
            <div class="craft-body">
              <div class="craft-name">
                {{ a.name }}
                <span v-if="a.tier && a.tier !== 'standard'" class="augment-tier" :class="'tier-' + a.tier">{{ augmentTierLabel(a.tier) }}</span>
              </div>
              <div class="ability-desc">{{ a.desc || "—" }}</div>
            </div>
          </div>
        </div>
        <p v-if="filteredAugmentRows.length === 0" class="empty">没有匹配的海克斯</p>
      </template>

      <!-- 阵容列表视图 -->
      <template v-else-if="view === 'compList'">
        <header class="topbar">
          <h2>阵容列表</h2>
          <span class="muted">点击收藏 / 取消收藏阵容</span>
        </header>
        <div v-if="effectivePin && pinnedCompCard" class="pinned-banner" @click="showCompDetail(effectivePin)">
          <span class="pinned-banner-ico">📌</span>
          <span>当前阵容：<b>{{ pinnedCompCard.display_name_cn }}</b></span>
          <span class="muted tiny">商店 / 符文 / 装备建议都围绕它</span>
          <button class="ghost small" @click.stop="setPinnedComp(null)">取消</button>
        </div>
        <template v-if="selectedCompDetail && selectedCard">
          <div class="comp-detail">
            <header class="topbar">
              <button class="ghost" @click="closeCompDetail">← 返回阵容列表</button>
              <h2 class="detail-title">
                <img loading="lazy" :src="tfImg(selectedCard.compIcon)" :alt="selectedCard.display_name_cn" class="top-icon" />
                {{ selectedCard.display_name_cn }}
                <span class="muted en-name">{{ selectedCard.display_name_en }}</span>
                <span v-if="isVariantComp(selectedCompSlug as string)" class="variant-badge clickable" :title="`查看主阵容 ${compName(variantBaseOf(selectedCompSlug as string))}`" @click="showCompDetail(variantBaseOf(selectedCompSlug as string))">
                  {{ compName(variantBaseOf(selectedCompSlug as string)) }} 的变体 ↗
                </span>
              </h2>
              <button
                class="ghost small pin-btn"
                :class="{ pinned: effectivePin === selectedCompSlug }"
                @click="setPinnedComp(effectivePin === selectedCompSlug ? null : selectedCompSlug)"
              >
                {{ effectivePin === selectedCompSlug ? "📌 已设为当前阵容" : "📌 设为当前阵容" }}
              </button>
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

            <!-- 分阶段攻略：每阶段拿什么 / 升几级 / 搜牌 / 装备 -->
            <section class="board-section">
              <h3>分阶段攻略 <span class="muted tiny">运营类型：{{ selectedCompDetail ? compStageGuide(selectedCompSlug as string)?.playstyle : "" }} · 核心费用 {{ selectedCompDetail ? compStageGuide(selectedCompSlug as string)?.coreCosts : "" }}</span></h3>
              <div class="stage-guide">
                <div
                  v-for="g in (selectedCompDetail ? compStageGuide(selectedCompSlug as string)?.stages ?? [] : [])"
                  :key="g.key"
                  class="stage-guide-row"
                  :class="{ current: live.available && currentEconRow && stageInRange(g.stage, live.stage) }"
                >
                  <div class="sg-head">
                    <span class="sg-stage">{{ g.stage }}</span>
                    <span class="sg-lvl">目标 {{ g.targetLevel }} 级</span>
                  </div>
                  <div class="sg-take">
                    <img
                      v-for="u in g.take"
                      :key="u.id"
                      loading="lazy"
                      :src="unitIcon(u.id)"
                      :alt="u.name"
                      :title="u.name"
                      class="mini-unit"
                      :class="{ core: u.core }"
                    />
                    <span v-if="!g.take.length" class="muted tiny">—</span>
                  </div>
                  <div class="sg-line"><span class="sg-lbl">升级</span>{{ g.level }}</div>
                  <div class="sg-line"><span class="sg-lbl">搜牌</span>{{ g.roll }}</div>
                  <div class="sg-line"><span class="sg-lbl">装备</span>{{ g.item }}</div>
                </div>
              </div>
            </section>

            <!-- 核心阵容 -->
            <section class="board-section">
              <h3>核心弈子与出装</h3>
              <div class="detail-board">
                <div v-for="u in selectedCompDetail.composition.overview.coreUnits" :key="u.id" class="detail-unit">
                  <div class="detail-unit-icon-wrap" :class="'cost' + u.cost">
                    <img loading="lazy" :src="unitIcon(u.id)" :alt="u.nameCn" class="unit-icon" />
                  </div>
                  <div class="detail-unit-name">{{ u.nameCn }}</div>
                  <div class="detail-unit-en muted">{{ u.nameEn }}</div>
                  <div class="detail-builds">
                    <div v-for="b in u.builds" :key="b.index" class="detail-build" :title="'出装 ' + b.index">
                      <img loading="lazy" v-for="it in b.items" :key="it.id" :src="itemIcon(it.id)" :alt="it.id" class="mini-item" />
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
                  <img loading="lazy" v-if="t.icon" :src="tfImg(t.icon)" :alt="traitName(t.id)" class="trait-icon" />
                  <span class="trait-name">{{ traitName(t.id) }}</span>
                  <span class="trait-tier">{{ t.unitsRequired }} 单位</span>
                </div>
              </div>
            </section>

            <!-- 推荐站位 -->
            <section class="board-section">
              <h3>推荐站位</h3>
              <div class="match-board comp-pos-board">
                <div v-for="(cell, idx) in compPosCells" :key="idx" class="board-cell">
                  <div v-if="cell" class="cell-unit">
                    <img
                      loading="lazy"
                      :src="unitIcon(cell)"
                      :alt="unitName(cell)"
                      :title="unitName(cell)"
                      class="cell-icon"
                      :style="{ borderColor: costColor(unitCost(cell)) }"
                    />
                  </div>
                </div>
              </div>
              <div v-if="compPosBench.length" class="comp-pos-bench">
                <span class="muted tiny" style="margin-right:6px">替补</span>
                <img
                  v-for="b in compPosBench"
                  :key="b"
                  loading="lazy"
                  :src="unitIcon(selectedCompDetail.composition.overview.coreUnits[b - 1]?.id)"
                  :title="unitName(selectedCompDetail.composition.overview.coreUnits[b - 1]?.id)"
                  class="mini-unit"
                />
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
                    <img loading="lazy" v-for="u in v.units" :key="u.id" :src="unitIcon(u.id)" :alt="unitName(u.id)" class="mini-unit" :title="unitName(u.id)" />
                  </div>
                  <div v-if="v.traits.length" class="variant-traits">
                    <span v-for="t in v.traits" :key="t.id" class="trait-chip">
                      <img loading="lazy" v-if="t.icon" :src="tfImg(t.icon)" :alt="traitName(t.id)" class="trait-icon-sm" />
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
                  <img loading="lazy" :src="unitIcon(r.unitId)" class="top-icon" :alt="unitName(r.unitId)" />
                  <span class="prio-unit">{{ unitName(r.unitId) }}</span>
                  <span class="muted tiny" style="margin-left:4px; font-size:10px">{{ unitNameEn(r.unitId) }}</span>
                  <span v-if="r.unitPriority != null" class="muted tiny" style="margin-left:4px">优先度 {{ r.unitPriority.toFixed(2) }}</span>
                </div>
                <div class="prio-buckets">
                  <div v-if="r.buckets.Core.length" class="prio-bucket">
                    <span class="prio-bucket-lbl">核心</span>
                    <div class="prio-items">
                      <div v-for="it in r.buckets.Core" :key="it.itemId" class="prio-item">
                        <img loading="lazy" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId) + ' (' + it.necessity.toFixed(2) + ')'" />
                        <span class="prio-item-nec">{{ it.necessity.toFixed(2) }}</span>
                      </div>
                    </div>
                  </div>
                  <div v-if="r.buckets.Important.length" class="prio-bucket">
                    <span class="prio-bucket-lbl">重要</span>
                    <div class="prio-items">
                      <div v-for="it in r.buckets.Important" :key="it.itemId" class="prio-item">
                        <img loading="lazy" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId) + ' (' + it.necessity.toFixed(2) + ')'" />
                        <span class="prio-item-nec">{{ it.necessity.toFixed(2) }}</span>
                      </div>
                    </div>
                  </div>
                  <div v-if="r.buckets.Optional.length" class="prio-bucket">
                    <span class="prio-bucket-lbl">可选</span>
                    <div class="prio-items">
                      <div v-for="it in r.buckets.Optional" :key="it.itemId" class="prio-item">
                        <img loading="lazy" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId) + ' (' + it.necessity.toFixed(2) + ')'" />
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
                      <img loading="lazy" :src="unitIcon(g.unitId)" :alt="unitName(g.unitId)" class="mini-unit" />
                      <span class="artifact-carrier">{{ unitName(g.unitId) }}</span>
                      <img loading="lazy" v-for="it in g.items" :key="it.id" :src="itemIcon(it.id)" class="mini-item" :title="itemName(it.id) + ' 均 ' + it.avgPlacement.toFixed(2)" />
                    </div>
                  </div>
                </div>
              </div>
              <div v-if="selectedCompDetail.composition.specialGroups.radiants.length" class="special-block">
                <h4 class="special-lbl">光明装备</h4>
                <div class="artifact-picks">
                  <div v-for="g in selectedCompDetail.composition.specialGroups.radiants" :key="g.unitId" class="artifact-pick">
                    <div class="artifact-pick-main">
                      <img loading="lazy" :src="unitIcon(g.unitId)" :alt="unitName(g.unitId)" class="mini-unit" />
                      <span class="artifact-carrier">{{ unitName(g.unitId) }}</span>
                      <img loading="lazy" v-for="it in g.items" :key="it.id" :src="itemIcon(it.id)" class="mini-item" :title="itemName(it.id) + ' 均 ' + it.avgPlacement.toFixed(2)" />
                    </div>
                  </div>
                </div>
              </div>
              <div v-if="selectedCompDetail.composition.specialGroups.emblems.length" class="special-block">
                <h4 class="special-lbl">纹章</h4>
                <div class="artifact-picks">
                  <div v-for="g in selectedCompDetail.composition.specialGroups.emblems" :key="g.itemId" class="artifact-pick">
                    <div class="artifact-pick-main">
                      <img loading="lazy" :src="itemIcon(g.itemId)" :alt="itemName(g.itemId)" class="mini-item" />
                      <span class="artifact-name">{{ itemName(g.itemId) }}</span>
                      <span class="muted tiny">均 {{ g.avgPlacement.toFixed(2) }} · 出现率 {{ (g.appearanceRate * 100).toFixed(1) }}%</span>
                      <img loading="lazy" v-for="c in g.carriers" :key="c.unit_id" :src="unitIcon(c.unit_id)" class="mini-unit" :title="unitName(c.unit_id) + (c.best ? ' ★' : '')" />
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
                    <img loading="lazy" :src="tfImg(p.artifactIcon)" :alt="p.artifactName" class="mini-item" />
                    <span class="artifact-name">{{ p.artifactName }}</span>
                    <span class="muted tiny">→</span>
                    <img loading="lazy" :src="tfImg(p.carrierIcon)" :alt="p.carrierName" class="mini-unit" />
                    <span class="artifact-carrier">{{ p.carrierName }}</span>
                    <span class="muted tiny">强度 {{ p.strength.toFixed(3) }}</span>
                  </div>
                  <div v-if="p.topPair" class="artifact-pair">
                    <img loading="lazy" v-for="it in p.topPair.items" :key="it.itemId" :src="tfImg(it.icon)" :alt="it.name" class="mini-item" :title="it.name" />
                  </div>
                </div>
              </div>
            </section>
          </div>
        </template>

        <template v-else>
          <template v-if="tftData">
          <section v-if="savedComps.length" class="board-section">
            <h3>已收藏 ({{ savedComps.length }})</h3>
            <div class="comp-grid">
              <div v-for="s in savedCompDetail" :key="s.slug" class="comp-card" @click="showCompDetail(s.slug)">
                <div class="comp-card-head">
                  <img loading="lazy" :src="tfImg(s.card.compIcon)" :alt="s.card.display_name_cn" class="comp-card-icon" />
                  <div class="comp-card-title">
                    <span class="comp-card-name">{{ s.card.display_name_cn }}</span>
                    <span class="comp-card-en muted">{{ s.card.display_name_en }}</span>
                  </div>
                </div>
                <div class="comp-card-stats">
                  <div class="comp-card-stat">
                    <span class="comp-card-val">{{ s.card.averagePlacementValue.toFixed(2) }}</span>
                    <span class="comp-card-lbl">AVP</span>
                  </div>
                </div>
                <button class="ghost small" @click.stop="toggleSavedComp(s.slug)">✕ 移除</button>
              </div>
            </div>
          </section>
          <section class="board-section">
            <h3>全部阵容（{{ tftData.compositionCards.length }} 套，含变体）</h3>
            <div class="comp-grid">
              <div v-for="c in tftData.compositionCards" :key="c.slug" class="comp-card" @click="showCompDetail(c.slug)">
                <div class="comp-card-head">
                  <img loading="lazy" :src="tfImg(c.compIcon)" :alt="c.display_name_cn" class="comp-card-icon" />
                  <div class="comp-card-title">
                    <span class="comp-card-name">{{ c.display_name_cn }}</span>
                    <span class="comp-card-en muted">{{ c.display_name_en }}</span>
                  </div>
                  <span v-if="isVariantComp(c.slug)" class="variant-badge" :title="`${compName(variantBaseOf(c.slug))} 的变体`">变体</span>
                  <span class="comp-cost-tag">${{ c.compCost }}</span>
                </div>
                <div class="comp-card-stats">
                  <div class="comp-card-stat">
                    <span class="comp-card-val">{{ c.averagePlacementValue.toFixed(2) }}</span>
                    <span class="comp-card-lbl">AVP</span>
                  </div>
                  <div class="comp-card-stat">
                    <span class="comp-card-val">{{ (statOf(c.slug).playRate * 100).toFixed(2) }}%</span>
                    <span class="comp-card-lbl">出场率</span>
                  </div>
                </div>
                <button class="ghost small" :class="{ favorited: savedComps.includes(c.slug) }" @click.stop="toggleSavedComp(c.slug)">
                  {{ savedComps.includes(c.slug) ? "★ 已收藏" : "☆ 收藏" }}
                </button>
                <button
                  class="ghost small pin-btn"
                  :class="{ pinned: effectivePin === c.slug }"
                  @click.stop="setPinnedComp(effectivePin === c.slug ? null : c.slug)"
                >
                  {{ effectivePin === c.slug ? "📌 当前" : "📌 Pin" }}
                </button>
              </div>
            </div>
          </section>
        </template>
        <p v-else-if="!metaLoading" class="empty">加载中…</p>
        </template>
      </template>

      <!-- 设置视图 -->
      <template v-else-if="view === 'settings'">
        <header class="topbar">
          <h2>设置</h2>
        </header>

        <section class="board-section">
          <h3>数据更新</h3>
          <div class="settings-row">
            <span class="settings-label">当前版本</span>
            <span class="settings-value">{{ tftData?.patch || '—' }}</span>
          </div>
          <div class="settings-row">
            <span class="settings-label">样本量</span>
            <span class="settings-value">{{ tftData?.sampleSize?.toLocaleString() || '—' }}</span>
          </div>
          <div class="settings-row">
            <span class="settings-label">更新时间</span>
            <span class="settings-value">{{ tftData ? new Date(tftData.timestamp).toLocaleString("zh-CN") : '—' }}</span>
          </div>
          <div class="settings-row">
            <button class="ghost small" @click="checkForUpdate" :disabled="updateCheck.checking">
              {{ updateCheck.checking ? "检查中…" : "检查更新" }}
            </button>
            <button class="ghost small" @click="downloadUpdate" :disabled="updating">
              {{ updating ? "下载中…" : "在线下载最新数据" }}
            </button>
            <button class="ghost small" @click="getMeta(true)" :disabled="metaLoading">
              {{ metaLoading ? "刷新中…" : "强制刷新数据" }}
            </button>
          </div>
          <template v-if="!updateCheck.checking && updateCheck.latest_patch">
            <div class="settings-row">
              <span class="settings-label">线上版本</span>
              <span class="settings-value">{{ updateCheck.latest_patch || '—' }}</span>
            </div>
            <div v-if="updateCheck.has_update" class="update-banner">
              检测到新版本 {{ updateCheck.latest_patch }}（本地 {{ updateCheck.local_patch }}），点击「在线下载最新数据」自动更新。
            </div>
            <div v-else class="update-banner ok">数据已是最新版本。</div>
          </template>
        </section>

        <section class="board-section">
          <h3>游戏内悬浮窗</h3>
          <div class="settings-row">
            <span class="settings-label">对局中显示阶段/金币/血量 + 版本推荐</span>
            <button class="ghost small" @click="toggleOverlay">
              打开 / 隐藏
            </button>
          </div>
          <div class="settings-row"><span class="settings-label">Ctrl/Cmd + 0</span><span class="settings-value">切换游戏内悬浮窗</span></div>
          <p class="hint">悬浮窗为置顶小窗，可拖动到游戏画面任意位置。对局中自动刷新实时状态。</p>
        </section>

        <section class="board-section">
          <h3>键盘快捷键</h3>
          <div class="settings-row"><span class="settings-label">Ctrl/Cmd + 1</span><span class="settings-value">对局历史</span></div>
          <div class="settings-row"><span class="settings-label">Ctrl/Cmd + 2</span><span class="settings-value">统计</span></div>
          <div class="settings-row"><span class="settings-label">Ctrl/Cmd + 3</span><span class="settings-value">阵容列表</span></div>
          <div class="settings-row"><span class="settings-label">Ctrl/Cmd + 4</span><span class="settings-value">前期</span></div>
          <div class="settings-row"><span class="settings-label">Ctrl/Cmd + 5</span><span class="settings-value">阵容构建器</span></div>
          <div class="settings-row"><span class="settings-label">Ctrl/Cmd + 6</span><span class="settings-value">数据检索器</span></div>
          <div class="settings-row"><span class="settings-label">Ctrl/Cmd + 7</span><span class="settings-value">羁绊</span></div>
          <div class="settings-row"><span class="settings-label">Ctrl/Cmd + 8</span><span class="settings-value">强化符文</span></div>
          <div class="settings-row"><span class="settings-label">Ctrl/Cmd + 0</span><span class="settings-value">开关游戏内悬浮窗</span></div>
          <p class="hint" style="margin-top: 8px">侧边栏分组可折叠；Meta 数据 / 机制手册下的二级菜单也可用数字键切换。</p>
        </section>

        <section class="board-section">
          <h3>关于</h3>
          <div class="settings-row"><span class="settings-label">应用</span><span class="settings-value">{{ info?.name }} v{{ info?.version }}</span></div>
          <div class="settings-row"><span class="settings-label">数据库</span><span class="settings-value">{{ info?.db_path }}</span></div>
          <div class="settings-row"><span class="settings-label">LCU</span><span class="settings-value">{{ info?.lcu_connected ? '已连接' : '未连接' }}</span></div>
          <p class="hint" style="margin-top: 8px">数据来源：TFTable (tftable.cc) + CDragon。仅用于学习与娱乐，非 Riot 官方产品。</p>
        </section>
      </template>

      <!-- Meta 视图 -->
      <template v-else-if="view === 'meta'">
        <header class="topbar">
          <h2>{{ ({
            overview: 'Meta总览',
            trends: '阵容趋势',
            daily: '每日数据',
            tierlist: '装备梯队',
            unitItems: '按弈子',
            itemGuide: '按装备',
          })[metaTab] || 'Meta' }}</h2>
          <button @click="getMeta(true)" :disabled="metaLoading">
            {{ metaLoading ? "加载中…" : "刷新" }}
          </button>
        </header>
        <p v-if="lastSync" class="hint" style="margin: 0 0 8px">{{ lastSync }}</p>
        <p v-if="tftData" class="hint" style="margin: 0 0 8px; font-size:11px; color:var(--faint)">更新时间: {{ new Date(tftData.timestamp).toLocaleString("zh-CN") }}</p>
        <p v-if="dataStale && !metaLoading" class="stale-banner" style="margin: 0 0 8px; padding: 6px 10px; border-radius: 6px; font-size: 12px; background: rgba(240,194,75,0.08); border: 1px solid rgba(240,194,75,0.3); color: var(--gold); cursor: pointer" @click="getMeta(true)">数据可能已过期，点击刷新</p>

        <template v-if="tftData">
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
                      <img loading="lazy" :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
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
                      <img loading="lazy" :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
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
                      <img loading="lazy" :src="traitIcon(t.trait_id)" :alt="t.trait_name_cn" class="top-icon" />
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
                      <img loading="lazy" :src="traitIcon(t.trait_id)" :alt="t.trait_name_cn" class="top-icon" />
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
                    <img loading="lazy" :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
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
                    <img loading="lazy" :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
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
                    <img loading="lazy" :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
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
                    <img loading="lazy" :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
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
                    <img loading="lazy" :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
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
                    <img loading="lazy" :src="compCardIcon(c.slug)" :alt="compName(c.slug)" class="top-icon" />
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
                    <img loading="lazy" :src="itemIcon(r.artifact_id)" :alt="r.artifact_id" class="mini-item" />
                    <span class="tierlist-name">{{ itemName(r.artifact_id) }}</span>
                  </div>
                  <div class="tierlist-comps">
                    <div v-for="cc in r.comps" :key="cc.comp_id" class="tierlist-comp" @click="showCompDetail(cc.comp_id)">
                      <img loading="lazy" :src="compCardIcon(cc.comp_id)" :alt="compName(cc.comp_id)" class="mini-unit" />
                      <span class="tierlist-comp-name">{{ compName(cc.comp_id) }}</span>
                      <span class="tierlist-comp-strength">{{ cc.item_strength.toFixed(2) }}</span>
                      <img loading="lazy" v-if="cc.top_pair" v-for="it in cc.top_pair.item_ids" :key="it" :src="itemIcon(it)" class="mini-item" :title="itemName(it)" />
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
                    <img loading="lazy" :src="itemIcon(r.artifact_id)" :alt="r.artifact_id" class="mini-item" />
                    <span class="tierlist-name">{{ itemName(r.artifact_id) }}</span>
                  </div>
                  <div class="tierlist-comps">
                    <div v-for="cc in r.comps" :key="cc.comp_id" class="tierlist-comp" @click="showCompDetail(cc.comp_id)">
                      <img loading="lazy" :src="compCardIcon(cc.comp_id)" :alt="compName(cc.comp_id)" class="mini-unit" />
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
                      <img loading="lazy" :src="itemIcon(e.artifact_id)" :alt="e.artifact_id" class="mini-item" />
                      <span class="tierlist-name">{{ itemName(e.artifact_id) }}</span>
                    </div>
                    <div class="tierlist-comps">
                      <div v-for="cc in e.comps" :key="cc.comp_id" class="tierlist-comp" @click="showCompDetail(cc.comp_id)">
                        <img loading="lazy" :src="compCardIcon(cc.comp_id)" :alt="compName(cc.comp_id)" class="mini-unit" />
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
                    <img loading="lazy" :src="unitIcon(u.unitId)" :alt="unitName(u.unitId)" class="champ-icon" />
                    <span class="champ-name">{{ unitName(u.unitId) }}</span>
                    <span class="muted tiny" style="font-size:10px">{{ unitNameEn(u.unitId) }}</span>
                    <span class="muted tiny">${{ u.unitCost }}</span>
                    <span v-if="u.unitPriority != null" class="muted tiny" style="margin-left:auto">投资优先度 {{ u.unitPriority.toFixed(2) }}</span>
                  </div>
                  <div v-if="u.bisItemIds && u.bisItemIds.length" class="unit-bis">
                    <span class="unit-bucket-lbl">BiS</span>
                    <img loading="lazy" v-for="it in u.bisItemIds" :key="it" :src="itemIcon(it)" class="mini-item" :title="itemName(it)" />
                  </div>
                  <div class="unit-item-buckets">
                    <div v-if="u.buckets.Core.length" class="unit-bucket">
                      <span class="unit-bucket-lbl">核心</span>
                      <img loading="lazy" v-for="it in u.buckets.Core" :key="it.itemId" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId) + ' 必需度 ' + it.necessity.toFixed(2)" />
                    </div>
                    <div v-if="u.buckets.Important.length" class="unit-bucket">
                      <span class="unit-bucket-lbl">重要</span>
                      <img loading="lazy" v-for="it in u.buckets.Important" :key="it.itemId" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId)" />
                    </div>
                    <div v-if="u.buckets.Optional.length" class="unit-bucket">
                      <span class="unit-bucket-lbl">可选</span>
                      <img loading="lazy" v-for="it in u.buckets.Optional" :key="it.itemId" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId)" />
                    </div>
                    <div v-if="u.buckets.HighFrequency.length" class="unit-bucket">
                      <span class="unit-bucket-lbl">高频</span>
                      <img loading="lazy" v-for="it in u.buckets.HighFrequency" :key="it.itemId" :src="itemIcon(it.itemId)" class="mini-item" :title="itemName(it.itemId) + ' 需求度 ' + it.necessity.toFixed(2)" />
                    </div>
                  </div>
                  <div v-if="u.artifacts.length" class="unit-artifacts">
                    <span class="muted tiny">神器:</span>
                    <img loading="lazy" v-for="a in u.artifacts" :key="a.itemId" :src="itemIcon(a.itemId)" class="mini-item" :title="itemName(a.itemId) + ' 均 ' + a.avgPlacement.toFixed(2)" />
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
                    <img loading="lazy" :src="itemIcon(g.itemId)" :alt="itemName(g.itemId)" class="champ-icon" />
                    <span class="champ-name">{{ itemName(g.itemId) }}</span>
                    <span class="muted tiny" style="font-size:10px">{{ g.itemName }}</span>
                  </div>
                  <div class="item-guide-buckets">
                    <div v-for="(units, cost) in sortedBuckets(g.costBuckets)" :key="cost" class="guide-bucket">
                      <span class="unit-bucket-lbl">{{ cost }}费</span>
                      <img loading="lazy" v-for="u in units" :key="u.unitId" :src="unitIcon(u.unitId)" class="mini-unit" :title="unitName(u.unitId) + ' 必需度 ' + u.necessity.toFixed(3)" />
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

        <!-- 筛选栏 -->
        <div v-if="matches.length" class="filter-bar">
          <input
            v-model="matchSearch"
            class="search-input"
            placeholder="搜索模式/日期…"
          />
          <select v-model="matchQueueFilter" class="filter-select">
            <option value="all">全部模式</option>
            <option v-for="q in matchQueues" :key="q" :value="q">{{ queueLabel(q) }}</option>
          </select>
          <div class="filter-group">
            <button class="tab" :class="{ active: matchPlaceFilter === 'all' }" @click="matchPlaceFilter = 'all'">全部</button>
            <button class="tab" :class="{ active: matchPlaceFilter === 'win' }" @click="matchPlaceFilter = 'win'">吃鸡</button>
            <button class="tab" :class="{ active: matchPlaceFilter === 'top4' }" @click="matchPlaceFilter = 'top4'">前四</button>
            <button class="tab" :class="{ active: matchPlaceFilter === 'out' }" @click="matchPlaceFilter = 'out'">后四</button>
          </div>
          <span class="muted tiny">{{ filteredMatches.length }} / {{ matches.length }} 局</span>
        </div>

        <!-- 加载骨架屏 -->
        <div v-if="loading" class="match-skeleton">
          <div v-for="i in 8" :key="i" class="skel-row">
            <span class="skel skel-placement"></span>
            <span class="skel skel-text" style="width: 180px"></span>
            <span class="skel skel-text" style="width: 140px"></span>
            <span class="skel skel-text" style="width: 40px"></span>
          </div>
        </div>

        <p v-else-if="matches.length === 0" class="empty">
          还没有记录对局。打开客户端登录后，点击「同步战绩」。
        </p>

        <p v-else-if="filteredMatches.length === 0" class="empty">
          没有符合条件的对局，试试调整筛选条件。
        </p>

        <table v-if="filteredMatches.length > 0" class="match-table">
          <thead>
            <tr>
              <th>名次</th>
              <th>模式</th>
              <th>日期</th>
              <th>等级</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="m in filteredMatches" :key="m.game_id" @click="openMatch(m.game_id)">
              <td class="placement" :class="`p${m.placement}`">#{{ m.placement }}</td>
              <td>{{ queueLabel(m.queue_id) || m.game_mode || "—" }}</td>
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
                <img loading="lazy" v-if="t.icon_url" :src="t.icon_url" :alt="t.name" class="trait-icon" />
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
                  <img loading="lazy" v-if="u.icon_url" :src="u.icon_url" :alt="u.name" class="unit-icon" />
                  <div v-if="u.tier" class="stars">
                    {{ "★".repeat(u.tier) }}
                  </div>
                </div>
                <div class="unit-name">{{ u.name || u.character_id }}</div>
                <div class="unit-items">
                  <div v-for="it in u.items" :key="it.id" class="item" :title="it.name">
                    <img loading="lazy" v-if="it.icon_url" :src="it.icon_url" :alt="it.name" />
                    <span v-else class="item-fallback">{{ it.name || it.id }}</span>
                  </div>
                </div>
              </div>
            </div>
          </section>

          <!-- 站位棋盘（无真实站位时展示已匹配阵容的参考预设站位） -->
          <section v-if="matchBoard && matchBoard.hasBoard" class="board-section">
            <h3>站位 <span v-if="matchBoard.isReference" class="muted">配合参考 · {{ matchBoard.compName }}</span></h3>
            <div class="match-board">
              <div
                v-for="(cell, idx) in matchBoard.cells"
                :key="idx"
                class="board-cell"
              >
                <div v-if="cell" class="cell-unit" :class="{ missing: !cell.held }">
                  <img loading="lazy" :src="cell.unit.icon" :alt="cell.unit.name" class="cell-icon" :style="{ borderColor: costColor(cell.unit.cost) }" />
                  <span v-if="cell.tier" class="cell-stars">{{ "★".repeat(cell.tier) }}</span>
                  <span v-if="!cell.held" class="cell-missing" title="本局未持有该棋子">缺</span>
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


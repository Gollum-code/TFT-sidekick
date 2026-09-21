export interface ChampionData {
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

export interface TraitTierData {
  min_units: number;
  max_units: number;
  style: number;
}

export interface TraitUnit {
  id: string;
  name: string;
  icon_url: string;
}

export interface TraitData {
  id: string;
  name: string;
  icon_url: string;
  desc: string;
  tiers: TraitTierData[];
  units: TraitUnit[];
}

export interface ItemData {
  id: string;
  name: string;
  icon_url: string;
  composition: string[];
  desc: string;
}

export interface ShopOddsLevel {
  level: number;
  odds: number[];
}

export interface RulesSection {
  title: string;
  rows: string[][];
}

// ===== TFTable 数据结构 =====
export interface TFTraitShare {
  trait_id: string;
  trait_name_cn: string;
  trait_name_en: string;
  tier: number | null;
  units_required: number | null;
  wins: number;
}
export interface TFCompShare { slug: string; wins: number; }
export interface TFMetaStats {
  metadata: { patch: string; ranked_tiers: string[]; total_sample_size: number; timestamp: string };
  winShare: { sample_size: number; compositions: TFCompShare[]; traits: TFTraitShare[] };
  top4Share: { sample_size: number; compositions: TFCompShare[]; traits: TFTraitShare[] };
  balance: { involution_index: number; involution_k: number; diversity_index: number; notes: { involution: string; diversity: string } };
  core_cost_share: { win_share: Record<string, number>; top4_share: Record<string, number> };
}
export interface TFCompositionCard {
  key: string; slug: string; display_name_cn: string; display_name_en: string;
  compIcon: string; compCost: number; averagePlacementValue: number;
}
export interface TFStrengthSeries { key: string; slug: string; points: { date: string; strength: number }[]; }
export interface TFCompStrengthTrends {
  patch: string; comparisonPatch: string; generatedAt: string; dates: string[];
  series: TFStrengthSeries[]; patchChangeIndices: number[];
  dateToPatch: Record<string, string>;
  balance_daily: Record<string, any>;
}
export interface TFCompSummary { slug: string; avgPlacementValue: number; trendDelta: number; playRateDelta: number; winShareDelta: number; }
export interface TFCompStat { strength: number; playRate: number; strengthDelta: number; }
export interface TFRowDelta { text: string; positive: boolean | null; value: number | null; }
export interface TFTrendRow {
  slug: string; name: string; playRate: string; playRateDelta: TFRowDelta;
  avgPlacement: string; avgPlacementDelta: TFRowDelta; avgPlacementValue: number;
  versionDeltaAvg: TFRowDelta | null; versionDeltaPlay: TFRowDelta | null; versionDeltaValue: number | null;
  dateMetrics: { date: string; avgPlacement: string; rank: number | null }[];
}
export interface TFTrendItem {
  slug: string; name: string; playRate: string; playRateDelta: TFRowDelta;
  winShare: string | null; winShareDelta: TFRowDelta | null;
  top4Share: string | null; top4ShareDelta: TFRowDelta | null;
  avgPlacement: string; avgPlacementDelta: TFRowDelta;
}
export interface TFUnitItem {
  unitId: string; unitCost: number; unitPriority: number | null;
  buckets: Record<string, { itemId: string; necessity: number; priorityScore: number | null }[]>;
  bisItemIds: string[];
  artifacts: { itemId: string; avgPlacement: number }[];
  coreCompStrengths: { comp_key: string }[];
}
export interface TFItemGuideEntry {
  itemId: string; itemName: string;
  costBuckets: Record<string, { unitId: string; normalized: number; necessity: number }[]>;
  compositions: { slug: string; name: string; unit_id: string }[];
  type: string;
}
export interface TFTierlistComp { comp_id: string; item_strength: number; top_carrier_unit_id: string; top_pair?: { item_ids: string[] }; }
export interface TFTierlistRow { artifact_id: string; comps: TFTierlistComp[]; }
export interface TFCompDetailUnit { id: string; name: string; nameCn: string; nameEn: string; cost: number; builds: { index: number; items: { id: string }[] }[]; }
export interface TFCompDetailTrait { id: string; icon: string; unitsRequired: number; tierIndex: number | null; }
export interface TFPriorityRow {
  unitId: string; unitCost: number; unitPriority: number | null; bisItemIds: string[] | null;
  buckets: Record<string, { itemId: string; necessity: number; priorityScore: number | null }[]>;
}
export interface TFArtifactPick {
  kind: string; artifactId: string; artifactName: string; artifactIcon: string;
  carrierUnitId: string; carrierName: string; carrierIcon: string; carrierCost: number;
  strength: number; topPair: { items: { itemId: string; name: string; icon: string }[] } | null;
}
export interface TFCompDetail {
  slug: string; key: string; title: string; titles: Record<string, string>;
  overview: {
    stats: { avgPlacement: string; avgPlacementValue: number; winRate: string; winRateValue: number };
    coreUnits: TFCompDetailUnit[]; traits: TFCompDetailTrait[]; teamCode: string;
  };
  summary: { sampleSize: number; winRate: string };
  metatftLink?: string;
  filters?: { expr: string; carrierExclusions: string[]; exprByLocale: Record<string, string>; carrierExclusionsByLocale: Record<string, string> };
  variants: { is_primary: boolean; variant_id: string; variant_label: string; clustered_stats: { share: number; avg_placement: number }; units: { id: string }[]; traits: TFCompDetailTrait[] }[];
  priorityMatrixRows: TFPriorityRow[];
  specialGroups: { artifacts: { unitId: string; items: { id: string; avgPlacement: number }[] }[]; radiants: { unitId: string; items: { id: string; avgPlacement: number }[] }[]; emblems: { itemId: string; avgPlacement: number; appearanceRate: number; carriers: { unit_id: string; best: boolean }[] }[] };
  matchups: { benefits: { key: string; delta: number }[]; counteredBy: { key: string; delta: number }[]; mutualConstraints: { key: string; delta: number }[] } | null;
  artifactPicks: { compStrength: number | null; picks: TFArtifactPick[] } | null;
  dataSource: { patch: string; sample_count: number };
}
export interface TFTtableData {
  patch: string; ranked_tiers: string[]; sampleSize: number; timestamp: string;
  metaStats: TFMetaStats; compositionCards: TFCompositionCard[];
  compStrengthTrends: TFCompStrengthTrends;
  compAtlas: { compositionSummaries: TFCompSummary[]; compStats: Record<string, TFCompStat> };
  trends: { dateColumns: string[]; versionDeltaMode: string; versionDeltaWindowHint: string; compositionRows: TFTrendRow[]; compositionTrendUp: TFTrendItem[]; compositionTrendDown: TFTrendItem[]; winshareTrendUp: TFTrendItem[]; winshareTrendDown: TFTrendItem[] };
  dailyStats: { dateColumns: string[]; versionDeltaMode: string; versionDeltaWindowHint: string; compositionRows: TFTrendRow[]; compositionTrendUp: TFTrendItem[]; compositionTrendDown: TFTrendItem[]; winshareTrendUp: TFTrendItem[]; winshareTrendDown: TFTrendItem[] };
  unitItems: TFUnitItem[];
  itemGuide: { items: TFItemGuideEntry[]; artifacts: TFItemGuideEntry[]; emblems: TFItemGuideEntry[] };
  itemTierlist: { generated_at?: string; tabs: { artifacts: { rows: TFTierlistRow[] }; "anima-items": { rows: TFTierlistRow[] }; emblems: { rows: TFTierlistRow[]; essentials?: TFTierlistRow[] } } };
  compDetails: Record<string, { composition: TFCompDetail; itemCategories: Record<string, string[]>; unitCategories: Record<string, string[]> }>;
}

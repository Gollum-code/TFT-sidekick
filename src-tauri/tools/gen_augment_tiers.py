"""生成 augment_tiers.json —— 海克斯 apiName → 稀有度分级。

数据源：
  1. CDragon cdragon tft（items[] 里的 isAugment 条目）  → 提供当前赛季 apiName
  2. CDragon rcp-be-lol-game-data tftitems.json           → 提供英文 name（分级信号）

分级规则（两个信号任一命中即可，tftitems 的 nameId 与 cdragon 的 apiName 完全一致，作为 join key）：
  - nameId/name 含 Prismatic → prismatic
  - nameId/name 含 Silver    → silver
  - nameId/name 含 Gold      → gold
  - 英文名以罗马数字结尾 I/II/III → silver/gold/prismatic
  - 其余                        → standard（常规海克斯，无等级）

用法：
  python tools/gen_augment_tiers.py            # 写入 src-tauri/augment_tiers.json
  python tools/gen_augment_tiers.py -o out.json
"""

from __future__ import annotations

import argparse
import collections
import json
import os
import re
import ssl
import sys
import urllib.request

CDRAGON_TFT = "https://raw.communitydragon.org/latest/cdragon/tft/zh_cn.json"
TFTITEMS = (
    "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/"
    "global/default/v1/tftitems.json"
)
UA = "TFT-Sidekick/0.1"

ROMAN_TIER = {"I": "silver", "II": "gold", "III": "prismatic", "IV": "prismatic"}


def _ctx() -> ssl.SSLContext:
    ctx = ssl.create_default_context()
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    return ctx


def fetch_json(url: str) -> dict:
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=60, context=_ctx()) as r:
        return json.loads(r.read())


def classify(name_id: str, name: str | None) -> str:
    blob = f"{name_id} {name or ''}"
    if re.search(r"Prismatic", blob, re.I):
        return "prismatic"
    if re.search(r"Silver", blob, re.I):
        return "silver"
    if re.search(r"\bGold\b", blob, re.I):
        return "gold"
    m = re.search(r"\b(III|II|I)\b\s*$", (name or "").strip())
    if m:
        return ROMAN_TIER[m.group(1)]
    return "standard"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("-o", "--out", default=None)
    args = ap.parse_args()

    cdragon = fetch_json(CDRAGON_TFT)
    items = fetch_json(TFTITEMS)

    # 当前赛季海克斯（set_code 与 static_data.rs 的推导一致：DA_ 前缀）
    augs = {
        it["apiName"]
        for it in cdragon.get("items", [])
        if it.get("isAugment") and it.get("apiName", "").startswith("DA_")
    }
    by_name_id = {
        x["nameId"]: x for x in items if (x.get("nameId") or "").startswith("DA_")
    }

    out = {
        api: classify(api, by_name_id.get(api, {}).get("name"))
        for api in sorted(augs)
    }

    dest = args.out or os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        "augment_tiers.json",
    )
    with open(dest, "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, indent=0, sort_keys=True)

    dist = collections.Counter(out.values())
    print(f"wrote {dest}: {len(out)} entries")
    print("dist:", dict(dist))
    return 0


if __name__ == "__main__":
    sys.exit(main())
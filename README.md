**TFT Sidekick** — A TFT desktop companion tool

TFT Sidekick is a lightweight desktop companion for Teamfight Tactics players on the Chinese server. It combines three core features in one seamless experience:

**1. Personal Match History** (Core Feature)
- Automatic match recording via LCU (League Client Update)
- Full board visualization (final comp + equipment + traits)
- Basic statistics (win rate, average placement, popular comps, level trend)
- In-game hover window and post-match toast notification

**2. Hidden Data Tables** (Datamine Reference)
- Complete offline tables for:
  - Shop odds, augment odds, encounter odds, loot orb contents, player damage
  - Item recipes and components
  - Champion stats, abilities, and synergies
  - Trait effects and tiers
  - Emblem recipes and patch schedule
- All data cached from CDragon datamine (updated with each patch)

**3. High-Elo Meta Analysis** (Community Statistics)
- Aggregated win share and top-4 share by comp from high-elo (Master+/Challenger) matches
- Item tier list and best items per unit
- Cached and updated data from Riot API (international servers)
- Meta transferable across regions (same patch, same units/items)

**Key Features**
- Always-on-top floating window for quick reference during games
- One-click synchronization with live client
- Dark, clean, high-contrast UI optimized for TFT players
- Cross-platform (Windows primary, future macOS/Linux support)
- No third-party account login required

**Privacy & Compliance**
- Only reads local LCU data (no account credentials stored)
- Meta data is pre-crawled by the project (not per-user API keys)
- CDragon datamine for static tables (community source, no reverse engineering)
- Strictly follows Riot's ToS

**Tech Stack**
- Desktop: Tauri 2 (Rust + WebView)
- Backend: Rust (LCU via remoting, Riot API via official endpoints)
- Frontend: Vue 3 + TypeScript + Vite
- Storage: SQLite (local match history + cached data)
- Data Sources: LCU (国服), CDragon (datamine), Riot API (international meta)

**Development**
```bash
npm install
npm run tauri dev
```

**Meta Data Prep**
Meta 页（comps / items / 阵容详情 / 趋势 / 装备梯队）读取 `tftable_data.json`，
由应用内「设置 → 在线更新」从 tftable.cc 抓取重建，无需 API key。

手动重建（可选）：
```bash
# 在应用内点击「在线更新」，或直接调用后端 update_tftable_data 命令
```
`tftable_data.json` 已随仓库提交作为兜底；应用启动时若本地缺失会使用内嵌副本。

**Augment Tiers**
海克斯稀有度表 `src-tauri/augment_tiers.json`（`apiName` → silver / gold / prismatic / standard）
随补丁更新后重新生成：

```bash
python src-tauri/tools/gen_augment_tiers.py
```

分级信号来自 CDragon `tftitems.json` 的英文名（`Silver` / `Gold` / `Prismatic`
或罗马数字 `I` / `II` / `III`）。表中未收录时后端会按 `apiName` 命名启发式兜底。

**Future Roadmap**
- International server support (add Riot API for global meta)
- Custom comp search and filter
- Cloud sync for multi-device history
- AI replay coach (future paid feature)

**License**
This software is for personal use only. It is not affiliated with Riot Games or any official game server. All data is sourced from public APIs and community datamines. Use at your own risk.

---

**产品组**：TFT Sidekick Team  
**网址**：https://github.com/Gollum-code/TFT-sidekick  
**说明**：This is an independent fan project for Teamfight Tactics players. It is not endorsed by Riot Games. All data is sourced from public APIs and community datamines. Use at your own risk.
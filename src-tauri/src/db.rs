use std::path::Path;

use rusqlite::{params, Connection};
use thiserror::Error;

use crate::models::{MatchDetail, MatchSummary, MatchTrait, MatchUnit};

#[derive(Debug, Error)]
pub enum DbError {
    #[error("数据库错误: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),
}

pub struct Database {
    conn: Connection,
    path: std::path::PathBuf,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS matches (
    game_id         TEXT PRIMARY KEY,
    queue_id        INTEGER NOT NULL DEFAULT 0,
    game_mode       TEXT,
    game_version    TEXT,
    game_datetime   TEXT NOT NULL,
    game_length     REAL,
    placement       INTEGER NOT NULL,
    players_eliminated INTEGER,
    level           INTEGER,
    augments        TEXT,
    created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS match_units (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id      TEXT NOT NULL REFERENCES matches(game_id) ON DELETE CASCADE,
    character_id TEXT NOT NULL,
    name         TEXT,
    rarity       INTEGER,
    tier         INTEGER,
    items        TEXT,
    position_x   INTEGER,
    position_y   INTEGER
);
CREATE INDEX IF NOT EXISTS idx_match_units_game ON match_units(game_id);

CREATE TABLE IF NOT EXISTS match_traits (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id      TEXT NOT NULL REFERENCES matches(game_id) ON DELETE CASCADE,
    trait_id     TEXT NOT NULL,
    name         TEXT,
    tier_current INTEGER,
    tier_total   INTEGER
);
CREATE INDEX IF NOT EXISTS idx_match_traits_game ON match_traits(game_id);
"#;

impl Database {
    pub fn open(path: &Path) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>, DbError> {
        let mut stmt = self.conn.prepare("SELECT value FROM meta WHERE key = ?1")?;
        let mut rows = stmt.query([key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            [key, value],
        )?;
        Ok(())
    }

    pub fn list_matches(&self, limit: u32) -> Result<Vec<MatchSummary>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT game_id, game_mode, game_version, game_datetime, placement, level, augments
             FROM matches ORDER BY game_datetime DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |row| {
            let augments_json: Option<String> = row.get(6)?;
            let augments = augments_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e)))?
                .unwrap_or_default();
            Ok(MatchSummary {
                game_id: row.get(0)?,
                game_mode: row.get(1)?,
                game_version: row.get(2)?,
                game_datetime: row.get(3)?,
                placement: row.get(4)?,
                level: row.get(5)?,
                augments,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn get_match(&self, game_id: &str) -> Result<MatchDetail, DbError> {
        let summary = self
            .conn
            .query_row(
                "SELECT game_id, game_mode, game_version, game_datetime, placement, level, augments
                 FROM matches WHERE game_id = ?1",
                [game_id],
                |row| {
                    let augments_json: Option<String> = row.get(6)?;
                    let augments = augments_json
                        .as_deref()
                        .map(serde_json::from_str)
                        .transpose()
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e)))?
                        .unwrap_or_default();
                    Ok(MatchSummary {
                        game_id: row.get(0)?,
                        game_mode: row.get(1)?,
                        game_version: row.get(2)?,
                        game_datetime: row.get(3)?,
                        placement: row.get(4)?,
                        level: row.get(5)?,
                        augments,
                    })
                },
            )
            .map_err(DbError::from)?;

        let mut unit_stmt = self.conn.prepare(
            "SELECT character_id, name, rarity, tier, items, position_x, position_y
             FROM match_units WHERE game_id = ?1 ORDER BY id",
        )?;
        let units = unit_stmt
            .query_map([game_id], |row| {
                let items_json: Option<String> = row.get(4)?;
                let items = items_json
                    .as_deref()
                    .map(serde_json::from_str)
                    .transpose()
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e)))?
                    .unwrap_or_default();
                Ok(MatchUnit {
                    character_id: row.get(0)?,
                    name: row.get(1)?,
                    rarity: row.get(2)?,
                    tier: row.get(3)?,
                    items,
                    position_x: row.get(5)?,
                    position_y: row.get(6)?,
                })
            })
            .map_err(DbError::from)?
            .collect::<Result<Vec<_>, _>>()?;

        let mut trait_stmt = self.conn.prepare(
            "SELECT trait_id, name, tier_current, tier_total
             FROM match_traits WHERE game_id = ?1 ORDER BY id",
        )?;
        let traits = trait_stmt
            .query_map([game_id], |row| {
                Ok(MatchTrait {
                    trait_id: row.get(0)?,
                    name: row.get(1)?,
                    tier_current: row.get(2)?,
                    tier_total: row.get(3)?,
                })
            })
            .map_err(DbError::from)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(MatchDetail {
            summary,
            units,
            traits,
        })
    }

    /// 落库一局对局（含阵容/羁绊）。返回是否为新插入（重复 game_id 返回 false）。
    pub fn insert_match(&self, detail: &MatchDetail) -> Result<bool, DbError> {
        let s = &detail.summary;
        let augments_json = serde_json::to_string(&s.augments)?;

        let tx = self.conn.unchecked_transaction()?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO matches
             (game_id, queue_id, game_mode, game_version, game_datetime, game_length,
              placement, players_eliminated, level, augments, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, ?9, ?10)",
            params![
                s.game_id,
                0i64,
                s.game_mode,
                s.game_version,
                s.game_datetime,
                None::<f64>,
                s.placement,
                s.level,
                augments_json,
                chrono::Utc::now().to_rfc3339(),
            ],
        )? > 0;

        if !inserted {
            tx.commit()?;
            return Ok(false);
        }

        for unit in &detail.units {
            let items_json = serde_json::to_string(&unit.items)?;
            tx.execute(
                "INSERT INTO match_units
                 (game_id, character_id, name, rarity, tier, items, position_x, position_y)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    s.game_id,
                    unit.character_id,
                    unit.name,
                    unit.rarity,
                    unit.tier,
                    items_json,
                    unit.position_x,
                    unit.position_y,
                ],
            )?;
        }

        for tr in &detail.traits {
            tx.execute(
                "INSERT INTO match_traits
                 (game_id, trait_id, name, tier_current, tier_total)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    s.game_id,
                    tr.trait_id,
                    tr.name,
                    tr.tier_current,
                    tr.tier_total,
                ],
            )?;
        }

        tx.commit()?;
        Ok(true)
    }
}

/// 原始统计（未富化名称）。
pub struct DbStats {
    pub total: u32,
    pub wins: u32,
    pub top4: u32,
    pub sum_placement: i64,
    pub sum_level: i64,
    pub level_count: u32,
    pub placement_dist: [u32; 8],
    pub top_traits: Vec<(String, u32)>,
    pub top_units: Vec<(String, u32)>,
}

impl Database {
    pub fn get_stats(&self) -> Result<DbStats, DbError> {
        let (total, wins, top4, sum_placement, sum_level, level_count): (i64, i64, i64, i64, i64, i64) =
            self.conn.query_row(
                "SELECT COUNT(*),
                        COALESCE(SUM(CASE WHEN placement = 1 THEN 1 ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN placement <= 4 THEN 1 ELSE 0 END), 0),
                        COALESCE(SUM(placement), 0),
                        COALESCE(SUM(level), 0),
                        COALESCE(SUM(CASE WHEN level IS NOT NULL THEN 1 ELSE 0 END), 0)
                 FROM matches",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )?;

        let mut placement_dist = [0u32; 8];
        {
            let mut stmt = self
                .conn
                .prepare("SELECT placement, COUNT(*) FROM matches GROUP BY placement")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?;
            for r in rows {
                let (p, c) = r?;
                if (1..=8).contains(&p) {
                    placement_dist[(p - 1) as usize] = c as u32;
                }
            }
        }

        let top_traits = self.top_by("match_traits", "trait_id")?;
        let top_units = self.top_by("match_units", "character_id")?;

        Ok(DbStats {
            total: total as u32,
            wins: wins as u32,
            top4: top4 as u32,
            sum_placement,
            sum_level,
            level_count: level_count as u32,
            placement_dist,
            top_traits,
            top_units,
        })
    }

    fn top_by(&self, table: &str, col: &str) -> Result<Vec<(String, u32)>, DbError> {
        let sql = format!(
            "SELECT {col}, COUNT(*) AS c FROM {table} GROUP BY {col} ORDER BY c DESC LIMIT 10"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u32))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MatchDetail, MatchSummary, MatchTrait, MatchUnit};

    fn sample_match(game_id: &str, placement: i32, level: Option<i32>) -> MatchDetail {
        MatchDetail {
            summary: MatchSummary {
                game_id: game_id.into(),
                game_mode: None,
                game_version: None,
                game_datetime: "2026-09-10T00:00:00Z".into(),
                placement,
                level,
                augments: vec![],
            },
            units: vec![
                MatchUnit {
                    character_id: "A".into(),
                    name: None,
                    rarity: None,
                    tier: None,
                    items: vec![],
                    position_x: None,
                    position_y: None,
                },
                MatchUnit {
                    character_id: "B".into(),
                    name: None,
                    rarity: None,
                    tier: None,
                    items: vec![],
                    position_x: None,
                    position_y: None,
                },
            ],
            traits: vec![MatchTrait {
                trait_id: "T1".into(),
                name: None,
                tier_current: None,
                tier_total: None,
            }],
        }
    }

    #[test]
    fn stats_aggregate() {
        let dir = std::env::temp_dir().join("tft_stats_test.db");
        let _ = std::fs::remove_file(&dir);
        let db = Database::open(&dir).unwrap();
        db.insert_match(&sample_match("1", 1, Some(9))).unwrap();
        db.insert_match(&sample_match("2", 3, Some(8))).unwrap();
        db.insert_match(&sample_match("3", 5, Some(7))).unwrap();
        db.insert_match(&sample_match("4", 8, None)).unwrap();

        let s = db.get_stats().unwrap();
        assert_eq!(s.total, 4);
        assert_eq!(s.wins, 1);
        assert_eq!(s.top4, 2);
        assert_eq!(s.sum_placement, 17);
        assert_eq!(s.placement_dist[0], 1);
        assert_eq!(s.placement_dist[2], 1);
        assert_eq!(s.placement_dist[4], 1);
        assert_eq!(s.placement_dist[7], 1);
        assert_eq!(s.top_units.len(), 2);
        assert_eq!(s.top_traits.len(), 1);

        let _ = std::fs::remove_file(&dir);
    }
}

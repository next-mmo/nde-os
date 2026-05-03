//! Usage tracking store — records LLM usage events for cost monitoring.

use crate::memory::types::*;
use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct UsageStore {
    conn: Arc<Mutex<Connection>>,
}

impl UsageStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self { Self { conn } }

    pub fn record(&self, record: &UsageRecord) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO usage_events (id,agent_id,timestamp,model,input_tokens,output_tokens,cost_usd,tool_calls) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![id, record.agent_id.0.to_string(), now, record.model, record.input_tokens as i64, record.output_tokens as i64, record.cost_usd, record.tool_calls as i64],
        )?;
        Ok(())
    }

    pub fn query_hourly(&self, agent_id: AgentId) -> Result<f64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let cost: f64 = conn.query_row(
            "SELECT COALESCE(SUM(cost_usd),0.0) FROM usage_events WHERE agent_id=?1 AND timestamp>datetime('now','-1 hour')",
            rusqlite::params![agent_id.0.to_string()], |row| row.get(0),
        )?;
        Ok(cost)
    }

    pub fn query_daily(&self, agent_id: AgentId) -> Result<f64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let cost: f64 = conn.query_row(
            "SELECT COALESCE(SUM(cost_usd),0.0) FROM usage_events WHERE agent_id=?1 AND timestamp>datetime('now','start of day')",
            rusqlite::params![agent_id.0.to_string()], |row| row.get(0),
        )?;
        Ok(cost)
    }

    pub fn query_summary(&self, agent_id: Option<AgentId>) -> Result<UsageSummary> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let (sql, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match agent_id {
            Some(aid) => (
                "SELECT COALESCE(SUM(input_tokens),0),COALESCE(SUM(output_tokens),0),COALESCE(SUM(cost_usd),0.0),COUNT(*),COALESCE(SUM(tool_calls),0) FROM usage_events WHERE agent_id=?1",
                vec![Box::new(aid.0.to_string())],
            ),
            None => (
                "SELECT COALESCE(SUM(input_tokens),0),COALESCE(SUM(output_tokens),0),COALESCE(SUM(cost_usd),0.0),COUNT(*),COALESCE(SUM(tool_calls),0) FROM usage_events",
                vec![],
            ),
        };
        let prefs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let s = conn.query_row(sql, prefs.as_slice(), |row| {
            Ok(UsageSummary {
                total_input_tokens: row.get::<_,i64>(0)? as u64,
                total_output_tokens: row.get::<_,i64>(1)? as u64,
                total_cost_usd: row.get(2)?,
                call_count: row.get::<_,i64>(3)? as u64,
                total_tool_calls: row.get::<_,i64>(4)? as u64,
            })
        })?;
        Ok(s)
    }

    pub fn query_by_model(&self) -> Result<Vec<ModelUsage>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut stmt = conn.prepare("SELECT model,COALESCE(SUM(cost_usd),0.0),COALESCE(SUM(input_tokens),0),COALESCE(SUM(output_tokens),0),COUNT(*) FROM usage_events GROUP BY model ORDER BY SUM(cost_usd) DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok(ModelUsage { model: row.get(0)?, total_cost_usd: row.get(1)?, total_input_tokens: row.get::<_,i64>(2)? as u64, total_output_tokens: row.get::<_,i64>(3)? as u64, call_count: row.get::<_,i64>(4)? as u64 })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(|e| e.into())
    }

    pub fn query_daily_breakdown(&self, days: u32) -> Result<Vec<DailyBreakdown>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut stmt = conn.prepare(&format!(
            "SELECT date(timestamp),COALESCE(SUM(cost_usd),0.0),COALESCE(SUM(input_tokens)+SUM(output_tokens),0),COUNT(*) FROM usage_events WHERE timestamp>datetime('now','-{days} days') GROUP BY date(timestamp) ORDER BY date(timestamp) ASC"
        ))?;
        let rows = stmt.query_map([], |row| {
            Ok(DailyBreakdown { date: row.get(0)?, cost_usd: row.get(1)?, tokens: row.get::<_,i64>(2)? as u64, calls: row.get::<_,i64>(3)? as u64 })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(|e| e.into())
    }

    pub fn cleanup_old(&self, days: u32) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let deleted = conn.execute(&format!("DELETE FROM usage_events WHERE timestamp<datetime('now','-{days} days')"), [])?;
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::migration::run_migrations;

    fn setup() -> UsageStore {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        UsageStore::new(Arc::new(Mutex::new(conn)))
    }

    #[test]
    fn test_record_and_summary() {
        let store = setup();
        let aid = AgentId::new();
        store.record(&UsageRecord { agent_id: aid, model: "haiku".into(), input_tokens: 100, output_tokens: 50, cost_usd: 0.001, tool_calls: 2 }).unwrap();
        let s = store.query_summary(Some(aid)).unwrap();
        assert_eq!(s.call_count, 1);
        assert_eq!(s.total_input_tokens, 100);
    }

    #[test]
    fn test_empty_summary() {
        let store = setup();
        let s = store.query_summary(None).unwrap();
        assert_eq!(s.call_count, 0);
    }
}

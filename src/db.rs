use rusqlite::Connection;
use thiserror::Error;

const TRIGGER_NAME: &str = "prevent_delete_on_analytics_events";

#[derive(Debug, Error)]
pub enum TriggerError {
    #[error("trigger already exists: {0}")]
    AlreadyExists(&'static str),
    #[error("trigger does not exist: {0}")]
    NotFound(&'static str),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

fn trigger_exists(conn: &Connection) -> rusqlite::Result<bool> {
    let mut stmt =
        conn.prepare("SELECT 1 FROM sqlite_master WHERE type='trigger' AND name = ?1 LIMIT 1;")?;
    let mut rows = stmt.query([TRIGGER_NAME])?;
    Ok(rows.next()?.is_some())
}

pub fn install_analytics_events_trigger(conn: &Connection) -> Result<(), TriggerError> {
    if trigger_exists(conn)? {
        return Err(TriggerError::AlreadyExists(TRIGGER_NAME));
    }
    conn.execute_batch(
        "CREATE TRIGGER prevent_delete_on_analytics_events
         BEFORE DELETE ON AnalyticsEvents
         BEGIN
           SELECT RAISE(ABORT, 'Deletion is not allowed on AnalyticsEvents table');
         END;",
    )?;
    Ok(())
}

pub fn remove_analytics_events_trigger(conn: &Connection) -> Result<(), TriggerError> {
    if !trigger_exists(conn)? {
        return Err(TriggerError::NotFound(TRIGGER_NAME));
    }
    conn.execute_batch("DROP TRIGGER prevent_delete_on_analytics_events;")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE AnalyticsEvents (
                Id TEXT PRIMARY KEY,
                Type TEXT NOT NULL,
                Timestamp TEXT NOT NULL,
                Attributes TEXT,
                Metrics TEXT
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_trigger_install_and_delete_block() {
        let conn = setup_test_db();
        install_analytics_events_trigger(&conn).unwrap();
        conn.execute(
            "INSERT INTO AnalyticsEvents (Id, Type, Timestamp, Attributes, Metrics) VALUES (?1, ?2, ?3, ?4, ?5)",
            ["row1", "AppStart", "2023-01-01T00:00:00Z", "", ""],
        )
        .unwrap();
        let err = conn.execute("DELETE FROM AnalyticsEvents", []).unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("Deletion is not allowed"));
    }

    #[test]
    fn test_trigger_remove() {
        let conn = setup_test_db();
        install_analytics_events_trigger(&conn).unwrap();
        remove_analytics_events_trigger(&conn).unwrap();
        conn.execute("DELETE FROM AnalyticsEvents", []).unwrap();
    }

    #[test]
    fn test_trigger_install_existing() {
        let conn = setup_test_db();
        install_analytics_events_trigger(&conn).unwrap();
        let err = install_analytics_events_trigger(&conn).unwrap_err();
        assert!(matches!(err, TriggerError::AlreadyExists(_)));
    }

    #[test]
    fn test_trigger_remove_missing() {
        let conn = setup_test_db();
        let err = remove_analytics_events_trigger(&conn).unwrap_err();
        assert!(matches!(err, TriggerError::NotFound(_)));
    }
}

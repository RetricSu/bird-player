use rusqlite::{Connection, DatabaseName, Error, ErrorCode, Result};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::app::constants::CONFIG_APP_NAME;

pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

impl Database {
    // The current schema version - increment this when making schema changes
    const SCHEMA_VERSION: i32 = 8;
    const BACKUP_RETENTION: usize = 10;

    pub fn new() -> Result<Self> {
        // Get the app's configuration directory
        let db_path = Self::get_database_path()?;
        let should_backup = db_path
            .metadata()
            .map(|metadata| metadata.is_file() && metadata.len() > 0)
            .unwrap_or(false);

        // Ensure the parent directory exists
        if let Some(parent) = db_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return Err(Error::SqliteFailure(
                    rusqlite::ffi::Error {
                        code: ErrorCode::CannotOpen,
                        extended_code: 0,
                    },
                    Some(format!("Failed to create directory: {}", e)),
                ));
            }
        }

        // Create or open the database connection
        let connection = Connection::open(&db_path)?;

        // Keep a consistent SQLite snapshot outside the live database before
        // any schema initialization or migration can modify it.
        if should_backup {
            Self::backup_database(&connection, &db_path)?;
        }

        // Initialize the database schema
        Self::initialize_schema(&connection)?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn get_database_path() -> Result<PathBuf> {
        let config_dir = confy::get_configuration_file_path(CONFIG_APP_NAME, None)
            .map_err(|_| rusqlite::Error::ExecuteReturnedResults)?
            .parent()
            .ok_or(rusqlite::Error::ExecuteReturnedResults)?
            .to_path_buf();

        Ok(config_dir.join("bird-player.db"))
    }

    fn backup_database(connection: &Connection, db_path: &Path) -> Result<PathBuf> {
        let config_dir = db_path.parent().ok_or(Error::InvalidPath(db_path.into()))?;
        let backup_dir = config_dir.join("backups");
        std::fs::create_dir_all(&backup_dir).map_err(|err| {
            Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: ErrorCode::CannotOpen,
                    extended_code: 0,
                },
                Some(format!("Failed to create database backup folder: {err}")),
            )
        })?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let schema_version = connection
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get::<_, i32>(0)
            })
            .unwrap_or(0);
        let backup_path = backup_dir.join(format!("bird-player-{timestamp}-v{schema_version}.db"));

        connection.backup(DatabaseName::Main, &backup_path, None)?;

        let backup_connection = Connection::open(&backup_path)?;
        let integrity: String =
            backup_connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: ErrorCode::DatabaseCorrupt,
                    extended_code: 0,
                },
                Some(format!(
                    "Database backup integrity check failed: {integrity}"
                )),
            ));
        }
        drop(backup_connection);

        Self::prune_old_backups(&backup_dir);
        tracing::info!("Created database backup at '{}'", backup_path.display());
        Ok(backup_path)
    }

    fn prune_old_backups(backup_dir: &Path) {
        let Ok(entries) = std::fs::read_dir(backup_dir) else {
            return;
        };
        let mut backups = entries
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("bird-player-") && name.ends_with(".db"))
            })
            .collect::<Vec<_>>();
        backups.sort();

        let remove_count = backups.len().saturating_sub(Self::BACKUP_RETENTION);
        for backup in backups.into_iter().take(remove_count) {
            if let Err(err) = std::fs::remove_file(&backup) {
                tracing::warn!(
                    "Failed to remove old database backup '{}': {}",
                    backup.display(),
                    err
                );
            }
        }
    }

    fn initialize_schema(connection: &Connection) -> Result<()> {
        let has_existing_app_tables = [
            "library_paths",
            "library_items",
            "pictures",
            "playlists",
            "playlist_items",
        ]
        .into_iter()
        .try_fold(false, |found, table| {
            Ok::<_, Error>(found || Self::table_exists(connection, table)?)
        })?;

        connection.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL
            )",
            [],
        )?;

        // Check current schema version
        let current_version: i32 = connection
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        if current_version == Self::SCHEMA_VERSION {
            return Ok(());
        }

        // An older app must refuse a newer database. Continuing could silently
        // discard columns or relations even if it does not drop tables.
        if current_version > Self::SCHEMA_VERSION {
            return Err(Self::unsupported_schema_error(current_version));
        }

        if current_version == 0 && !has_existing_app_tables {
            return Self::create_schema(connection);
        }

        // Unknown or incomplete legacy schemas are preserved for recovery.
        if !matches!(current_version, 4..=7) {
            return Err(Self::unsupported_schema_error(current_version));
        }

        let transaction = connection.unchecked_transaction()?;
        let mut migrated_version = current_version;

        if migrated_version == 4 {
            tracing::info!("Running database migration from version 4 to 5");

            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            let mut stmt = transaction.prepare("PRAGMA table_info(playlists)")?;
            let existing_columns: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()?;

            if !existing_columns.contains(&"description".to_string()) {
                transaction.execute("ALTER TABLE playlists ADD COLUMN description TEXT", [])?;
            }

            if !existing_columns.contains(&"created_at".to_string()) {
                transaction.execute(
                    "ALTER TABLE playlists ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0",
                    [],
                )?;
            }

            if !existing_columns.contains(&"updated_at".to_string()) {
                transaction.execute(
                    "ALTER TABLE playlists ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
                    [],
                )?;
            }

            transaction.execute(
                "UPDATE playlists SET created_at = ?1, updated_at = ?1 WHERE created_at = 0",
                [current_time],
            )?;
            transaction.execute("UPDATE schema_version SET version = 5", [])?;

            migrated_version = 5;
            tracing::info!("Database migration to version 5 completed");
        }

        if migrated_version == 5 {
            tracing::info!("Running database migration to version 6");

            let mut stmt = transaction.prepare("PRAGMA table_info(playlists)")?;
            let existing_columns: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()?;

            if !existing_columns.contains(&"sort_order".to_string()) {
                transaction.execute(
                    "ALTER TABLE playlists ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0",
                    [],
                )?;
                transaction.execute(
                    "UPDATE playlists SET sort_order = id WHERE sort_order = 0",
                    [],
                )?;
            }

            transaction.execute("UPDATE schema_version SET version = 6", [])?;
            migrated_version = 6;
            tracing::info!("Database migration to version 6 completed");
        }

        if migrated_version == 6 {
            tracing::info!("Running database migration to version 7");

            let playlist_columns = Self::table_columns(&transaction, "playlists")?;
            if !playlist_columns.contains(&"curator".to_string()) {
                transaction.execute("ALTER TABLE playlists ADD COLUMN curator TEXT", [])?;
            }
            if !playlist_columns.contains(&"booklet".to_string()) {
                transaction.execute("ALTER TABLE playlists ADD COLUMN booklet TEXT", [])?;
            }
            if !playlist_columns.contains(&"cover_mime_type".to_string()) {
                transaction.execute("ALTER TABLE playlists ADD COLUMN cover_mime_type TEXT", [])?;
            }
            if !playlist_columns.contains(&"cover_data".to_string()) {
                transaction.execute("ALTER TABLE playlists ADD COLUMN cover_data BLOB", [])?;
            }

            let playlist_item_columns = Self::table_columns(&transaction, "playlist_items")?;
            if !playlist_item_columns.contains(&"curator_note".to_string()) {
                transaction.execute(
                    "ALTER TABLE playlist_items ADD COLUMN curator_note TEXT",
                    [],
                )?;
            }

            transaction.execute("UPDATE schema_version SET version = 7", [])?;
            migrated_version = 7;
            tracing::info!("Database migration to version 7 completed");
        }

        if migrated_version == 7 {
            tracing::info!("Running database migration to version 8");

            // Duplicate path rows could be created by older builds. Point
            // every item at one stable row, mark duplicated empty folders for
            // a startup rescan, then enforce uniqueness for future writes.
            transaction.execute(
                "UPDATE library_paths
                 SET status = 0
                 WHERE path IN (
                     SELECT path FROM library_paths GROUP BY path HAVING COUNT(*) > 1
                 )",
                [],
            )?;
            transaction.execute(
                "UPDATE library_items
                 SET library_path_id = (
                     SELECT MIN(duplicate.id)
                     FROM library_paths current
                     JOIN library_paths duplicate ON duplicate.path = current.path
                     WHERE current.id = library_items.library_path_id
                 )
                 WHERE library_path_id IN (
                     SELECT id
                     FROM library_paths
                     WHERE path IN (
                         SELECT path
                         FROM library_paths
                         GROUP BY path
                         HAVING COUNT(*) > 1
                     )
                 )",
                [],
            )?;
            transaction.execute(
                "DELETE FROM library_paths
                 WHERE id NOT IN (
                     SELECT MIN(id) FROM library_paths GROUP BY path
                 )",
                [],
            )?;
            transaction.execute(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_library_paths_path
                 ON library_paths(path)",
                [],
            )?;
            transaction.execute("UPDATE schema_version SET version = 8", [])?;
            tracing::info!("Database migration to version 8 completed");
        }

        transaction.commit()
    }

    fn create_schema(connection: &Connection) -> Result<()> {
        let transaction = connection.unchecked_transaction()?;
        transaction.execute(
            "CREATE TABLE IF NOT EXISTS library_paths (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL,
                status INTEGER NOT NULL,
                display_name TEXT NOT NULL
            )",
            [],
        )?;
        transaction.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_library_paths_path
             ON library_paths(path)",
            [],
        )?;

        transaction.execute(
            "CREATE TABLE IF NOT EXISTS library_items (
                key TEXT PRIMARY KEY,
                library_path_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                file_hash TEXT NOT NULL DEFAULT '',
                title TEXT,
                artist TEXT,
                album TEXT,
                year INTEGER,
                genre TEXT,
                track_number INTEGER,
                lyrics TEXT,
                FOREIGN KEY (library_path_id) REFERENCES library_paths (id)
            )",
            [],
        )?;

        transaction.execute(
            "CREATE TABLE IF NOT EXISTS pictures (
                id INTEGER PRIMARY KEY,
                library_item_id TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                picture_type INTEGER NOT NULL,
                description TEXT NOT NULL,
                file_path TEXT NOT NULL,
                FOREIGN KEY (library_item_id) REFERENCES library_items (key)
            )",
            [],
        )?;

        transaction.execute(
            "CREATE INDEX IF NOT EXISTS idx_pictures_lib_id ON pictures(library_item_id)",
            [],
        )?;

        transaction.execute(
            "CREATE TABLE IF NOT EXISTS playlists (
                id INTEGER PRIMARY KEY,
                name TEXT,
                description TEXT,
                curator TEXT,
                booklet TEXT,
                cover_mime_type TEXT,
                cover_data BLOB,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        transaction.execute(
            "CREATE TABLE IF NOT EXISTS playlist_items (
                id INTEGER PRIMARY KEY,
                playlist_id INTEGER NOT NULL,
                library_item_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                curator_note TEXT,
                FOREIGN KEY (playlist_id) REFERENCES playlists (id),
                FOREIGN KEY (library_item_id) REFERENCES library_items (key)
            )",
            [],
        )?;

        transaction.execute("DELETE FROM schema_version", [])?;
        transaction.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            rusqlite::params![Self::SCHEMA_VERSION],
        )?;

        transaction.commit()
    }

    fn table_exists(connection: &Connection, table: &str) -> Result<bool> {
        connection.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
            )",
            [table],
            |row| row.get(0),
        )
    }

    fn table_columns(connection: &Connection, table: &str) -> Result<Vec<String>> {
        let mut stmt = connection.prepare(&format!("PRAGMA table_info({table})"))?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        rows.collect()
    }

    fn unsupported_schema_error(version: i32) -> Error {
        Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::SchemaChanged,
                extended_code: 0,
            },
            Some(format!(
                "Unsupported Bird Player database schema {version}; expected {}. \
                 The database was left unchanged.",
                Self::SCHEMA_VERSION
            )),
        )
    }

    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.connection.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::Database;
    use rusqlite::Connection;

    #[test]
    fn newer_schema_is_refused_without_rebuild() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute("CREATE TABLE schema_version (version INTEGER NOT NULL)", [])
            .unwrap();
        connection
            .execute("INSERT INTO schema_version (version) VALUES (9)", [])
            .unwrap();
        connection
            .execute("CREATE TABLE future_data (value TEXT NOT NULL)", [])
            .unwrap();
        connection
            .execute("INSERT INTO future_data (value) VALUES ('keep me')", [])
            .unwrap();

        assert!(Database::initialize_schema(&connection).is_err());

        let value: String = connection
            .query_row("SELECT value FROM future_data", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "keep me");
    }

    #[test]
    fn unknown_legacy_schema_is_refused_without_rebuild() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute("CREATE TABLE schema_version (version INTEGER NOT NULL)", [])
            .unwrap();
        connection
            .execute("INSERT INTO schema_version (version) VALUES (3)", [])
            .unwrap();
        connection
            .execute("CREATE TABLE library_items (sentinel TEXT NOT NULL)", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO library_items (sentinel) VALUES ('keep me')",
                [],
            )
            .unwrap();

        assert!(Database::initialize_schema(&connection).is_err());

        let value: String = connection
            .query_row("SELECT sentinel FROM library_items", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "keep me");
    }

    #[test]
    fn fresh_database_creates_current_schema() {
        let connection = Connection::open_in_memory().unwrap();

        Database::initialize_schema(&connection).unwrap();

        let version: i32 = connection
            .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, Database::SCHEMA_VERSION);
        assert!(Database::table_exists(&connection, "library_items").unwrap());
        assert!(Database::table_exists(&connection, "playlists").unwrap());

        let playlist_columns = {
            let mut stmt = connection.prepare("PRAGMA table_info(playlists)").unwrap();
            stmt.query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        };
        assert!(playlist_columns.contains(&"booklet".to_string()));
        assert!(playlist_columns.contains(&"cover_data".to_string()));
    }

    #[test]
    fn version_six_migration_preserves_playlist_rows_and_adds_booklet_fields() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute("CREATE TABLE schema_version (version INTEGER NOT NULL)", [])
            .unwrap();
        connection
            .execute("INSERT INTO schema_version (version) VALUES (6)", [])
            .unwrap();
        connection
            .execute(
                "CREATE TABLE library_paths (
                    id INTEGER PRIMARY KEY,
                    path TEXT NOT NULL,
                    status INTEGER NOT NULL,
                    display_name TEXT NOT NULL
                )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "CREATE TABLE library_items (
                    key TEXT PRIMARY KEY,
                    library_path_id INTEGER NOT NULL
                )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "CREATE TABLE playlists (
                    id INTEGER PRIMARY KEY,
                    name TEXT,
                    description TEXT,
                    created_at INTEGER NOT NULL DEFAULT 0,
                    updated_at INTEGER NOT NULL DEFAULT 0,
                    sort_order INTEGER NOT NULL DEFAULT 0
                )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "CREATE TABLE playlist_items (
                    id INTEGER PRIMARY KEY,
                    playlist_id INTEGER NOT NULL,
                    library_item_id TEXT NOT NULL,
                    position INTEGER NOT NULL
                )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO playlists (id, name, description, created_at, updated_at, sort_order)
                 VALUES (1, 'keep me', 'notes', 11, 22, 1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO playlist_items
                 (id, playlist_id, library_item_id, position) VALUES (1, 1, 'track-key', 0)",
                [],
            )
            .unwrap();
        Database::initialize_schema(&connection).unwrap();

        let version: i32 = connection
            .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
            .unwrap();
        let name: String = connection
            .query_row("SELECT name FROM playlists WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        let track_key: String = connection
            .query_row(
                "SELECT library_item_id FROM playlist_items WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 8);
        assert_eq!(name, "keep me");
        assert_eq!(track_key, "track-key");
    }

    #[test]
    fn version_seven_migration_merges_duplicate_library_paths_and_preserves_items() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute("CREATE TABLE schema_version (version INTEGER NOT NULL)", [])
            .unwrap();
        connection
            .execute("INSERT INTO schema_version (version) VALUES (7)", [])
            .unwrap();
        connection
            .execute(
                "CREATE TABLE library_paths (
                    id INTEGER PRIMARY KEY,
                    path TEXT NOT NULL,
                    status INTEGER NOT NULL,
                    display_name TEXT NOT NULL
                )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "CREATE TABLE library_items (
                    key TEXT PRIMARY KEY,
                    library_path_id INTEGER NOT NULL,
                    FOREIGN KEY (library_path_id) REFERENCES library_paths (id)
                )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO library_paths (id, path, status, display_name)
                 VALUES (10, '/music/duplicate', 1, 'duplicate'),
                        (20, '/music/duplicate', 1, 'duplicate')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO library_items (key, library_path_id) VALUES ('track-key', 20)",
                [],
            )
            .unwrap();
        connection
            .execute_batch(
                "PRAGMA foreign_keys = OFF;
                 INSERT INTO library_items (key, library_path_id)
                 VALUES ('legacy-orphan', 99);
                 PRAGMA foreign_keys = ON;",
            )
            .unwrap();

        Database::initialize_schema(&connection).unwrap();

        let version: i32 = connection
            .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
            .unwrap();
        let path_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM library_paths WHERE path = '/music/duplicate'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let item_path_id: i64 = connection
            .query_row(
                "SELECT library_path_id FROM library_items WHERE key = 'track-key'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let status: i64 = connection
            .query_row(
                "SELECT status FROM library_paths WHERE path = '/music/duplicate'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let orphan_path_id: i64 = connection
            .query_row(
                "SELECT library_path_id FROM library_items WHERE key = 'legacy-orphan'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(version, 8);
        assert_eq!(path_count, 1);
        assert_eq!(item_path_id, 10);
        assert_eq!(orphan_path_id, 99);
        assert_eq!(status, 0);
        assert!(connection
            .execute(
                "INSERT INTO library_paths (id, path, status, display_name)
                 VALUES (30, '/music/duplicate', 1, 'duplicate')",
                [],
            )
            .is_err());
    }

    #[test]
    fn database_backup_is_consistent() {
        let test_dir =
            std::env::temp_dir().join(format!("bird-player-db-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_dir).unwrap();
        let db_path = test_dir.join("bird-player.db");
        let connection = Connection::open(&db_path).unwrap();
        connection
            .execute("CREATE TABLE schema_version (version INTEGER NOT NULL)", [])
            .unwrap();
        connection
            .execute("INSERT INTO schema_version (version) VALUES (8)", [])
            .unwrap();
        connection
            .execute("CREATE TABLE sentinel (value TEXT NOT NULL)", [])
            .unwrap();
        connection
            .execute("INSERT INTO sentinel (value) VALUES ('keep me')", [])
            .unwrap();

        let backup_path = Database::backup_database(&connection, &db_path).unwrap();
        let backup = Connection::open(backup_path).unwrap();
        let value: String = backup
            .query_row("SELECT value FROM sentinel", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "keep me");

        drop(backup);
        drop(connection);
        std::fs::remove_dir_all(test_dir).unwrap();
    }

    #[test]
    fn production_data_requires_explicit_feature() {
        let expected = if cfg!(feature = "production-data") {
            "bird-player"
        } else {
            "bird-player-dev"
        };
        assert_eq!(crate::app::constants::CONFIG_APP_NAME, expected);
    }
}

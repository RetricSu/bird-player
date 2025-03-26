use rusqlite::{Connection, Error, ErrorCode, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

impl Database {
    // The current schema version - increment this when making schema changes
    const SCHEMA_VERSION: i32 = 2;

    pub fn new() -> Result<Self> {
        // Get the app's configuration directory
        let db_path = Self::get_database_path()?;

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

        // Initialize the database schema
        Self::initialize_schema(&connection)?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn get_database_path() -> Result<PathBuf> {
        let config_dir = confy::get_configuration_file_path("bird-player", None)
            .map_err(|_| rusqlite::Error::ExecuteReturnedResults)?
            .parent()
            .ok_or(rusqlite::Error::ExecuteReturnedResults)?
            .to_path_buf();

        Ok(config_dir.join("bird-player.db"))
    }

    fn initialize_schema(connection: &Connection) -> Result<()> {
        // Create schema_version table first if it doesn't exist
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

        // If schema version is current, no need to rebuild
        if current_version == Self::SCHEMA_VERSION {
            return Ok(());
        }

        // Drop existing tables if they exist to reset the schema
        Self::drop_tables_if_exist(connection)?;

        // Create the library_paths table
        connection.execute(
            "CREATE TABLE IF NOT EXISTS library_paths (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL,
                status INTEGER NOT NULL,
                display_name TEXT NOT NULL
            )",
            [],
        )?;

        // Create the library_items table
        connection.execute(
            "CREATE TABLE IF NOT EXISTS library_items (
                key TEXT PRIMARY KEY,
                library_path_id INTEGER NOT NULL,
                path TEXT NOT NULL,
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

        // Create the pictures table
        connection.execute(
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

        // Create the playlists table
        connection.execute(
            "CREATE TABLE IF NOT EXISTS playlists (
                id INTEGER PRIMARY KEY,
                name TEXT
            )",
            [],
        )?;

        // Create the playlist_items table (mapping tracks to playlists)
        connection.execute(
            "CREATE TABLE IF NOT EXISTS playlist_items (
                id INTEGER PRIMARY KEY,
                playlist_id INTEGER NOT NULL,
                library_item_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                FOREIGN KEY (playlist_id) REFERENCES playlists (id),
                FOREIGN KEY (library_item_id) REFERENCES library_items (key)
            )",
            [],
        )?;

        // Update schema version
        connection.execute("DELETE FROM schema_version", [])?;
        connection.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            rusqlite::params![Self::SCHEMA_VERSION],
        )?;

        Ok(())
    }

    fn drop_tables_if_exist(connection: &Connection) -> Result<()> {
        // Drop tables in the reverse order of their dependency
        let tables = [
            "playlist_items",
            "playlists",
            "pictures",
            "library_items",
            "library_paths",
        ];

        for table in &tables {
            connection.execute(&format!("DROP TABLE IF EXISTS {}", table), [])?;
        }

        Ok(())
    }

    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.connection.clone()
    }
}

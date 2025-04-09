use rusqlite::{Connection, Error, ErrorCode, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};

pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

// New enum for database operations
pub enum DbOperation {
    Execute {
        sql: String,
        params: Vec<Box<dyn rusqlite::ToSql + Send>>,
        response_tx: Sender<Result<usize>>,
    },
    Transaction {
        operations: Vec<(String, Vec<Box<dyn rusqlite::ToSql + Send>>)>,
        response_tx: Sender<Result<()>>,
    },
    LoadAsync {
        loader: Box<dyn FnOnce(&Connection) -> rusqlite::Result<()> + Send>,
        response_tx: Sender<Result<()>>,
    },
}

// New worker for background database operations
#[derive(Clone)]
pub struct DbWorker {
    operation_tx: Sender<DbOperation>,
}

impl DbWorker {
    pub fn new(db_connection: Arc<Mutex<Connection>>) -> Self {
        let (operation_tx, operation_rx) = mpsc::channel::<DbOperation>();
        
        // Spawn a background thread to handle database operations
        thread::spawn(move || {
            Self::worker_thread(db_connection, operation_rx);
        });
        
        Self { operation_tx }
    }
    
    fn worker_thread(db_connection: Arc<Mutex<Connection>>, operation_rx: Receiver<DbOperation>) {
        while let Ok(operation) = operation_rx.recv() {
            match operation {
                DbOperation::Execute { sql, params, response_tx } => {
                    let result = {
                        let mut conn = db_connection.lock().unwrap();
                        let mut stmt = match conn.prepare(&sql) {
                            Ok(stmt) => stmt,
                            Err(e) => {
                                let _ = response_tx.send(Err(e));
                                continue;
                            }
                        };
                        
                        // Convert params to slice of ToSql trait objects
                        let param_refs: Vec<&dyn rusqlite::ToSql> = params
                            .iter()
                            .map(|p| p.as_ref() as &dyn rusqlite::ToSql)
                            .collect();
                        
                        stmt.execute(param_refs.as_slice())
                    };
                    
                    let _ = response_tx.send(result);
                },
                DbOperation::Transaction { operations, response_tx } => {
                    let result = {
                        let mut conn = db_connection.lock().unwrap();
                        let tx = match conn.transaction() {
                            Ok(tx) => tx,
                            Err(e) => {
                                let _ = response_tx.send(Err(e));
                                continue;
                            }
                        };
                        
                        for (sql, params) in operations {
                            let mut stmt = match tx.prepare(&sql) {
                                Ok(stmt) => stmt,
                                Err(e) => {
                                    let _ = response_tx.send(Err(e));
                                    return;
                                }
                            };
                            
                            // Convert params to slice of ToSql trait objects
                            let param_refs: Vec<&dyn rusqlite::ToSql> = params
                                .iter()
                                .map(|p| p.as_ref() as &dyn rusqlite::ToSql)
                                .collect();
                            
                            if let Err(e) = stmt.execute(param_refs.as_slice()) {
                                let _ = response_tx.send(Err(e));
                                return;
                            }
                        }
                        
                        tx.commit().map(|_| ())
                    };
                    
                    let _ = response_tx.send(result);
                },
                DbOperation::LoadAsync { loader, response_tx } => {
                    let result = {
                        let conn = db_connection.lock().unwrap();
                        loader(&conn)
                    };
                    
                    let _ = response_tx.send(result);
                }
            }
        }
    }
    
    pub fn execute(&self, sql: String, params: Vec<Box<dyn rusqlite::ToSql + Send>>) -> Result<usize> {
        let (response_tx, response_rx) = mpsc::channel();
        
        self.operation_tx.send(DbOperation::Execute {
            sql,
            params,
            response_tx,
        }).map_err(|_| Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::CannotOpen,
                extended_code: 0,
            },
            Some("Failed to send database operation to worker thread".to_string()),
        ))?;
        
        response_rx.recv().map_err(|_| Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::CannotOpen,
                extended_code: 0,
            },
            Some("Failed to receive response from worker thread".to_string()),
        ))?
    }
    
    pub fn transaction(&self, operations: Vec<(String, Vec<Box<dyn rusqlite::ToSql + Send>>)>) -> Result<()> {
        let (response_tx, response_rx) = mpsc::channel();
        
        self.operation_tx.send(DbOperation::Transaction {
            operations,
            response_tx,
        }).map_err(|_| Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::CannotOpen,
                extended_code: 0,
            },
            Some("Failed to send transaction operation to worker thread".to_string()),
        ))?;
        
        response_rx.recv().map_err(|_| Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::CannotOpen,
                extended_code: 0,
            },
            Some("Failed to receive response from worker thread".to_string()),
        ))?
    }
    
    // Add a method to load data asynchronously
    pub fn load_async<F, T>(&self, loader_fn: F) -> Result<()> 
    where
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static
    {
        // Create channel for result
        let (completion_tx, _) = mpsc::channel::<T>();
        
        // Create a new operation type for this
        let (response_tx, response_rx) = mpsc::channel();
        
        self.operation_tx.send(DbOperation::LoadAsync {
            loader: Box::new(move |conn| {
                match loader_fn(conn) {
                    Ok(result) => {
                        // Send result to calling thread
                        let _ = completion_tx.send(result);
                        Ok(())
                    }
                    Err(e) => Err(e)
                }
            }),
            response_tx,
        }).map_err(|_| Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::CannotOpen,
                extended_code: 0,
            },
            Some("Failed to send load async operation to worker thread".to_string()),
        ))?;
        
        // Wait for operation to start but don't wait for completion
        response_rx.recv().map_err(|_| Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::CannotOpen,
                extended_code: 0,
            },
            Some("Failed to receive response from worker thread".to_string()),
        ))?
    }
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
    
    // Create a worker to handle background operations
    pub fn create_worker(&self) -> DbWorker {
        DbWorker::new(self.connection.clone())
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

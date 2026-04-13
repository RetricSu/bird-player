# Database Implementation Issues

This document outlines the current architectural constraints and performance bottlenecks in the SQLite database implementation of Bird Player. As the library grows to thousands of tracks, these issues must be addressed to ensure scalability and a smooth user experience.

## 1. Full-Replacement Write Pattern
Currently, whenever the application exits and triggers `save_state`, the `Library::save_to_db` and `Playlist::save_to_db_and_update_id` methods dump the entire memory state back to the database.
- It iterates through the entire library, performing `INSERT OR REPLACE INTO library_items` for EVERY track.
- For every track, it executes `DELETE FROM pictures WHERE library_item_id = ?` and then re-inserts all associated pictures.
- **Impact**: Severe disk I/O inflation. For a library of 10,000 songs, quitting the app will cause tens of thousands of write operations, freezing the app on exit and reducing SSD lifespan. **Solution**: Implement dirty flags (`is_dirty`) so only modified tracks are written, or apply updates incrementally as changes happen.

## 2. N+1 Queries and Missing Indexes (Slow Startup)
When loading the library via `Library::load_from_db`, the application fetches album art via an N+1 query pattern:
```rust
for item in &mut items {
    let mut pic_stmt = conn_guard.prepare("SELECT ... FROM pictures WHERE library_item_id = ?")?;
    // ...
}
```
- **The Issue**: There is NO index on `pictures.library_item_id`. This means that for 5,000 tracks, the app executes 5,000 separate `SELECT` queries, and each one performs a full table scan on the `pictures` table.
- **Impact**: Application startup time will increase exponentially as the library grows.
- **Solution**: 
  1. Add an index: `CREATE INDEX idx_pictures_lib_id ON pictures(library_item_id);`
  2. Refactor to query all pictures at once `SELECT * FROM pictures` and match them to library items in memory via a HashMap.

## 3. The "Ghost File" Bug (File Identification)
Currently, a music file's uniqueness is determined solely by its absolute file path (`item.path()`). 
- **The Issue**: If a user renames an MP3 file (e.g., `A.mp3` to `B.mp3`) or moves it to another folder, a rescanning operation will treat it as a brand-new track and assign it a new database `key`. Furthermore, the resync logic never removes paths that no longer exist on the filesystem.
- **Impact**: The old track entry (`A.mp3`) remains in the database as a "ghost track". Clicking it fails. Any playlist containing the old track becomes corrupted since the playlist item points to the dead `key`.
- **Solution**: Do not rely on absolute paths for identification.
  - **Inode / File ID**: Read the OS-level Inode (Unix) or File ID (Windows). This number remains unchanged even if the file is renamed or moved within the same drive partitioning.
  - **Heuristics / Hashing**: Use a combination of `file size + creation time` or perform a partial hash (e.g., first and last 128KB of the audio file) as a stable fingerprint.

## 4. Primary Key Generation and Type Mismatches
- **The Issue**: `LibraryItem::key()` is generated in-memory using `rand::thread_rng().gen::<usize>()`. It is stored as `TEXT` in the database.
- However, when querying the `pictures` table, the code parses the key as a 64-bit integer:
  ```rust
  let item_key = item.key() as i64; // Type cast
  pic_stmt.query_map(rusqlite::params![item_key], ...);
  ```
- **Impact**: Relying on SQLite's implicit type conversion (from `TEXT` to `i64`) can cause unexpected query failures, index misses, and potential panics if a generated `usize` exceeds `i64::MAX`.
- **Solution**: Use standardized unique identifiers (such as ULIDs or UUIDs), store them strictly as `TEXT`, and query them as strings.

## 5. Destructive Schema Migrations
- **The Issue**: The initialization logic compares `current_version` with `SCHEMA_VERSION`. If they don't match, it executes `drop_tables_if_exist()`.
- **Impact**: Any future update that increments the schema version will permanently and silently delete all user playlists, play records, and library data during application startup.
- **Solution**: Implement a proper migration system (using `ALTER TABLE` statements or via a migration crate like `refinery` or `rusqlite_migration`) to evolve the database schema safely without data loss.

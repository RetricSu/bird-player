#!/bin/bash
set -euo pipefail

APP_NAME="Bird Player"
REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILT_APP="$REPO_DIR/target/release/bundle/osx/$APP_NAME.app"
INSTALLED_APP="/Applications/$APP_NAME.app"
DATA_DIR="$HOME/Library/Application Support/rs.bird-player"
DATABASE="$DATA_DIR/bird-player.db"
BACKUP_DIR="$DATA_DIR/install-backups"
STAMP="$(date '+%Y%m%d-%H%M%S')"
APP_BACKUP="$BACKUP_DIR/$APP_NAME-$STAMP.app.backup"
DB_BACKUP="$BACKUP_DIR/bird-player-$STAMP.db"
FAILED_APP="$BACKUP_DIR/$APP_NAME-failed-$STAMP.app.failed"
BEFORE_LIBRARY_ITEMS=0
BEFORE_PLAYLISTS=0
EXPECTED_SCHEMA_VERSION="$(sed -nE \
    's/^[[:space:]]*const SCHEMA_VERSION: i32 = ([0-9]+);/\1/p' \
    "$REPO_DIR/src/app/db.rs" | head -1)"

if ! [[ "$EXPECTED_SCHEMA_VERSION" =~ ^[0-9]+$ ]]; then
    echo "Could not determine the expected database schema version." >&2
    exit 1
fi

restore_previous_install() {
    osascript -e 'tell application "Bird Player" to quit' >/dev/null 2>&1 || true
    if [ -d "$INSTALLED_APP" ]; then
        mv "$INSTALLED_APP" "$FAILED_APP"
    fi
    if [ -d "$APP_BACKUP" ]; then
        mv "$APP_BACKUP" "$INSTALLED_APP"
    fi
    if [ -f "$DB_BACKUP" ]; then
        chmod u+w "$DB_BACKUP"
        sqlite3 "$DB_BACKUP" ".backup '$DATABASE'"
        chmod a-w "$DB_BACKUP"
    fi
}

cd "$REPO_DIR"

echo "Building production bundle..."
cargo bundle --release --features production-data
codesign --force --deep --sign - "$BUILT_APP" >/dev/null
codesign --verify --deep --strict "$BUILT_APP"

echo "Closing Bird Player..."
osascript -e 'tell application "Bird Player" to quit' >/dev/null 2>&1 || true
for _ in $(seq 1 40); do
    if ! pgrep -f '/Applications/Bird Player.app/Contents/MacOS/bird-player' >/dev/null; then
        break
    fi
    sleep 0.25
done
if pgrep -f '/Applications/Bird Player.app/Contents/MacOS/bird-player' >/dev/null; then
    echo "Bird Player did not quit cleanly; installation stopped." >&2
    exit 1
fi

mkdir -p "$BACKUP_DIR"

if [ -f "$DATABASE" ]; then
    echo "Backing up the production database..."
    sqlite3 "$DATABASE" ".backup '$DB_BACKUP'"
    if [ "$(sqlite3 "$DB_BACKUP" 'PRAGMA integrity_check;')" != "ok" ]; then
        echo "Database backup failed its integrity check; installation stopped." >&2
        exit 1
    fi
    chmod a-w "$DB_BACKUP"
    BEFORE_LIBRARY_ITEMS="$(sqlite3 "$DATABASE" 'SELECT COUNT(*) FROM library_items;' 2>/dev/null || printf '0')"
    BEFORE_PLAYLISTS="$(sqlite3 "$DATABASE" 'SELECT COUNT(*) FROM playlists;' 2>/dev/null || printf '0')"
fi

if [ -d "$INSTALLED_APP" ]; then
    echo "Preserving the currently installed app..."
    mv "$INSTALLED_APP" "$APP_BACKUP"
fi

echo "Installing the new app..."
if ! ditto "$BUILT_APP" "$INSTALLED_APP"; then
    restore_previous_install
    echo "Installation failed; the previous app was restored." >&2
    exit 1
fi

if ! codesign --verify --deep --strict "$INSTALLED_APP"; then
    restore_previous_install
    echo "Installed app signature verification failed; the previous app was restored." >&2
    exit 1
fi
if ! open "$INSTALLED_APP"; then
    restore_previous_install
    echo "The new app could not be opened; the previous app was restored." >&2
    exit 1
fi
sleep 3

if ! pgrep -f '/Applications/Bird Player.app/Contents/MacOS/bird-player' >/dev/null; then
    restore_previous_install
    echo "The new app exited during startup; the previous app and database were restored." >&2
    exit 1
fi

if [ -f "$DATABASE" ] && [ "$(sqlite3 "$DATABASE" 'PRAGMA integrity_check;')" != "ok" ]; then
    restore_previous_install
    echo "Post-install database verification failed; the previous app and database were restored." >&2
    exit 1
fi

if [ -f "$DATABASE" ]; then
    AFTER_SCHEMA_VERSION="$(sqlite3 "$DATABASE" \
        'SELECT version FROM schema_version LIMIT 1;' 2>/dev/null || true)"
    if [ "$AFTER_SCHEMA_VERSION" != "$EXPECTED_SCHEMA_VERSION" ]; then
        restore_previous_install
        echo "The new app did not finish database startup; the previous app and database were restored." >&2
        exit 1
    fi

    AFTER_LIBRARY_ITEMS="$(sqlite3 "$DATABASE" 'SELECT COUNT(*) FROM library_items;' 2>/dev/null || printf '0')"
    AFTER_PLAYLISTS="$(sqlite3 "$DATABASE" 'SELECT COUNT(*) FROM playlists;' 2>/dev/null || printf '0')"
    if { [ "$BEFORE_LIBRARY_ITEMS" -gt 0 ] && [ "$AFTER_LIBRARY_ITEMS" -eq 0 ]; } ||
        { [ "$BEFORE_PLAYLISTS" -gt 1 ] && [ "$AFTER_PLAYLISTS" -le 1 ]; }; then
        restore_previous_install
        echo "Post-install data counts dropped catastrophically; the previous app and database were restored." >&2
        exit 1
    fi
fi

echo "Installed $INSTALLED_APP"
echo "Backups are in $BACKUP_DIR"

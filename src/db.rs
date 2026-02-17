use rusqlite::{Connection, Result, params};
use std::sync::Mutex;
use tracing::info;

use crate::models::{Camera, Mosaic, MosaicCamera, MosaicWithCameras, User, UserPublic, Capture, Notification, Role, Permission, RoleWithPermissions, PermissionInput, MosaicShare};

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn: Mutex::new(conn) };
        db.init_tables()?;
        info!("Base de datos SQLite inicializada en {}", path);
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS locations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT DEFAULT '',
                is_system INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS areas (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                location_id INTEGER NOT NULL,
                description TEXT DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (location_id) REFERENCES locations(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS cameras (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                host TEXT NOT NULL,
                port INTEGER NOT NULL DEFAULT 554,
                username TEXT DEFAULT '',
                password TEXT DEFAULT '',
                path TEXT NOT NULL DEFAULT '/defaultPrimary?streamType=m',
                protocol TEXT NOT NULL DEFAULT 'rtsp',
                enabled INTEGER NOT NULL DEFAULT 1,
                record INTEGER NOT NULL DEFAULT 1,
                source_on_demand INTEGER NOT NULL DEFAULT 1,
                location TEXT DEFAULT '',
                area TEXT DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS mosaics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                layout TEXT NOT NULL DEFAULT '2x2',
                active INTEGER NOT NULL DEFAULT 0,
                pid INTEGER,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS mosaic_cameras (
                mosaic_id INTEGER NOT NULL,
                camera_id INTEGER NOT NULL,
                position INTEGER NOT NULL,
                PRIMARY KEY (mosaic_id, camera_id),
                FOREIGN KEY (mosaic_id) REFERENCES mosaics(id) ON DELETE CASCADE,
                FOREIGN KEY (camera_id) REFERENCES cameras(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                email TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'viewer',
                active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS password_reset_tokens (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                token TEXT NOT NULL UNIQUE,
                expires_at TEXT NOT NULL,
                used INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS captures (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                camera_id INTEGER NOT NULL,
                camera_name TEXT NOT NULL,
                capture_type TEXT NOT NULL DEFAULT 'screenshot',
                file_path TEXT NOT NULL,
                file_size INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (camera_id) REFERENCES cameras(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS notifications (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                category TEXT NOT NULL DEFAULT 'system',
                title TEXT NOT NULL,
                message TEXT NOT NULL DEFAULT '',
                severity TEXT NOT NULL DEFAULT 'info',
                read INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS roles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT DEFAULT '',
                is_system INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS role_permissions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                role_id INTEGER NOT NULL,
                module TEXT NOT NULL,
                can_view INTEGER NOT NULL DEFAULT 0,
                can_create INTEGER NOT NULL DEFAULT 0,
                can_edit INTEGER NOT NULL DEFAULT 0,
                can_delete INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
                UNIQUE(role_id, module)
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS mosaic_shares (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mosaic_id INTEGER NOT NULL,
                mosaic_name TEXT NOT NULL,
                token TEXT NOT NULL UNIQUE,
                emails TEXT NOT NULL DEFAULT '',
                expires_at TEXT NOT NULL,
                schedule_start TEXT,
                schedule_end TEXT,
                active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (mosaic_id) REFERENCES mosaics(id) ON DELETE CASCADE
            );"
        )?;
        
        // Run migrations to add new columns without losing data
        // Pass the already-locked connection to avoid Mutex deadlock
        Self::run_migrations(&conn)?;
        
        Ok(())
    }

    fn run_migrations(conn: &Connection) -> Result<()> {
        
        // Check if is_system column exists in locations table
        let column_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('locations') WHERE name='is_system'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0) > 0;
        
        if !column_exists {
            // Add is_system column with default value 0 (false)
            conn.execute(
                "ALTER TABLE locations ADD COLUMN is_system INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
            info!("Migration: Added is_system column to locations table");
        }

        // Check if location column exists in cameras table
        let location_col_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('cameras') WHERE name='location'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0) > 0;

        if !location_col_exists {
            conn.execute(
                "ALTER TABLE cameras ADD COLUMN location TEXT DEFAULT ''",
                [],
            )?;
            info!("Migration: Added location column to cameras table");
        }

        // Check if area column exists in cameras table
        let area_col_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('cameras') WHERE name='area'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0) > 0;

        if !area_col_exists {
            conn.execute(
                "ALTER TABLE cameras ADD COLUMN area TEXT DEFAULT ''",
                [],
            )?;
            info!("Migration: Added area column to cameras table");
        }
        
        Ok(())
    }

    // =========================================================================
    // Camera CRUD
    // =========================================================================

    pub fn list_cameras(&self, search: Option<&str>) -> Result<Vec<Camera>> {
        let conn = self.conn.lock().unwrap();
        let mut cameras = Vec::new();

        if let Some(q) = search {
            let like = format!("%{}%", q);
            let mut stmt = conn.prepare(
                "SELECT id, name, host, port, username, password, path, protocol, enabled, record, source_on_demand, location, area, created_at, updated_at 
                 FROM cameras WHERE name LIKE ?1 OR host LIKE ?1 OR location LIKE ?1 OR area LIKE ?1 ORDER BY name"
            )?;
            let rows = stmt.query_map(params![like], |row| {
                Ok(Camera {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    host: row.get(2)?,
                    port: row.get(3)?,
                    username: row.get(4)?,
                    password: row.get(5)?,
                    path: row.get(6)?,
                    protocol: row.get(7)?,
                    enabled: row.get(8)?,
                    record: row.get(9)?,
                    source_on_demand: row.get(10)?,
                    location: row.get(11)?,
                    area: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            })?;
            for row in rows {
                cameras.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, name, host, port, username, password, path, protocol, enabled, record, source_on_demand, location, area, created_at, updated_at 
                 FROM cameras ORDER BY name"
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(Camera {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    host: row.get(2)?,
                    port: row.get(3)?,
                    username: row.get(4)?,
                    password: row.get(5)?,
                    path: row.get(6)?,
                    protocol: row.get(7)?,
                    enabled: row.get(8)?,
                    record: row.get(9)?,
                    source_on_demand: row.get(10)?,
                    location: row.get(11)?,
                    area: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            })?;
            for row in rows {
                cameras.push(row?);
            }
        }
        Ok(cameras)
    }

    pub fn get_camera(&self, id: i64) -> Result<Option<Camera>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, host, port, username, password, path, protocol, enabled, record, source_on_demand, location, area, created_at, updated_at 
             FROM cameras WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Camera {
                id: row.get(0)?,
                name: row.get(1)?,
                host: row.get(2)?,
                port: row.get(3)?,
                username: row.get(4)?,
                password: row.get(5)?,
                path: row.get(6)?,
                protocol: row.get(7)?,
                enabled: row.get(8)?,
                record: row.get(9)?,
                source_on_demand: row.get(10)?,
                location: row.get(11)?,
                area: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn create_camera(&self, cam: &Camera) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO cameras (name, host, port, username, password, path, protocol, enabled, record, source_on_demand, location, area) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![cam.name, cam.host, cam.port, cam.username, cam.password, cam.path, cam.protocol, cam.enabled, cam.record, cam.source_on_demand, cam.location, cam.area],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_camera(&self, id: i64, cam: &Camera) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE cameras SET name=?1, host=?2, port=?3, username=?4, password=?5, path=?6, protocol=?7, enabled=?8, record=?9, source_on_demand=?10, location=?11, area=?12, updated_at=datetime('now')
             WHERE id=?13",
            params![cam.name, cam.host, cam.port, cam.username, cam.password, cam.path, cam.protocol, cam.enabled, cam.record, cam.source_on_demand, cam.location, cam.area, id],
        )?;
        Ok(rows > 0)
    }

    pub fn delete_camera(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM cameras WHERE id=?1", params![id])?;
        Ok(rows > 0)
    }

    // =========================================================================
    // Mosaic CRUD
    // =========================================================================

    pub fn list_mosaics(&self) -> Result<Vec<MosaicWithCameras>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, layout, active, pid, created_at FROM mosaics ORDER BY name"
        )?;
        let mosaics: Vec<Mosaic> = stmt.query_map([], |row| {
            Ok(Mosaic {
                id: row.get(0)?,
                name: row.get(1)?,
                layout: row.get(2)?,
                active: row.get(3)?,
                pid: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        let mut result = Vec::new();
        for mosaic in mosaics {
            let mut cam_stmt = conn.prepare(
                "SELECT mc.position, c.id, c.name, c.host, c.port, c.username, c.password, c.path, c.protocol
                 FROM mosaic_cameras mc JOIN cameras c ON mc.camera_id = c.id
                 WHERE mc.mosaic_id = ?1 ORDER BY mc.position"
            )?;
            let cameras: Vec<MosaicCamera> = cam_stmt.query_map(params![mosaic.id], |row| {
                Ok(MosaicCamera {
                    position: row.get(0)?,
                    camera_id: row.get(1)?,
                    camera_name: row.get(2)?,
                    host: row.get(3)?,
                    port: row.get(4)?,
                    username: row.get(5)?,
                    password: row.get(6)?,
                    path: row.get(7)?,
                    protocol: row.get(8)?,
                })
            })?.filter_map(|r| r.ok()).collect();

            result.push(MosaicWithCameras { mosaic, cameras });
        }
        Ok(result)
    }

    pub fn get_mosaic(&self, id: i64) -> Result<Option<MosaicWithCameras>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, layout, active, pid, created_at FROM mosaics WHERE id=?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Mosaic {
                id: row.get(0)?,
                name: row.get(1)?,
                layout: row.get(2)?,
                active: row.get(3)?,
                pid: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        match rows.next() {
            Some(Ok(mosaic)) => {
                drop(rows);
                drop(stmt);

                let mut cam_stmt = conn.prepare(
                    "SELECT mc.position, c.id, c.name, c.host, c.port, c.username, c.password, c.path, c.protocol
                     FROM mosaic_cameras mc JOIN cameras c ON mc.camera_id = c.id
                     WHERE mc.mosaic_id = ?1 ORDER BY mc.position"
                )?;
                let cameras: Vec<MosaicCamera> = cam_stmt.query_map(params![mosaic.id], |row| {
                    Ok(MosaicCamera {
                        position: row.get(0)?,
                        camera_id: row.get(1)?,
                        camera_name: row.get(2)?,
                        host: row.get(3)?,
                        port: row.get(4)?,
                        username: row.get(5)?,
                        password: row.get(6)?,
                        path: row.get(7)?,
                        protocol: row.get(8)?,
                    })
                })?.filter_map(|r| r.ok()).collect();

                Ok(Some(MosaicWithCameras { mosaic, cameras }))
            }
            _ => Ok(None),
        }
    }

    pub fn create_mosaic(&self, name: &str, layout: &str, camera_ids: &[i64]) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO mosaics (name, layout) VALUES (?1, ?2)",
            params![name, layout],
        )?;
        let mosaic_id = conn.last_insert_rowid();
        for (pos, cam_id) in camera_ids.iter().enumerate() {
            conn.execute(
                "INSERT INTO mosaic_cameras (mosaic_id, camera_id, position) VALUES (?1, ?2, ?3)",
                params![mosaic_id, cam_id, pos as i64],
            )?;
        }
        Ok(mosaic_id)
    }

    pub fn update_mosaic(&self, id: i64, name: &str, layout: &str, camera_ids: &[i64]) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE mosaics SET name=?1, layout=?2 WHERE id=?3",
            params![name, layout, id],
        )?;
        if rows == 0 { return Ok(false); }
        conn.execute("DELETE FROM mosaic_cameras WHERE mosaic_id=?1", params![id])?;
        for (pos, cam_id) in camera_ids.iter().enumerate() {
            conn.execute(
                "INSERT INTO mosaic_cameras (mosaic_id, camera_id, position) VALUES (?1, ?2, ?3)",
                params![id, cam_id, pos as i64],
            )?;
        }
        Ok(true)
    }

    pub fn delete_mosaic(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM mosaics WHERE id=?1", params![id])?;
        Ok(rows > 0)
    }

    pub fn set_mosaic_active(&self, id: i64, active: bool, pid: Option<u32>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE mosaics SET active=?1, pid=?2 WHERE id=?3",
            params![active as i32, pid.map(|p| p as i64), id],
        )?;
        Ok(())
    }

    /// Limpia todos los mosaicos activos al iniciar (procesos FFmpeg huérfanos)
    pub fn cleanup_orphaned_mosaics(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "UPDATE mosaics SET active=0, pid=NULL WHERE active=1",
            [],
        )?;
        Ok(count)
    }

    // =========================================================================
    // Seed existing cameras from mediamtx.yml
    // =========================================================================
    pub fn seed_if_empty(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM cameras", [], |row| row.get(0))?;
        if count > 0 {
            info!("DB ya tiene {} cámaras, no se hace seed", count);
            return Ok(());
        }
        drop(conn);

        info!("Seeding DB con cámaras existentes de mediamtx.yml...");
        let cameras = vec![
            ("entrance-guardhouse-bottom", "10.0.0.30", 554, "root", "dynamics8249", "/axis-media/media.amp", "rtsp"),
            ("entrance-guardhouse-top", "10.0.0.30", 554, "root", "dynamics8249", "/axis-media/media.amp", "rtsp"),
            ("entrance-guardhouse-right-corner", "10.0.0.45", 554, "zeus", "zeus", "/defaultPrimary?mtu=1440&streamType=m", "rtsp"),
            ("entrance-guardhouse-left-corner", "10.0.0.46", 554, "zeus", "zeus", "/defaultPrimary?mtu=1440&streamType=m", "rtsp"),
            ("entrance-warehouse-office-entry-corner", "10.0.0.33", 554, "root", "dynamics8249", "/axis-media/media.amp", "rtsp"),
            ("main-entrance-first-camera", "10.0.0.34", 554, "root", "dynamics8249", "/axis-media/media.amp", "rtsp"),
            ("enrtre-warehouse-sar-corner-1", "10.0.0.222", 554, "zeus", "zeus", "/rtsp/defaultPrimary-0?streamType=m", "rtsp"),
            ("enrtre-warehouse-sar-corner-2", "10.0.0.222", 554, "zeus", "zeus", "/rtsp/defaultPrimary-1?streamType=m", "rtsp"),
            ("enrtre-warehouse-sar-corner-3", "10.0.0.222", 554, "zeus", "zeus", "/rtsp/defaultPrimary-2?streamType=m", "rtsp"),
            ("verificaction-camera", "10.1.7.106", 554, "admin", "carmi2025.", "", "rtsp"),
            ("W1-South-East-Yard-Corner", "10.0.0.123", 554, "zeus", "zeus", "/rtsp/defaultPrimary?streamType=m", "rtsp"),
            ("W1-Door-9-11", "10.0.0.71", 554, "zeus", "zeus", "/rtsp/defaultPrimary?streamType=m", "rtsp"),
            ("W1-Door-24-22", "10.0.0.70", 554, "zeus", "zeus", "/rtsp/defaultPrimary?streamType=m", "rtsp"),
            ("W1-Door-14-17", "10.0.0.79", 554, "zeus", "zeus", "/rtsp/defaultPrimary?streamType=m", "rtsp"),
            ("W1-Door-14-12", "10.0.0.77", 554, "zeus", "zeus", "/rtsp/defaultPrimary?streamType=m", "rtsp"),
            ("W2-Door-Preview-39-49", "10.0.0.121", 554, "zeus", "zeus", "/rtsp/defaultPrimary?streamType=m", "rtsp"),
            ("W2-Door-Preview-43-To-Ramp", "10.0.0.123", 554, "zeus", "zeus", "/rtsp/defaultPrimary?streamType=m", "rtsp"),
        ];

        let conn = self.conn.lock().unwrap();
        for (name, host, port, user, pass, path, proto) in cameras {
            conn.execute(
                "INSERT OR IGNORE INTO cameras (name, host, port, username, password, path, protocol, enabled, record, source_on_demand, location, area)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 1, 0, ?8, ?9)",
                params![name, host, port, user, pass, path, proto, "Warehouse 1", "Entrance"],
            )?;
        }
        info!("Seed completado: 17 cámaras insertadas");
        Ok(())
    }

    // =========================================================================
    // Location CRUD
    // =========================================================================

    pub fn list_locations(&self) -> Result<Vec<crate::models::Location>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, description, is_system, created_at FROM locations ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::models::Location {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                is_system: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
            })
        })?;
        
        let mut locations = Vec::new();
        for row in rows {
            locations.push(row?);
        }
        Ok(locations)
    }

    pub fn create_location(&self, location: &crate::models::Location) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO locations (name, description, is_system) VALUES (?1, ?2, ?3)",
            params![location.name, location.description, if location.is_system { 1 } else { 0 }],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_location(&self, id: i64, location: &crate::models::Location) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE locations SET name=?1, description=?2, is_system=?3 WHERE id=?4",
            params![location.name, location.description, if location.is_system { 1 } else { 0 }, id],
        )?;
        Ok(rows > 0)
    }

    pub fn get_location(&self, id: i64) -> Result<Option<crate::models::Location>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, description, is_system, created_at FROM locations WHERE id=?1")?;
        let mut rows = stmt.query(params![id])?;
        
        if let Some(row) = rows.next()? {
            Ok(Some(crate::models::Location {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                is_system: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_location(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM locations WHERE id=?1", params![id])?;
        Ok(rows > 0)
    }

    // =========================================================================
    // Area CRUD
    // =========================================================================

    pub fn list_areas(&self) -> Result<Vec<crate::models::AreaWithLocation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT a.id, a.name, a.location_id, l.name as location_name, a.description, a.created_at 
             FROM areas a 
             JOIN locations l ON a.location_id = l.id 
             ORDER BY l.name, a.name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::models::AreaWithLocation {
                id: row.get(0)?,
                name: row.get(1)?,
                location_id: row.get(2)?,
                location_name: row.get(3)?,
                description: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        
        let mut areas = Vec::new();
        for row in rows {
            areas.push(row?);
        }
        Ok(areas)
    }

    pub fn list_areas_by_location(&self, location_id: i64) -> Result<Vec<crate::models::Area>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, location_id, description, created_at 
             FROM areas WHERE location_id = ?1 ORDER BY name"
        )?;
        let rows = stmt.query_map(params![location_id], |row| {
            Ok(crate::models::Area {
                id: row.get(0)?,
                name: row.get(1)?,
                location_id: row.get(2)?,
                description: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        
        let mut areas = Vec::new();
        for row in rows {
            areas.push(row?);
        }
        Ok(areas)
    }

    pub fn create_area(&self, area: &crate::models::Area) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO areas (name, location_id, description) VALUES (?1, ?2, ?3)",
            params![area.name, area.location_id, area.description],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_area(&self, id: i64, area: &crate::models::Area) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE areas SET name=?1, location_id=?2, description=?3 WHERE id=?4",
            params![area.name, area.location_id, area.description, id],
        )?;
        Ok(rows > 0)
    }

    pub fn delete_area(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM areas WHERE id=?1", params![id])?;
        Ok(rows > 0)
    }

    // =========================================================================
    // User CRUD
    // =========================================================================

    pub fn seed_admin_user(&self, password_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
        if count > 0 {
            info!("Users table already has {} users, skipping admin seed", count);
            return Ok(());
        }
        conn.execute(
            "INSERT INTO users (username, email, password_hash, role, active) VALUES (?1, ?2, ?3, ?4, 1)",
            params!["admin", "admin@example.com", password_hash, "admin"],
        )?;
        info!("Seeded default admin user (admin / admin)");
        Ok(())
    }

    pub fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, username, email, password_hash, role, active, created_at, updated_at FROM users WHERE username=?1"
        )?;
        let mut rows = stmt.query_map(params![username], |row| {
            Ok(User {
                id: row.get(0)?, username: row.get(1)?, email: row.get(2)?,
                password_hash: row.get(3)?, role: row.get(4)?, active: row.get(5)?,
                created_at: row.get(6)?, updated_at: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, username, email, password_hash, role, active, created_at, updated_at FROM users WHERE email=?1"
        )?;
        let mut rows = stmt.query_map(params![email], |row| {
            Ok(User {
                id: row.get(0)?, username: row.get(1)?, email: row.get(2)?,
                password_hash: row.get(3)?, role: row.get(4)?, active: row.get(5)?,
                created_at: row.get(6)?, updated_at: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn get_user_by_id(&self, id: i64) -> Result<Option<User>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, username, email, password_hash, role, active, created_at, updated_at FROM users WHERE id=?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(User {
                id: row.get(0)?, username: row.get(1)?, email: row.get(2)?,
                password_hash: row.get(3)?, role: row.get(4)?, active: row.get(5)?,
                created_at: row.get(6)?, updated_at: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn list_users(&self) -> Result<Vec<UserPublic>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, username, email, role, active, created_at, updated_at FROM users ORDER BY username"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(UserPublic {
                id: row.get(0)?, username: row.get(1)?, email: row.get(2)?,
                role: row.get(3)?, active: row.get(4)?, created_at: row.get(5)?, updated_at: row.get(6)?,
            })
        })?;
        let mut users = Vec::new();
        for row in rows { users.push(row?); }
        Ok(users)
    }

    pub fn create_user(&self, username: &str, email: &str, password_hash: &str, role: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO users (username, email, password_hash, role) VALUES (?1, ?2, ?3, ?4)",
            params![username, email, password_hash, role],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_user(&self, id: i64, username: &str, email: &str, password_hash: Option<&str>, role: &str, active: bool) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = if let Some(hash) = password_hash {
            conn.execute(
                "UPDATE users SET username=?1, email=?2, password_hash=?3, role=?4, active=?5, updated_at=datetime('now') WHERE id=?6",
                params![username, email, hash, role, active, id],
            )?
        } else {
            conn.execute(
                "UPDATE users SET username=?1, email=?2, role=?3, active=?4, updated_at=datetime('now') WHERE id=?5",
                params![username, email, role, active, id],
            )?
        };
        Ok(rows > 0)
    }

    pub fn delete_user(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM users WHERE id=?1", params![id])?;
        Ok(rows > 0)
    }

    pub fn update_user_password(&self, user_id: i64, password_hash: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE users SET password_hash=?1, updated_at=datetime('now') WHERE id=?2",
            params![password_hash, user_id],
        )?;
        Ok(rows > 0)
    }

    // =========================================================================
    // Password Reset Tokens
    // =========================================================================

    pub fn create_reset_token(&self, user_id: i64, token: &str, expires_at: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        // Invalidate old tokens for this user
        conn.execute("UPDATE password_reset_tokens SET used=1 WHERE user_id=?1 AND used=0", params![user_id])?;
        conn.execute(
            "INSERT INTO password_reset_tokens (user_id, token, expires_at) VALUES (?1, ?2, ?3)",
            params![user_id, token, expires_at],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn validate_reset_token(&self, token: &str) -> Result<Option<i64>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT user_id FROM password_reset_tokens WHERE token=?1 AND used=0 AND expires_at > datetime('now')"
        )?;
        let mut rows = stmt.query(params![token])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn consume_reset_token(&self, token: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE password_reset_tokens SET used=1 WHERE token=?1", params![token])?;
        Ok(())
    }

    // =========================================================================
    // Captures CRUD
    // =========================================================================

    pub fn create_capture(&self, camera_id: i64, camera_name: &str, capture_type: &str, file_path: &str, file_size: i64) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO captures (camera_id, camera_name, capture_type, file_path, file_size) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![camera_id, camera_name, capture_type, file_path, file_size],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_captures(&self, camera_id: Option<i64>, date: Option<&str>, capture_type: Option<&str>) -> Result<Vec<Capture>> {
        let conn = self.conn.lock().unwrap();
        let mut sql = String::from(
            "SELECT id, camera_id, camera_name, capture_type, file_path, file_size, created_at FROM captures WHERE 1=1"
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(cid) = camera_id {
            sql.push_str(&format!(" AND camera_id=?{}", idx));
            param_values.push(Box::new(cid));
            idx += 1;
        }
        if let Some(d) = date {
            sql.push_str(&format!(" AND date(created_at)=?{}", idx));
            param_values.push(Box::new(d.to_string()));
            idx += 1;
        }
        if let Some(ct) = capture_type {
            sql.push_str(&format!(" AND capture_type=?{}", idx));
            param_values.push(Box::new(ct.to_string()));
            let _ = idx;
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT 500");

        let mut stmt = conn.prepare(&sql)?;
        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_ref.as_slice(), |row| {
            Ok(Capture {
                id: row.get(0)?,
                camera_id: row.get(1)?,
                camera_name: row.get(2)?,
                capture_type: row.get(3)?,
                file_path: row.get(4)?,
                file_size: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        let mut captures = Vec::new();
        for row in rows { captures.push(row?); }
        Ok(captures)
    }

    pub fn delete_capture(&self, id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let path: Option<String> = conn.query_row(
            "SELECT file_path FROM captures WHERE id=?1", params![id], |row| row.get(0)
        ).ok();
        conn.execute("DELETE FROM captures WHERE id=?1", params![id])?;
        Ok(path)
    }

    // =========================================================================
    // Notifications CRUD
    // =========================================================================

    pub fn create_notification(&self, category: &str, title: &str, message: &str, severity: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO notifications (category, title, message, severity) VALUES (?1, ?2, ?3, ?4)",
            params![category, title, message, severity],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_notifications(&self, limit: i64) -> Result<Vec<Notification>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, category, title, message, severity, read, created_at FROM notifications ORDER BY created_at DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(Notification {
                id: row.get(0)?,
                category: row.get(1)?,
                title: row.get(2)?,
                message: row.get(3)?,
                severity: row.get(4)?,
                read: row.get::<_, i64>(5)? != 0,
                created_at: row.get(6)?,
            })
        })?;
        let mut notifs = Vec::new();
        for row in rows { notifs.push(row?); }
        Ok(notifs)
    }

    pub fn unread_notification_count(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM notifications WHERE read=0", [], |row| row.get(0))
    }

    pub fn mark_notification_read(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("UPDATE notifications SET read=1 WHERE id=?1", params![id])?;
        Ok(rows > 0)
    }

    pub fn mark_all_notifications_read(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE notifications SET read=1 WHERE read=0", [])?;
        Ok(())
    }

    pub fn delete_notification(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM notifications WHERE id=?1", params![id])?;
        Ok(rows > 0)
    }

    // =========================================================================
    // Roles & Permissions CRUD
    // =========================================================================

    pub fn seed_default_roles(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM roles", [], |row| row.get(0))?;
        if count > 0 { return Ok(()); }

        let modules = ["cameras", "mosaics", "locations", "users", "captures", "notifications", "roles", "settings"];

        // Admin role - full access
        conn.execute("INSERT INTO roles (name, description, is_system) VALUES ('admin', 'Acceso total al sistema', 1)", [])?;
        let admin_id = conn.last_insert_rowid();
        for m in &modules {
            conn.execute(
                "INSERT INTO role_permissions (role_id, module, can_view, can_create, can_edit, can_delete) VALUES (?1, ?2, 1, 1, 1, 1)",
                params![admin_id, m],
            )?;
        }

        // Operator role - can view and edit most, no user/role management
        conn.execute("INSERT INTO roles (name, description, is_system) VALUES ('operator', 'Puede gestionar cámaras y mosaicos', 1)", [])?;
        let op_id = conn.last_insert_rowid();
        for m in &modules {
            let (cv, cc, ce, cd) = match *m {
                "cameras" | "mosaics" | "locations" | "captures" => (1, 1, 1, 1),
                "notifications" => (1, 0, 1, 0),
                _ => (1, 0, 0, 0),
            };
            conn.execute(
                "INSERT INTO role_permissions (role_id, module, can_view, can_create, can_edit, can_delete) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![op_id, m, cv, cc, ce, cd],
            )?;
        }

        // Viewer role - read only
        conn.execute("INSERT INTO roles (name, description, is_system) VALUES ('viewer', 'Solo lectura', 1)", [])?;
        let viewer_id = conn.last_insert_rowid();
        for m in &modules {
            let cv = if *m == "users" || *m == "roles" || *m == "settings" { 0 } else { 1 };
            conn.execute(
                "INSERT INTO role_permissions (role_id, module, can_view, can_create, can_edit, can_delete) VALUES (?1, ?2, ?3, 0, 0, 0)",
                params![viewer_id, m, cv],
            )?;
        }

        info!("Seeded default roles: admin, operator, viewer");
        Ok(())
    }

    pub fn list_roles(&self) -> Result<Vec<RoleWithPermissions>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, description, is_system, created_at FROM roles ORDER BY id")?;
        let roles: Vec<Role> = stmt.query_map([], |row| {
            Ok(Role {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                is_system: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        let mut result = Vec::new();
        for role in roles {
            let mut pstmt = conn.prepare(
                "SELECT id, role_id, module, can_view, can_create, can_edit, can_delete FROM role_permissions WHERE role_id=?1 ORDER BY module"
            )?;
            let perms: Vec<Permission> = pstmt.query_map(params![role.id], |row| {
                Ok(Permission {
                    id: row.get(0)?,
                    role_id: row.get(1)?,
                    module: row.get(2)?,
                    can_view: row.get::<_, i64>(3)? != 0,
                    can_create: row.get::<_, i64>(4)? != 0,
                    can_edit: row.get::<_, i64>(5)? != 0,
                    can_delete: row.get::<_, i64>(6)? != 0,
                })
            })?.filter_map(|r| r.ok()).collect();
            result.push(RoleWithPermissions { role, permissions: perms });
        }
        Ok(result)
    }

    pub fn create_role(&self, name: &str, description: &str, permissions: &[PermissionInput]) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO roles (name, description) VALUES (?1, ?2)",
            params![name, description],
        )?;
        let role_id = conn.last_insert_rowid();
        for p in permissions {
            conn.execute(
                "INSERT INTO role_permissions (role_id, module, can_view, can_create, can_edit, can_delete) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![role_id, p.module, p.can_view as i64, p.can_create as i64, p.can_edit as i64, p.can_delete as i64],
            )?;
        }
        Ok(role_id)
    }

    pub fn update_role(&self, id: i64, name: &str, description: &str, permissions: &[PermissionInput]) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE roles SET name=?1, description=?2 WHERE id=?3",
            params![name, description, id],
        )?;
        if rows == 0 { return Ok(false); }
        conn.execute("DELETE FROM role_permissions WHERE role_id=?1", params![id])?;
        for p in permissions {
            conn.execute(
                "INSERT INTO role_permissions (role_id, module, can_view, can_create, can_edit, can_delete) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, p.module, p.can_view as i64, p.can_create as i64, p.can_edit as i64, p.can_delete as i64],
            )?;
        }
        Ok(true)
    }

    pub fn delete_role(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        // Don't delete system roles
        let is_system: bool = conn.query_row(
            "SELECT is_system FROM roles WHERE id=?1", params![id], |row| row.get::<_, i64>(0)
        ).map(|v| v != 0).unwrap_or(false);
        if is_system { return Ok(false); }
        let rows = conn.execute("DELETE FROM roles WHERE id=?1", params![id])?;
        Ok(rows > 0)
    }

    // =========================================================================
    // Settings
    // =========================================================================

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key=?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=?2",
            params![key, value],
        )?;
        Ok(())
    }

    // =========================================================================
    // Mosaic Shares
    // =========================================================================

    pub fn create_mosaic_share(&self, mosaic_id: i64, mosaic_name: &str, token: &str, emails: &str, expires_at: &str, schedule_start: Option<&str>, schedule_end: Option<&str>) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO mosaic_shares (mosaic_id, mosaic_name, token, emails, expires_at, schedule_start, schedule_end) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![mosaic_id, mosaic_name, token, emails, expires_at, schedule_start, schedule_end],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_mosaic_shares(&self, mosaic_id: Option<i64>) -> Result<Vec<MosaicShare>> {
        let conn = self.conn.lock().unwrap();
        let sql = if mosaic_id.is_some() {
            "SELECT id, mosaic_id, mosaic_name, token, emails, expires_at, schedule_start, schedule_end, active, created_at FROM mosaic_shares WHERE mosaic_id=?1 ORDER BY created_at DESC"
        } else {
            "SELECT id, mosaic_id, mosaic_name, token, emails, expires_at, schedule_start, schedule_end, active, created_at FROM mosaic_shares ORDER BY created_at DESC"
        };
        let mut stmt = conn.prepare(sql)?;
        let params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = if let Some(mid) = mosaic_id {
            vec![Box::new(mid)]
        } else {
            vec![]
        };
        let params_ref: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_ref.as_slice(), |row| {
            Ok(MosaicShare {
                id: row.get(0)?,
                mosaic_id: row.get(1)?,
                mosaic_name: row.get(2)?,
                token: row.get(3)?,
                emails: row.get(4)?,
                expires_at: row.get(5)?,
                schedule_start: row.get(6)?,
                schedule_end: row.get(7)?,
                active: row.get::<_, i64>(8)? != 0,
                created_at: row.get(9)?,
            })
        })?;
        let mut shares = Vec::new();
        for row in rows { shares.push(row?); }
        Ok(shares)
    }

    pub fn get_mosaic_share_by_token(&self, token: &str) -> Result<Option<MosaicShare>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, mosaic_id, mosaic_name, token, emails, expires_at, schedule_start, schedule_end, active, created_at FROM mosaic_shares WHERE token=?1"
        )?;
        let mut rows = stmt.query(params![token])?;
        if let Some(row) = rows.next()? {
            Ok(Some(MosaicShare {
                id: row.get(0)?,
                mosaic_id: row.get(1)?,
                mosaic_name: row.get(2)?,
                token: row.get(3)?,
                emails: row.get(4)?,
                expires_at: row.get(5)?,
                schedule_start: row.get(6)?,
                schedule_end: row.get(7)?,
                active: row.get::<_, i64>(8)? != 0,
                created_at: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_mosaic_share(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM mosaic_shares WHERE id=?1", params![id])?;
        Ok(rows > 0)
    }

    pub fn toggle_mosaic_share(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("UPDATE mosaic_shares SET active = NOT active WHERE id=?1", params![id])?;
        Ok(rows > 0)
    }
}

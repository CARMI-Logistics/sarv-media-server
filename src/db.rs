use rusqlite::{Connection, Result, params};
use std::sync::Mutex;
use tracing::info;

use crate::models::{Camera, Mosaic, MosaicCamera, MosaicWithCameras};

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
            "CREATE TABLE IF NOT EXISTS cameras (
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
            );"
        )?;
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
                "SELECT id, name, host, port, username, password, path, protocol, enabled, record, source_on_demand, created_at, updated_at 
                 FROM cameras WHERE name LIKE ?1 OR host LIKE ?1 ORDER BY name"
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
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })?;
            for row in rows {
                cameras.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, name, host, port, username, password, path, protocol, enabled, record, source_on_demand, created_at, updated_at 
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
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
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
            "SELECT id, name, host, port, username, password, path, protocol, enabled, record, source_on_demand, created_at, updated_at 
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
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
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
            "INSERT INTO cameras (name, host, port, username, password, path, protocol, enabled, record, source_on_demand) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![cam.name, cam.host, cam.port, cam.username, cam.password, cam.path, cam.protocol, cam.enabled, cam.record, cam.source_on_demand],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_camera(&self, id: i64, cam: &Camera) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE cameras SET name=?1, host=?2, port=?3, username=?4, password=?5, path=?6, protocol=?7, enabled=?8, record=?9, source_on_demand=?10, updated_at=datetime('now')
             WHERE id=?11",
            params![cam.name, cam.host, cam.port, cam.username, cam.password, cam.path, cam.protocol, cam.enabled, cam.record, cam.source_on_demand, id],
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
                "INSERT OR IGNORE INTO cameras (name, host, port, username, password, path, protocol, enabled, record, source_on_demand)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 1, 0)",
                params![name, host, port, user, pass, path, proto],
            )?;
        }
        info!("Seed completado: 17 cámaras insertadas");
        Ok(())
    }
}

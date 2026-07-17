//! Conexión a Postgres y ejecución de migraciones (HU 4.1).
//!
//! Responsabilidad única: obtener un pool de conexiones y aplicar las
//! migraciones de esquema al arranque.
//!
//! - Conexión CON REINTENTO: no dependemos de `depends_on` en el compose (en
//!   prod la BD es externa —Cloud SQL— y puede tardar en estar lista).
//! - Migraciones al boot y FAIL-CLOSED: si la BD o las migraciones fallan, el
//!   backend no arranca (mejor caer temprano que servir con esquema inconsistente).

use std::time::Duration;

use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::{info, warn};

/// Migraciones embebidas en el binario desde `./migrations` (se aplican al boot).
/// El macro lee el directorio en tiempo de compilación (no necesita BD).
static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Conecta al pool reintentando ante fallos transitorios (BD aún no lista).
/// Falla tras agotar los intentos.
pub async fn connect_with_retry(database_url: &str) -> Result<PgPool, sqlx::Error> {
    const MAX_ATTEMPTS: u32 = 10;
    const DELAY: Duration = Duration::from_secs(2);

    let mut attempt = 1;
    loop {
        match PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await
        {
            Ok(pool) => {
                info!("Conectado a Postgres (intento {}/{})", attempt, MAX_ATTEMPTS);
                return Ok(pool);
            }
            Err(e) if attempt < MAX_ATTEMPTS => {
                warn!(
                    "No se pudo conectar a Postgres (intento {}/{}): {}. Reintento en {:?}...",
                    attempt, MAX_ATTEMPTS, e, DELAY
                );
                tokio::time::sleep(DELAY).await;
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

/// Aplica las migraciones de esquema pendientes. Idempotente: sqlx registra las
/// aplicadas en `_sqlx_migrations` y no repite las ya corridas.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    info!("Aplicando migraciones de esquema...");
    MIGRATOR.run(pool).await?;
    info!("Migraciones de esquema al día.");
    Ok(())
}

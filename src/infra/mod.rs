//! Adaptadores de infraestructura (HU 4.1): acceso a datos y clientes externos.
//!
//! Aquí viven las implementaciones concretas (Postgres, cliente de la Control
//! API de MediaMTX, etc.) que satisfacen los puertos definidos en `domain`.

pub mod db;
// Adaptadores Postgres (implementan los puertos). Se ejercitan en tests; los
// endpoints los consumen desde HU 4.2+, por eso se permite dead_code hasta entonces.
#[allow(dead_code)]
pub mod postgres;
// Adaptador de la Control API de MediaMTX (lo consume el reconciler).
pub mod mediamtx;

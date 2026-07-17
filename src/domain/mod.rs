//! Capa de dominio (HU 4.1): entidades y contratos (puertos).
//!
//! No conoce infraestructura: aquí NO hay sqlx, ni HTTP, ni cifrado. Los
//! adaptadores en `infra/` implementan estos puertos, y los servicios dependen
//! de los traits (no de la implementación concreta) → Inversión de Dependencias.

pub mod models;
pub mod ports;

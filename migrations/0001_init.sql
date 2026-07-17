-- 0001_init.sql — Esquema inicial del media server (HU 4.1)
--
-- Fuente de verdad de proyectos y cámaras, relación n-a-n de acceso, e historial
-- de fallos. Los IDs uuid los genera la app (Postgres 16 tiene gen_random_uuid
-- nativo, pero la generación vive en el backend para tener una sola fuente de IDs).

-- Mantiene updated_at automáticamente en cada UPDATE (robusto ante ediciones manuales).
create or replace function set_updated_at()
returns trigger as $$
begin
    new.updated_at = now();
    return new;
end;
$$ language plpgsql;

-- Proyectos consumidores (reemplaza clients.json). Secretos hasheados (Argon2id).
-- all_cameras = true concede acceso a TODAS las cámaras (sin filas en project_cameras).
create table projects (
    id           uuid primary key,
    client_id    text not null unique,
    secret_hash  text not null,
    all_cameras  boolean not null default false,
    enabled      boolean not null default true,
    created_at   timestamptz not null default now(),
    updated_at   timestamptz not null default now()
);
create trigger projects_set_updated_at
    before update on projects
    for each row execute function set_updated_at();

-- Cámaras (reemplaza el bloque paths: del mediamtx.yml).
-- rtsp_url_enc: la rtsp_url CIFRADA en reposo (AES-256-GCM: nonce||ciphertext+tag).
create table cameras (
    id            uuid primary key,
    path          text not null unique,
    rtsp_url_enc  bytea not null,
    record        boolean not null default true,
    enabled       boolean not null default true,
    description   text,
    created_at    timestamptz not null default now(),
    updated_at    timestamptz not null default now()
);
create trigger cameras_set_updated_at
    before update on cameras
    for each row execute function set_updated_at();

-- Relación n-a-n: qué cámaras puede consumir cada proyecto.
-- (Si projects.all_cameras = true, el proyecto accede a todas y no requiere filas aquí.)
create table project_cameras (
    project_id  uuid not null references projects(id) on delete cascade,
    camera_id   uuid not null references cameras(id) on delete cascade,
    primary key (project_id, camera_id)
);
create index project_cameras_camera_idx on project_cameras (camera_id);

-- Historial de fallos/diagnósticos (lo llena el agente, HU 4.6).
-- camera_path es referencia LÓGICA (texto) para que el histórico sobreviva al
-- borrado de la cámara (auditoría). diagnosis/raw van SIN credenciales (redactado).
create table failure_history (
    id           bigint generated always as identity primary key,
    camera_path  text not null,
    detected_at  timestamptz not null,
    severity     text not null,
    diagnosis    text,
    raw          jsonb,
    created_at   timestamptz not null default now()
);
create index failure_history_camera_time_idx on failure_history (camera_path, detected_at desc);

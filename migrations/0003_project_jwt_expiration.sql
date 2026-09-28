-- 0003_project_jwt_expiration.sql — Expiración de JWT configurable por proyecto
--
-- jwt_exp_minutes NULL = usa el default global (JWT_EXP_MINUTES). jwt_never_expires
-- = true representa "no expira" con un exp ~100 años en el futuro (ver generate_jwt),
-- no con un exp ausente. Ambas columnas nacen en su valor neutro: ningún proyecto
-- existente cambia de comportamiento al agregarlas.
alter table projects
    add column jwt_exp_minutes integer,
    add column jwt_never_expires boolean not null default false;

-- Red de seguridad para cualquier escritura que no pase por create_project/update_project
-- (esos handlers ya validan lo mismo y responden 400 antes de llegar aquí).
alter table projects
    add constraint projects_jwt_exp_exclusive
    check (jwt_never_expires = false or jwt_exp_minutes is null);

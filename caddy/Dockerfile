# =============================================================================
# Caddy con el plugin de rate-limit (github.com/mholt/caddy-ratelimit)
# La imagen estándar caddy:2 no lo incluye; se compila con xcaddy.
# =============================================================================
FROM caddy:2-builder AS builder
RUN xcaddy build --with github.com/mholt/caddy-ratelimit

FROM caddy:2
COPY --from=builder /usr/bin/caddy /usr/bin/caddy

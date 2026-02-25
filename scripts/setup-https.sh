#!/bin/bash
# =============================================================================
# HTTPS/TLS Setup Script with Let's Encrypt
# =============================================================================
# This script helps configure HTTPS for production deployment

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    log_error "Please run as root or with sudo"
    exit 1
fi

log_info "HTTPS/TLS Setup for CamManager"
echo ""

# Get domain name
read -p "Enter your domain name (e.g., cammanager.example.com): " DOMAIN

if [ -z "$DOMAIN" ]; then
    log_error "Domain name is required"
    exit 1
fi

# Get email for Let's Encrypt
read -p "Enter your email for Let's Encrypt notifications: " EMAIL

if [ -z "$EMAIL" ]; then
    log_error "Email is required"
    exit 1
fi

# Update nginx config with domain
log_info "Updating nginx configuration with domain: $DOMAIN"
sed -i "s/your-domain.com/$DOMAIN/g" nginx/nginx.conf

# Create directories
mkdir -p certs
mkdir -p logs/nginx

# Start nginx with HTTP only first (for certbot validation)
log_info "Starting nginx in HTTP mode for certificate validation..."

# Request certificate
log_info "Requesting SSL certificate from Let's Encrypt..."
docker compose -f docker-compose.yml -f docker-compose.nginx.yml run --rm certbot certonly \
    --webroot \
    --webroot-path=/var/www/certbot \
    --email "$EMAIL" \
    --agree-tos \
    --no-eff-email \
    -d "$DOMAIN"

if [ $? -eq 0 ]; then
    log_info "Certificate obtained successfully!"
else
    log_error "Failed to obtain certificate"
    exit 1
fi

# Copy certificates to nginx directory
log_info "Copying certificates..."
cp -L "/etc/letsencrypt/live/$DOMAIN/fullchain.pem" certs/
cp -L "/etc/letsencrypt/live/$DOMAIN/privkey.pem" certs/
cp -L "/etc/letsencrypt/live/$DOMAIN/chain.pem" certs/

# Set permissions
chmod 644 certs/*.pem

# Restart nginx with HTTPS
log_info "Restarting nginx with HTTPS enabled..."
docker compose -f docker-compose.yml -f docker-compose.nginx.yml restart nginx

# Test configuration
log_info "Testing nginx configuration..."
docker exec nginx-proxy nginx -t

if [ $? -eq 0 ]; then
    log_info "✅ HTTPS setup complete!"
    echo ""
    log_info "Your application is now accessible at: https://$DOMAIN"
    echo ""
    log_warn "Important reminders:"
    echo "  1. Ensure your DNS A record points to this server's IP"
    echo "  2. Firewall must allow ports 80 and 443"
    echo "  3. Certificates will auto-renew via certbot container"
    echo "  4. Test your SSL configuration: https://www.ssllabs.com/ssltest/"
else
    log_error "Nginx configuration test failed"
    exit 1
fi

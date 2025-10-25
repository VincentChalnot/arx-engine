# Deployment Checklist

Use this checklist to ensure a smooth deployment of Arx Engine to your remote servers.

## Pre-Deployment Checklist

### Server Requirements
- [ ] Server(s) running Linux (Ubuntu, Debian, or similar)
- [ ] SSH access configured
- [ ] Sudo/root access available for initial setup
- [ ] Domain names configured with DNS pointing to server(s)

### GitHub Repository Setup
- [ ] Repository forked or cloned
- [ ] GitHub Container Registry access enabled (Settings → Packages)
- [ ] Write permissions for GitHub Actions enabled

## Server Setup (One-time)

### For Each Environment Server

#### 1. Install Required Software
```bash
# SSH into the server
ssh user@server

# Run setup script as root
sudo ./setup-server.sh --all
```

Or manually:
- [ ] Docker Engine installed (`docker --version`)
- [ ] Docker Compose V2 installed (`docker compose version`)
- [ ] Docker proxy network created (`docker network create proxy`)
- [ ] Deployment directories created:
  - [ ] `/opt/arx-engine` (production)
  - [ ] `/opt/arx-engine-staging` (staging)
  - [ ] `/opt/arx-engine-dev` (development)

#### 2. Configure Traefik (if not already done)
- [ ] Traefik container running
- [ ] Connected to `proxy` network
- [ ] SSL resolver `gandi` configured
- [ ] Entrypoint `websecure` (port 443) configured

Example Traefik docker-compose.yml:
```yaml
services:
  traefik:
    image: traefik:v2.10
    command:
      - --api.dashboard=true
      - --providers.docker=true
      - --providers.docker.exposedbydefault=false
      - --entrypoints.websecure.address=:443
      - --certificatesresolvers.gandi.acme.dnschallenge=true
      - --certificatesresolvers.gandi.acme.dnschallenge.provider=gandiv5
      - --certificatesresolvers.gandi.acme.email=your-email@example.com
      - --certificatesresolvers.gandi.acme.storage=/letsencrypt/acme.json
    ports:
      - "443:443"
      - "80:80"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - traefik-letsencrypt:/letsencrypt
    networks:
      - proxy
    environment:
      - GANDIV5_API_KEY=your-gandi-api-key

networks:
  proxy:
    external: true

volumes:
  traefik-letsencrypt:
```

#### 3. Setup SSH Keys
- [ ] SSH key pair generated (if not exists)
- [ ] Public key added to server's `~/.ssh/authorized_keys`
- [ ] Private key accessible for GitHub Actions

## GitHub Actions Setup

### 1. Configure Secrets
Navigate to: Repository → Settings → Secrets and variables → Actions → Secrets

#### Production Secrets
- [ ] `PROD_SSH_HOST` - Production server hostname/IP
- [ ] `PROD_SSH_USER` - SSH username for production
- [ ] `PROD_SSH_KEY` - SSH private key content (full file)

#### Staging Secrets
- [ ] `STAGING_SSH_HOST` - Staging server hostname/IP
- [ ] `STAGING_SSH_USER` - SSH username for staging
- [ ] `STAGING_SSH_KEY` - SSH private key content (full file)

#### Development Secrets
- [ ] `DEV_SSH_HOST` - Development server hostname/IP
- [ ] `DEV_SSH_USER` - SSH username for development
- [ ] `DEV_SSH_KEY` - SSH private key content (full file)

### 2. Configure Variables
Navigate to: Repository → Settings → Secrets and variables → Actions → Variables

#### Production Variables
- [ ] `PROD_FRONTEND_DOMAIN` - e.g., `arx.example.com`
- [ ] `PROD_BACKEND_DOMAIN` - e.g., `api.arx.example.com`
- [ ] `PROD_BACKEND_URL` - e.g., `https://api.arx.example.com`
- [ ] `PROD_DEPLOY_PATH` - (optional) default: `/opt/arx-engine`

#### Staging Variables
- [ ] `STAGING_FRONTEND_DOMAIN` - e.g., `staging.arx.example.com`
- [ ] `STAGING_BACKEND_DOMAIN` - e.g., `api.staging.arx.example.com`
- [ ] `STAGING_BACKEND_URL` - e.g., `https://api.staging.arx.example.com`
- [ ] `STAGING_DEPLOY_PATH` - (optional) default: `/opt/arx-engine-staging`

#### Development Variables
- [ ] `DEV_FRONTEND_DOMAIN` - e.g., `dev.arx.example.com`
- [ ] `DEV_BACKEND_DOMAIN` - e.g., `api.dev.arx.example.com`
- [ ] `DEV_BACKEND_URL` - e.g., `https://api.dev.arx.example.com`
- [ ] `DEV_DEPLOY_PATH` - (optional) default: `/opt/arx-engine-dev`

#### Optional Variables
- [ ] `RUST_LOG` - Logging level (default: `info`)

## DNS Configuration

### Production
- [ ] `PROD_FRONTEND_DOMAIN` A/CNAME record points to production server
- [ ] `PROD_BACKEND_DOMAIN` A/CNAME record points to production server

### Staging
- [ ] `STAGING_FRONTEND_DOMAIN` A/CNAME record points to staging server
- [ ] `STAGING_BACKEND_DOMAIN` A/CNAME record points to staging server

### Development
- [ ] `DEV_FRONTEND_DOMAIN` A/CNAME record points to development server
- [ ] `DEV_BACKEND_DOMAIN` A/CNAME record points to development server

## First Deployment

### Using GitHub Actions (Recommended)

#### Option 1: Automatic on Branch Push
- [ ] Push to `main` branch → deploys to production
- [ ] Push to `develop` branch → deploys to staging

#### Option 2: Manual Workflow Dispatch
1. [ ] Go to Actions tab in GitHub
2. [ ] Select "Build and Deploy" workflow
3. [ ] Click "Run workflow"
4. [ ] Select environment (production/staging/development)
5. [ ] Click "Run workflow" button
6. [ ] Monitor workflow execution in Actions tab

### Using Manual Deployment Script

```bash
./deploy.sh \
  --environment production \
  --host server.example.com \
  --user deploy \
  --key ~/.ssh/id_rsa \
  --frontend arx.example.com \
  --backend api.arx.example.com \
  --backend-url https://api.arx.example.com \
  --build-local
```

- [ ] Script executed successfully
- [ ] Images built (if --build-local) or pulled
- [ ] Files copied to server
- [ ] Containers started

## Post-Deployment Verification

### 1. Check Container Status
```bash
# SSH to server
ssh user@server
cd /opt/arx-engine  # or appropriate path

# Check running containers
docker compose ps
```

Expected output:
- [ ] Frontend container: Up
- [ ] Backend container: Up

### 2. Check Logs
```bash
# View all logs
docker compose logs

# View specific service logs
docker compose logs frontend
docker compose logs backend

# Follow logs in real-time
docker compose logs -f
```

- [ ] No critical errors in logs
- [ ] Backend listening on port 3000
- [ ] Frontend serving on port 80

### 3. Test Endpoints

#### Backend API
```bash
# Test backend directly (from server)
curl http://localhost:3000/new

# Test through Traefik
curl https://api.arx.example.com/new
```

- [ ] Backend responds with binary data (new game state)

#### Frontend
```bash
# Test frontend
curl https://arx.example.com
```

- [ ] Frontend returns HTML page
- [ ] Browser loads game interface
- [ ] Game is playable

### 4. Check SSL Certificates
- [ ] Frontend domain has valid SSL certificate
- [ ] Backend domain has valid SSL certificate
- [ ] No browser security warnings

### 5. Verify Traefik Routing
```bash
# Check Traefik logs
docker logs traefik | grep arx

# Or check Traefik dashboard (if enabled)
```

- [ ] Traefik routing traffic to frontend
- [ ] Traefik routing traffic to backend
- [ ] No routing errors in logs

## Ongoing Operations

### Monitoring
- [ ] Set up log monitoring (optional)
- [ ] Set up uptime monitoring (optional)
- [ ] Set up alerts for container failures (optional)

### Regular Maintenance
- [ ] Review logs periodically: `docker compose logs`
- [ ] Check disk usage: `df -h`
- [ ] Clean old images: `docker image prune -a`
- [ ] Update domains/certificates as needed

### Updating the Application
- [ ] Push changes to appropriate branch (automatic deployment)
- [ ] Or run manual workflow dispatch
- [ ] Or use `deploy.sh` script
- [ ] Verify deployment after update

### Rollback Procedure
If deployment fails:
1. [ ] SSH to server: `ssh user@server`
2. [ ] Navigate to deployment directory: `cd /opt/arx-engine`
3. [ ] Edit `.env` file: `nano .env`
4. [ ] Change image tags to previous version
5. [ ] Restart: `docker compose up -d`
6. [ ] Verify: `docker compose ps`

## Troubleshooting Common Issues

### Issue: Containers not starting
- [ ] Check logs: `docker compose logs`
- [ ] Verify .env file exists and has correct values
- [ ] Check disk space: `df -h`
- [ ] Verify images pulled: `docker images`

### Issue: Traefik not routing
- [ ] Verify proxy network: `docker network ls | grep proxy`
- [ ] Check containers on proxy network: `docker network inspect proxy`
- [ ] Review Traefik logs: `docker logs traefik`
- [ ] Verify domain DNS records

### Issue: SSL certificate errors
- [ ] Check Traefik certificate resolver configuration
- [ ] Verify DNS records are correct
- [ ] Check Traefik logs for ACME errors
- [ ] Ensure ports 80 and 443 are accessible

### Issue: Frontend can't connect to backend
- [ ] Verify config.json in frontend container
- [ ] Check backend URL in browser developer console
- [ ] Test backend endpoint manually: `curl https://api.arx.example.com/new`
- [ ] Check CORS configuration if cross-domain

### Issue: GitHub Actions deployment fails
- [ ] Check workflow logs in Actions tab
- [ ] Verify all secrets and variables are set
- [ ] Test SSH connection manually
- [ ] Check server disk space
- [ ] Verify Docker is running on server

## Security Checklist

- [ ] SSH password authentication disabled
- [ ] SSH key authentication used exclusively  
- [ ] Firewall configured (allow 22, 80, 443)
- [ ] Regular security updates enabled
- [ ] Non-root Docker usage (rootless mode) - optional
- [ ] GitHub secrets never committed to repository
- [ ] Server logs reviewed regularly
- [ ] SSL/TLS certificates auto-renewed

## Documentation References

- [ ] Read [QUICKSTART.md](./QUICKSTART.md) for quick overview
- [ ] Read [DEPLOYMENT.md](./DEPLOYMENT.md) for detailed guide
- [ ] Review [README.md](./README.md) for application details
- [ ] Check `.env.example` for environment variables

## Success Criteria

Your deployment is successful when:
- [x] All containers are running
- [x] Frontend accessible via HTTPS on configured domain
- [x] Backend API responding on configured domain
- [x] SSL certificates valid and auto-renewing
- [x] Game is fully playable in browser
- [x] No errors in container logs
- [x] Monitoring and alerting configured (optional)

Congratulations! Your Arx Engine deployment is complete! 🎉

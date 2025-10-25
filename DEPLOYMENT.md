# Deployment Guide

This guide explains how to deploy the Arx Engine application to remote servers using Docker, Docker Compose, and GitHub Actions.

## Architecture Overview

The deployment consists of two containers:
- **Backend**: A distroless Rust application serving the game API (port 3000)
- **Frontend**: A static web server serving the game UI (port 80)

Both containers are deployed behind a Traefik reverse proxy with automatic SSL/TLS certificate management.

## Prerequisites

### Remote Server Requirements
- Linux server with SSH access
- Docker Engine installed
- Docker Compose V2 installed
- Traefik reverse proxy running with:
  - Network named `proxy`
  - SSL resolver named `gandi`

### GitHub Repository Setup

#### Secrets (per environment)
Configure these secrets in your GitHub repository settings:

**Production:**
- `PROD_SSH_HOST`: SSH hostname or IP address
- `PROD_SSH_USER`: SSH username
- `PROD_SSH_KEY`: SSH private key (full contents)

**Staging:**
- `STAGING_SSH_HOST`: SSH hostname or IP address
- `STAGING_SSH_USER`: SSH username
- `STAGING_SSH_KEY`: SSH private key (full contents)

**Development:**
- `DEV_SSH_HOST`: SSH hostname or IP address
- `DEV_SSH_USER`: SSH username
- `DEV_SSH_KEY`: SSH private key (full contents)

#### Variables (per environment)
Configure these variables in your GitHub repository settings:

**Production:**
- `PROD_DEPLOY_PATH`: Deployment directory on server (default: `/opt/arx-engine`)
- `PROD_FRONTEND_DOMAIN`: Frontend domain (e.g., `arx.example.com`)
- `PROD_BACKEND_DOMAIN`: Backend domain (e.g., `api.arx.example.com`)
- `PROD_BACKEND_URL`: Full backend URL for frontend config (e.g., `https://api.arx.example.com`)

**Staging:**
- `STAGING_DEPLOY_PATH`: Deployment directory on server (default: `/opt/arx-engine-staging`)
- `STAGING_FRONTEND_DOMAIN`: Frontend domain (e.g., `staging.arx.example.com`)
- `STAGING_BACKEND_DOMAIN`: Backend domain (e.g., `api.staging.arx.example.com`)
- `STAGING_BACKEND_URL`: Full backend URL for frontend config

**Development:**
- `DEV_DEPLOY_PATH`: Deployment directory on server (default: `/opt/arx-engine-dev`)
- `DEV_FRONTEND_DOMAIN`: Frontend domain (e.g., `dev.arx.example.com`)
- `DEV_BACKEND_DOMAIN`: Backend domain (e.g., `api.dev.arx.example.com`)
- `DEV_BACKEND_URL`: Full backend URL for frontend config

**Optional (all environments):**
- `RUST_LOG`: Logging level for backend (default: `info`)

## Docker Images

### Backend Image (Distroless)
Built using `Dockerfile.prod`:
- Multi-stage build with Rust Alpine for compilation
- Runtime uses `gcr.io/distroless/static-debian12:nonroot`
- Minimal attack surface and smaller image size
- Non-root user for security

### Frontend Image
Built using `Dockerfile.frontend`:
- Based on `joseluisq/static-web-server:2`
- Embeds all static files in the container
- Includes dynamically generated `config.json` with backend URL

## Deployment Workflow

### Automatic Deployment
The GitHub Actions workflow (`.github/workflows/deploy.yml`) automatically:

1. **Builds** both backend and frontend Docker images
2. **Pushes** images to GitHub Container Registry (ghcr.io)
3. **Deploys** to the appropriate environment:
   - `main` branch → Production
   - `develop` branch → Staging
   - Manual workflow dispatch → Selected environment

### Manual Deployment
You can manually trigger a deployment:

1. Go to Actions tab in GitHub
2. Select "Build and Deploy" workflow
3. Click "Run workflow"
4. Select target environment
5. Click "Run workflow"

## Local Testing

### Build Images Locally
```bash
# Build backend
docker build -f Dockerfile.prod -t arx-backend:test .

# Create config.json for frontend
cat > public/config.json << EOF
{
  "backendUrl": "http://localhost:3000"
}
EOF

# Build frontend
docker build -f Dockerfile.frontend -t arx-frontend:test .
```

### Test with Docker Compose
```bash
# Use the development compose file
docker compose up

# Or test the production compose
export FRONTEND_IMAGE=arx-frontend:test
export BACKEND_IMAGE=arx-backend:test
export ENVIRONMENT=test
export FRONTEND_DOMAIN=localhost
export BACKEND_DOMAIN=localhost
docker compose -f compose.prod.yaml up
```

## Remote Server Setup

### 1. Install Docker and Docker Compose
```bash
# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh

# Verify installation
docker --version
docker compose version
```

### 2. Setup Traefik Network
```bash
# Create the proxy network
docker network create proxy
```

### 3. Configure SSH Access
```bash
# On your local machine, generate an SSH key if needed
ssh-keygen -t ed25519 -C "github-actions-deploy"

# Copy the public key to the server
ssh-copy-id user@server

# Add the private key as a GitHub secret
cat ~/.ssh/id_ed25519
```

### 4. Prepare Deployment Directory
```bash
# On the server
mkdir -p /opt/arx-engine
chown user:user /opt/arx-engine
```

## Monitoring and Maintenance

### View Logs
```bash
# SSH into server
ssh user@server

# Navigate to deployment directory
cd /opt/arx-engine

# View logs
docker compose logs -f

# View specific service logs
docker compose logs -f backend
docker compose logs -f frontend
```

### Update Deployment
Push changes to the appropriate branch:
- `main` → Updates production automatically
- `develop` → Updates staging automatically
- Other branches → Use manual workflow dispatch

### Rollback
```bash
# SSH into server
ssh user@server
cd /opt/arx-engine

# Edit .env to use previous image tags
nano .env

# Restart services
docker compose up -d
```

## Traefik Configuration

The `compose.prod.yaml` file includes labels for Traefik:
- Automatic routing based on domain
- HTTPS with Let's Encrypt via Gandi DNS
- Network isolation with the `proxy` network

### Example Traefik Labels
```yaml
labels:
  - "traefik.enable=true"
  - "traefik.http.routers.arx-frontend-prod.rule=Host(`arx.example.com`)"
  - "traefik.http.routers.arx-frontend-prod.entrypoints=websecure"
  - "traefik.http.routers.arx-frontend-prod.tls.certresolver=gandi"
  - "traefik.http.services.arx-frontend-prod.loadbalancer.server.port=80"
  - "traefik.docker.network=proxy"
```

## Security Considerations

1. **Distroless Backend**: Minimal attack surface with no shell or package manager
2. **Non-root Containers**: Both containers run as non-root users
3. **Network Isolation**: Services only exposed through Traefik
4. **Automated Updates**: Push to main/develop triggers automatic deployment
5. **SSH Key Authentication**: No password authentication for deployments
6. **Secrets Management**: Sensitive data stored as GitHub secrets

## Troubleshooting

### Images Not Pulling
```bash
# Check GitHub token permissions
# Ensure packages:write permission is enabled

# Login manually on server
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin
```

### Traefik Not Routing
```bash
# Check network
docker network inspect proxy

# Verify service is on proxy network
docker compose ps
docker inspect <container_id>

# Check Traefik logs
docker logs traefik
```

### Backend Not Starting
```bash
# Check logs
docker compose logs backend

# Verify environment variables
docker compose config
```

### Frontend Config Missing
The `config.json` is created during the GitHub Actions build step. Ensure:
- `BACKEND_URL` variable is set correctly
- The file is included in the frontend image build

## Multi-Environment Setup

To run multiple environments on the same server:

1. Use different deployment paths:
   - Production: `/opt/arx-engine`
   - Staging: `/opt/arx-engine-staging`
   - Development: `/opt/arx-engine-dev`

2. Use different domains:
   - Production: `arx.example.com`, `api.arx.example.com`
   - Staging: `staging.arx.example.com`, `api.staging.arx.example.com`
   - Development: `dev.arx.example.com`, `api.dev.arx.example.com`

3. Use environment-specific `.env` files with unique container names

## Alternative: Private Docker Registry

If you want to use a private registry instead of GitHub Container Registry:

1. Update `.github/workflows/deploy.yml`:
   ```yaml
   env:
     REGISTRY: registry.example.com
   ```

2. Add registry credentials as secrets:
   - `REGISTRY_USERNAME`
   - `REGISTRY_PASSWORD`

3. Update the login action:
   ```yaml
   - name: Log in to Container Registry
     uses: docker/login-action@v3
     with:
       registry: ${{ env.REGISTRY }}
       username: ${{ secrets.REGISTRY_USERNAME }}
       password: ${{ secrets.REGISTRY_PASSWORD }}
   ```

## Support

For issues or questions:
1. Check the GitHub Actions logs
2. Review server logs: `docker compose logs`
3. Verify Traefik configuration
4. Check network connectivity

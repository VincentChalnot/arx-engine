# Quick Start Guide

## Overview
This deployment setup provides a professional, multi-environment solution for deploying the Arx Engine to remote servers using Docker, GitHub Actions, and Traefik.

## Key Features
- ✅ **Distroless backend** for minimal attack surface
- ✅ **Static file frontend** with embedded config
- ✅ **Multi-environment support** (production, staging, development)
- ✅ **Automated CI/CD** via GitHub Actions
- ✅ **Traefik integration** with automatic SSL
- ✅ **Zero-downtime deployments**
- ✅ **Configurable via environment variables**

## Files Overview

### Docker Images
- `Dockerfile.prod` - Distroless backend image (statically linked Rust binary)
- `Dockerfile.frontend` - Frontend image with embedded static files

### Docker Compose
- `compose.yaml` - Development/local setup with volume mounts
- `compose.prod.yaml` - Production setup with Traefik labels

### CI/CD
- `.github/workflows/deploy.yml` - Automated build and deployment workflow

### Documentation
- `DEPLOYMENT.md` - Complete deployment guide
- `.env.example` - Environment variables template

## Quick Setup

### 1. Configure GitHub Secrets
Add these secrets in your GitHub repository (Settings → Secrets and variables → Actions):

**For Production:**
```
PROD_SSH_HOST       # Server hostname/IP
PROD_SSH_USER       # SSH username  
PROD_SSH_KEY        # SSH private key
```

**For Staging:**
```
STAGING_SSH_HOST
STAGING_SSH_USER
STAGING_SSH_KEY
```

**For Development:**
```
DEV_SSH_HOST
DEV_SSH_USER
DEV_SSH_KEY
```

### 2. Configure GitHub Variables
Add these variables in your GitHub repository (Settings → Secrets and variables → Actions → Variables):

**For Production:**
```
PROD_FRONTEND_DOMAIN    # e.g., arx.example.com
PROD_BACKEND_DOMAIN     # e.g., api.arx.example.com
PROD_BACKEND_URL        # e.g., https://api.arx.example.com
PROD_DEPLOY_PATH        # e.g., /opt/arx-engine (optional)
```

Repeat for `STAGING_*` and `DEV_*` with appropriate values.

### 3. Server Setup
On each remote server:

```bash
# Install Docker
curl -fsSL https://get.docker.com | sh

# Create Traefik proxy network
docker network create proxy

# Create deployment directory
mkdir -p /opt/arx-engine
```

### 4. Deploy
Push to the appropriate branch or manually trigger the workflow:

- Push to `main` → Deploys to production
- Push to `develop` → Deploys to staging  
- Manual workflow → Choose environment

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Traefik (Reverse Proxy)                │
│                   SSL/TLS (Let's Encrypt)                   │
└─────────────────┬──────────────────────┬────────────────────┘
                  │                      │
        ┌─────────▼─────────┐  ┌────────▼────────┐
        │    Frontend       │  │    Backend       │
        │  (Static Server)  │  │  (Rust/Axum)     │
        │   Port: 80        │  │   Port: 3000     │
        │  Distroless Base  │  │ Distroless Base  │
        └───────────────────┘  └──────────────────┘
```

## Directory Structure on Server

```
/opt/arx-engine/              # Production
├── compose.yaml              # Docker Compose config
└── .env                      # Environment variables

/opt/arx-engine-staging/      # Staging
├── compose.yaml
└── .env

/opt/arx-engine-dev/          # Development
├── compose.yaml
└── .env
```

## Local Development

### Using Docker Compose (Development Setup)
```bash
# Start development environment
docker compose up

# Access frontend at http://localhost:8080
# Access backend at http://localhost:3000
```

### Testing Production Images Locally
```bash
# Create config.json
cat > public/config.json << EOF
{
  "backendUrl": "http://localhost:3000"
}
EOF

# Build images
docker build -f Dockerfile.prod -t arx-backend:local .
docker build -f Dockerfile.frontend -t arx-frontend:local .

# Run with production compose
export FRONTEND_IMAGE=arx-frontend:local
export BACKEND_IMAGE=arx-backend:local
export ENVIRONMENT=local
export FRONTEND_DOMAIN=localhost
export BACKEND_DOMAIN=localhost
export RUST_LOG=debug

docker compose -f compose.prod.yaml up
```

## Monitoring

### View Deployment Status
```bash
# Check GitHub Actions for build/deploy status
# Navigate to: Repository → Actions

# SSH to server and check containers
ssh user@server
cd /opt/arx-engine
docker compose ps
docker compose logs -f
```

### Health Checks
```bash
# Check backend
curl https://api.arx.example.com/new

# Check frontend
curl https://arx.example.com
```

## Rollback Procedure

If a deployment fails:

```bash
# SSH to server
ssh user@server
cd /opt/arx-engine

# Edit .env to use previous image tag
nano .env

# Restart with previous version
docker compose up -d
```

## Security Features

1. **Distroless Images**: No shell, package manager, or unnecessary tools
2. **Non-root Execution**: Containers run as non-root user
3. **Private Network**: Containers only exposed via Traefik
4. **Automated SSL**: Let's Encrypt certificates via Traefik
5. **Secrets Management**: Sensitive data in GitHub Secrets
6. **SSH Key Auth**: No passwords for deployment

## Troubleshooting

### Issue: Deployment fails with SSH error
**Solution**: Verify SSH key is correctly added to GitHub Secrets and server authorized_keys

### Issue: Traefik not routing requests
**Solution**: Check that:
- Traefik proxy network exists: `docker network ls`
- Domains are correctly configured in DNS
- Environment variables are set in .env file

### Issue: Backend not responding
**Solution**: Check logs: `docker compose logs backend`

### Issue: Frontend shows connection error
**Solution**: Verify config.json contains correct backend URL

## Next Steps

1. Read the full [DEPLOYMENT.md](./DEPLOYMENT.md) guide
2. Configure your GitHub secrets and variables
3. Setup your remote servers with Docker and Traefik
4. Push to trigger your first deployment
5. Monitor the GitHub Actions workflow
6. Verify the deployment on your domain

## Support

For detailed information, see:
- [DEPLOYMENT.md](./DEPLOYMENT.md) - Full deployment documentation
- [README.md](./README.md) - Application overview
- [rules.md](./rules.md) - Game rules

GitHub Actions logs provide detailed information about build and deployment steps.

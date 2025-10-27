# Deployment Architecture

## High-Level Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                         Internet                                │
└──────────────────┬────────────────────┬────────────────────────┘
                   │                    │
                   │ HTTPS (443)        │ HTTPS (443)
                   │                    │
┌──────────────────▼────────────────────▼─────────────────────────┐
│                     Traefik Reverse Proxy                        │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Features:                                                │  │
│  │  • Automatic SSL/TLS via Let's Encrypt                    │  │
│  │  • DNS Challenge (Gandi)                                  │  │
│  │  • HTTP → HTTPS redirect                                  │  │
│  │  • Domain-based routing                                   │  │
│  │  • Load balancing                                         │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Routes:                                                         │
│  • arx.example.com         → Frontend Container                 │
│  • api.arx.example.com     → Backend Container                  │
└──────────────────┬────────────────────┬─────────────────────────┘
                   │                    │
          ┌────────▼────────┐  ┌───────▼────────┐
          │   Frontend      │  │    Backend     │
          │   Container     │  │   Container    │
          │                 │  │                │
          │  Port: 80       │  │  Port: 3000    │
          │  Image:         │  │  Image:        │
          │  static-server  │  │  distroless    │
          │                 │  │  Rust binary   │
          │  Content:       │  │                │
          │  • HTML/CSS/JS  │  │  API Endpoints:│
          │  • config.json  │  │  • /new        │
          │  • Assets       │  │  • /moves      │
          │                 │  │  • /play       │
          └─────────────────┘  └────────────────┘
                   │                    │
                   └──────────┬─────────┘
                              │
                    ┌─────────▼─────────┐
                    │  Docker Network   │
                    │     "proxy"       │
                    └───────────────────┘
```

## Multi-Environment Setup

```
┌─────────────────────────────────────────────────────────────┐
│                      Remote Server(s)                        │
│                                                              │
│  Production Environment (/opt/arx-engine)                   │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  • Domain: arx.example.com                           │  │
│  │  • Backend: api.arx.example.com                      │  │
│  │  • Branch: main                                      │  │
│  │  • Auto-deploy: Yes                                  │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                              │
│  Staging Environment (/opt/arx-engine-staging)              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  • Domain: staging.arx.example.com                   │  │
│  │  • Backend: api.staging.arx.example.com              │  │
│  │  • Branch: develop                                   │  │
│  │  • Auto-deploy: Yes                                  │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                              │
│  Development Environment (/opt/arx-engine-dev)              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  • Domain: dev.arx.example.com                       │  │
│  │  • Backend: api.dev.arx.example.com                  │  │
│  │  • Branch: manual trigger                            │  │
│  │  • Auto-deploy: No                                   │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                              │
│  All environments share the same "proxy" network            │
└─────────────────────────────────────────────────────────────┘
```

## CI/CD Pipeline

```
┌──────────────────────────────────────────────────────────────┐
│                     GitHub Repository                         │
│                                                               │
│  Developer pushes to:                                         │
│  • main branch                                                │
│  • develop branch                                             │
│  • Or manually triggers workflow                              │
└──────────────────┬───────────────────────────────────────────┘
                   │
                   │ Triggers
                   │
┌──────────────────▼───────────────────────────────────────────┐
│                  GitHub Actions Workflow                      │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  1. Build Stage                                      │    │
│  │     • Create config.json with backend URL            │    │
│  │     • Build backend (Dockerfile.prod)                │    │
│  │     • Build frontend (Dockerfile.frontend)           │    │
│  │     • Push to GitHub Container Registry              │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  2. Deploy Stage                                     │    │
│  │     • Determine target environment                   │    │
│  │     • Setup SSH connection                           │    │
│  │     • Create .env file                               │    │
│  │     • Copy compose.yaml and .env to server           │    │
│  │     • Execute: docker compose pull                   │    │
│  │     • Execute: docker compose up -d                  │    │
│  │     • Verify deployment                              │    │
│  │     • Clean old images                               │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────┬───────────────────────────────────────────┘
                   │
                   │ SSH/SCP
                   │
┌──────────────────▼───────────────────────────────────────────┐
│                     Remote Server                             │
│                                                               │
│  Docker Compose pulls images and starts containers           │
│  Traefik detects new containers via labels                   │
│  SSL certificates are automatically provisioned              │
│  Application is now live!                                    │
└──────────────────────────────────────────────────────────────┘
```

## Container Image Build Process

```
┌─────────────────────────────────────────────────────────────┐
│                 Backend (Dockerfile.prod)                    │
│                                                              │
│  Stage 1: Builder (rust:1.83-alpine)                        │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  • Install musl-dev                                    │ │
│  │  • Copy Cargo.toml, Cargo.lock                         │ │
│  │  • Copy src/                                           │ │
│  │  • cargo build --target x86_64-unknown-linux-musl      │ │
│  │  • Result: statically linked binary                    │ │
│  └────────────────────────────────────────────────────────┘ │
│                          ↓                                   │
│  Stage 2: Runtime (distroless/static-debian12:nonroot)      │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  • Copy binary from builder                            │ │
│  │  • Run as non-root user                                │ │
│  │  • No shell, no package manager                        │ │
│  │  • Minimal attack surface                              │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│              Frontend (Dockerfile.frontend)                  │
│                                                              │
│  Base: static-web-server:2                                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  • Copy public/ directory                              │ │
│  │  • Includes config.json (created in CI)                │ │
│  │  • All static files embedded                           │ │
│  │  • No volume mounts needed                             │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Request Flow

```
User Browser
     │
     │ 1. HTTPS Request
     │    https://arx.example.com
     │
     ▼
Traefik (443)
     │
     │ 2. Route based on domain
     │    Match: Host(`arx.example.com`)
     │
     ▼
Frontend Container (80)
     │
     │ 3. Serve HTML/JS/CSS
     │    + config.json
     │
     ▼
User Browser
     │
     │ 4. Load config.json
     │    Get backend URL
     │
     │ 5. API Request
     │    https://api.arx.example.com/new
     │
     ▼
Traefik (443)
     │
     │ 6. Route based on domain
     │    Match: Host(`api.arx.example.com`)
     │
     ▼
Backend Container (3000)
     │
     │ 7. Process request
     │    Generate new game state
     │
     │ 8. Return binary data
     │
     ▼
User Browser
     │
     │ 9. Render game board
     │
```

## Security Layers

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 1: Network                                            │
│  • Firewall: Only ports 22, 80, 443 open                    │
│  • SSH: Key-based authentication only                        │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  Layer 2: Traefik                                            │
│  • TLS 1.2+ only                                             │
│  • Automatic SSL certificate renewal                         │
│  • HTTP → HTTPS redirect                                     │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  Layer 3: Docker Network Isolation                           │
│  • Containers only accessible via Traefik                    │
│  • Internal "proxy" network                                  │
│  • No direct port exposure                                   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  Layer 4: Container Security                                 │
│  • Distroless images (no shell, no pkg manager)              │
│  • Non-root user execution                                   │
│  • Minimal dependencies                                      │
│  • Read-only file systems (where applicable)                 │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  Layer 5: Application                                        │
│  • Memory-safe Rust code                                     │
│  • Input validation                                          │
│  • CORS configured                                           │
└─────────────────────────────────────────────────────────────┘
```

## File Structure on Server

```
/opt/arx-engine/                    # Production
├── compose.yaml                    # Docker Compose config (copied from compose.prod.yaml)
└── .env                            # Environment variables

/opt/arx-engine-staging/            # Staging
├── compose.yaml
└── .env

/opt/arx-engine-dev/                # Development
├── compose.yaml
└── .env

# No persistent data directories needed currently
# (Future: Add /data or /volumes for game state persistence)
```

## Environment Variable Flow

```
GitHub Repository Variables/Secrets
           ↓
GitHub Actions Workflow
           ↓
.env file created on CI runner
           ↓
Copied to remote server via SCP
           ↓
Docker Compose reads .env
           ↓
Containers receive environment variables
           ↓
Application uses configuration
```

## Deployment Methods Comparison

```
┌─────────────────────┬────────────────────┬──────────────────┐
│ Method              │ Pros               │ Use Case         │
├─────────────────────┼────────────────────┼──────────────────┤
│ GitHub Actions      │ • Fully automated  │ Production       │
│ (Recommended)       │ • Version tracked  │ Staging          │
│                     │ • Audit trail      │ CI/CD pipeline   │
├─────────────────────┼────────────────────┼──────────────────┤
│ deploy.sh           │ • Simple           │ Testing          │
│ (Manual Script)     │ • No GitHub needed │ One-off deploys  │
│                     │ • Local control    │ Emergencies      │
├─────────────────────┼────────────────────┼──────────────────┤
│ Direct Docker       │ • Maximum control  │ Development      │
│ (Manual)            │ • Learning         │ Debugging        │
│                     │ • Customization    │ Experimentation  │
└─────────────────────┴────────────────────┴──────────────────┘
```

## Key Design Decisions

1. **Distroless Backend**: Minimal attack surface, no shell access
2. **Embedded Frontend Files**: No volume mounts, immutable containers
3. **Traefik Labels**: Self-configuring routing, no manual nginx config
4. **Multi-environment**: Separate domains and paths, isolated deployments
5. **GitHub Actions**: Automated CI/CD, version-controlled deployments
6. **SSH Deployment**: Simple, secure, works anywhere
7. **Environment Variables**: Flexible configuration without code changes
8. **No Persistent Data**: Stateless design (can add volumes later if needed)

## Monitoring Points

```
┌──────────────────────────────────────────────────────────────┐
│  What to Monitor                                             │
├──────────────────────────────────────────────────────────────┤
│  • Container Status: docker compose ps                       │
│  • Container Logs: docker compose logs                       │
│  • Traefik Logs: docker logs traefik                         │
│  • SSL Certificate Expiry: Traefik dashboard                 │
│  • Disk Usage: df -h                                         │
│  • Memory Usage: free -h                                     │
│  • Network Traffic: docker stats                             │
│  • HTTP Status: curl health checks                           │
│  • Response Times: browser dev tools                         │
└──────────────────────────────────────────────────────────────┘
```

This architecture provides a robust, secure, and maintainable deployment solution for the Arx Engine application.

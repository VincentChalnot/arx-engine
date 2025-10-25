#!/bin/bash
set -e

# Manual Deployment Script for Arx Engine
# This script can be used to deploy the application to a remote server
# without using GitHub Actions

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
print_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check if required tools are installed
check_requirements() {
    print_info "Checking requirements..."
    
    for cmd in docker ssh scp; do
        if ! command -v $cmd &> /dev/null; then
            print_error "$cmd is not installed"
            exit 1
        fi
    done
    
    print_info "All requirements met"
}

# Function to show usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Manual deployment script for Arx Engine

OPTIONS:
    -e, --environment    Environment (production|staging|development) [required]
    -h, --host          SSH host [required]
    -u, --user          SSH user [required]
    -k, --key           SSH key path (default: ~/.ssh/id_rsa)
    -p, --path          Deployment path (default: /opt/arx-engine)
    -f, --frontend      Frontend domain [required]
    -b, --backend       Backend domain [required]
    --backend-url       Backend URL for frontend config [required]
    --frontend-image    Frontend Docker image (default: build locally)
    --backend-image     Backend Docker image (default: build locally)
    --registry          Docker registry (default: ghcr.io/vincentchalnot/arx-engine)
    --build-local       Build images locally instead of pulling
    --help              Show this help message

EXAMPLES:
    # Deploy to production with local build
    $0 -e production -h server.example.com -u deploy \\
       -f arx.example.com -b api.arx.example.com \\
       --backend-url https://api.arx.example.com \\
       --build-local

    # Deploy to staging with pre-built images
    $0 -e staging -h staging.example.com -u deploy \\
       -f staging.arx.example.com -b api.staging.arx.example.com \\
       --backend-url https://api.staging.arx.example.com \\
       --frontend-image ghcr.io/user/arx-frontend:latest \\
       --backend-image ghcr.io/user/arx-backend:latest

EOF
    exit 1
}

# Parse command line arguments
ENVIRONMENT=""
SSH_HOST=""
SSH_USER=""
SSH_KEY="$HOME/.ssh/id_rsa"
DEPLOY_PATH=""
FRONTEND_DOMAIN=""
BACKEND_DOMAIN=""
BACKEND_URL=""
FRONTEND_IMAGE=""
BACKEND_IMAGE=""
REGISTRY="ghcr.io/vincentchalnot/arx-engine"
BUILD_LOCAL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -e|--environment)
            ENVIRONMENT="$2"
            shift 2
            ;;
        -h|--host)
            SSH_HOST="$2"
            shift 2
            ;;
        -u|--user)
            SSH_USER="$2"
            shift 2
            ;;
        -k|--key)
            SSH_KEY="$2"
            shift 2
            ;;
        -p|--path)
            DEPLOY_PATH="$2"
            shift 2
            ;;
        -f|--frontend)
            FRONTEND_DOMAIN="$2"
            shift 2
            ;;
        -b|--backend)
            BACKEND_DOMAIN="$2"
            shift 2
            ;;
        --backend-url)
            BACKEND_URL="$2"
            shift 2
            ;;
        --frontend-image)
            FRONTEND_IMAGE="$2"
            shift 2
            ;;
        --backend-image)
            BACKEND_IMAGE="$2"
            shift 2
            ;;
        --registry)
            REGISTRY="$2"
            shift 2
            ;;
        --build-local)
            BUILD_LOCAL=true
            shift
            ;;
        --help)
            usage
            ;;
        *)
            print_error "Unknown option: $1"
            usage
            ;;
    esac
done

# Validate required arguments
if [ -z "$ENVIRONMENT" ] || [ -z "$SSH_HOST" ] || [ -z "$SSH_USER" ] || \
   [ -z "$FRONTEND_DOMAIN" ] || [ -z "$BACKEND_DOMAIN" ] || [ -z "$BACKEND_URL" ]; then
    print_error "Missing required arguments"
    usage
fi

# Set default deployment path based on environment
if [ -z "$DEPLOY_PATH" ]; then
    case $ENVIRONMENT in
        production)
            DEPLOY_PATH="/opt/arx-engine"
            ;;
        staging)
            DEPLOY_PATH="/opt/arx-engine-staging"
            ;;
        development)
            DEPLOY_PATH="/opt/arx-engine-dev"
            ;;
        *)
            print_error "Invalid environment: $ENVIRONMENT"
            exit 1
            ;;
    esac
fi

# Set image names
if [ -z "$FRONTEND_IMAGE" ]; then
    if [ "$BUILD_LOCAL" = true ]; then
        FRONTEND_IMAGE="arx-frontend:${ENVIRONMENT}"
    else
        FRONTEND_IMAGE="${REGISTRY}-frontend:latest"
    fi
fi

if [ -z "$BACKEND_IMAGE" ]; then
    if [ "$BUILD_LOCAL" = true ]; then
        BACKEND_IMAGE="arx-backend:${ENVIRONMENT}"
    else
        BACKEND_IMAGE="${REGISTRY}-backend:latest"
    fi
fi

# Display configuration
print_info "Deployment Configuration:"
echo "  Environment:      $ENVIRONMENT"
echo "  SSH Host:         $SSH_HOST"
echo "  SSH User:         $SSH_USER"
echo "  Deploy Path:      $DEPLOY_PATH"
echo "  Frontend Domain:  $FRONTEND_DOMAIN"
echo "  Backend Domain:   $BACKEND_DOMAIN"
echo "  Backend URL:      $BACKEND_URL"
echo "  Frontend Image:   $FRONTEND_IMAGE"
echo "  Backend Image:    $BACKEND_IMAGE"
echo "  Build Local:      $BUILD_LOCAL"
echo ""

# Confirm deployment
read -p "Continue with deployment? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_warn "Deployment cancelled"
    exit 0
fi

check_requirements

# Build images locally if requested
if [ "$BUILD_LOCAL" = true ]; then
    print_info "Building Docker images locally..."
    
    # Create config.json for frontend
    print_info "Creating config.json..."
    mkdir -p public
    cat > public/config.json << EOF
{
  "backendUrl": "$BACKEND_URL"
}
EOF
    
    # Build backend
    print_info "Building backend image..."
    docker build -f Dockerfile.prod -t "$BACKEND_IMAGE" .
    
    # Build frontend
    print_info "Building frontend image..."
    docker build -f Dockerfile.frontend -t "$FRONTEND_IMAGE" .
    
    print_info "Images built successfully"
fi

# Test SSH connection
print_info "Testing SSH connection..."
if ! ssh -i "$SSH_KEY" -o BatchMode=yes -o ConnectTimeout=5 "$SSH_USER@$SSH_HOST" "echo 'Connection successful'" &> /dev/null; then
    print_error "Cannot connect to $SSH_HOST"
    exit 1
fi
print_info "SSH connection successful"

# Create deployment directory on remote server
print_info "Creating deployment directory..."
ssh -i "$SSH_KEY" "$SSH_USER@$SSH_HOST" "mkdir -p $DEPLOY_PATH"

# Create .env file
print_info "Creating .env file..."
cat > /tmp/arx-deploy.env << EOF
ENVIRONMENT=$ENVIRONMENT
FRONTEND_IMAGE=$FRONTEND_IMAGE
BACKEND_IMAGE=$BACKEND_IMAGE
FRONTEND_DOMAIN=$FRONTEND_DOMAIN
BACKEND_DOMAIN=$BACKEND_DOMAIN
RUST_LOG=info
EOF

# Copy files to server
print_info "Copying files to server..."
scp -i "$SSH_KEY" compose.prod.yaml "$SSH_USER@$SSH_HOST:$DEPLOY_PATH/compose.yaml"
scp -i "$SSH_KEY" /tmp/arx-deploy.env "$SSH_USER@$SSH_HOST:$DEPLOY_PATH/.env"

# Clean up local temp file
rm /tmp/arx-deploy.env

# If building locally, save and transfer images
if [ "$BUILD_LOCAL" = true ]; then
    print_info "Transferring Docker images to server..."
    
    docker save "$BACKEND_IMAGE" | ssh -i "$SSH_KEY" "$SSH_USER@$SSH_HOST" "docker load"
    docker save "$FRONTEND_IMAGE" | ssh -i "$SSH_KEY" "$SSH_USER@$SSH_HOST" "docker load"
    
    print_info "Images transferred successfully"
fi

# Deploy with Docker Compose
print_info "Deploying application..."

# Pull images if not built locally
if [ "$BUILD_LOCAL" != true ]; then
    ssh -i "$SSH_KEY" "$SSH_USER@$SSH_HOST" << EOF
cd $DEPLOY_PATH

# Pull images
docker compose pull

# Deploy
docker compose up -d

# Show status
docker compose ps

# Clean up old images
docker image prune -af --filter "until=24h" || true
EOF
else
    ssh -i "$SSH_KEY" "$SSH_USER@$SSH_HOST" << EOF
cd $DEPLOY_PATH

# Deploy (images already loaded)
docker compose up -d

# Show status
docker compose ps

# Clean up old images
docker image prune -af --filter "until=24h" || true
EOF
fi

# Verify deployment
print_info "Verifying deployment..."
if ssh -i "$SSH_KEY" "$SSH_USER@$SSH_HOST" "cd $DEPLOY_PATH && docker compose ps | grep -q 'Up'"; then
    print_info "✓ Deployment successful!"
    echo ""
    echo "Frontend: https://$FRONTEND_DOMAIN"
    echo "Backend:  https://$BACKEND_DOMAIN"
    echo ""
    print_info "Check logs with:"
    echo "  ssh -i $SSH_KEY $SSH_USER@$SSH_HOST 'cd $DEPLOY_PATH && docker compose logs -f'"
else
    print_error "Deployment verification failed"
    exit 1
fi

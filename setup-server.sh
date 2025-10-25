#!/bin/bash
set -e

# Server Setup Script for Arx Engine Deployment
# This script helps set up a server for deploying the Arx Engine application

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
print_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1"; }
print_step() { echo -e "${BLUE}[STEP]${NC} $1"; }

usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Setup script for Arx Engine deployment server

OPTIONS:
    --install-docker    Install Docker and Docker Compose
    --setup-network     Create Docker proxy network for Traefik
    --create-dirs       Create deployment directories
    --all               Perform all setup steps
    --help              Show this help message

EXAMPLES:
    # Full setup
    $0 --all

    # Install Docker only
    $0 --install-docker

    # Setup network and directories
    $0 --setup-network --create-dirs

EOF
    exit 1
}

# Check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then 
        print_error "Please run as root or with sudo"
        exit 1
    fi
}

# Install Docker and Docker Compose
install_docker() {
    print_step "Installing Docker..."
    
    if command -v docker &> /dev/null; then
        print_info "Docker is already installed"
        docker --version
    else
        print_info "Downloading and installing Docker..."
        curl -fsSL https://get.docker.com -o /tmp/get-docker.sh
        sh /tmp/get-docker.sh
        rm /tmp/get-docker.sh
        
        print_info "Starting Docker service..."
        systemctl enable docker
        systemctl start docker
        
        print_info "✓ Docker installed successfully"
        docker --version
    fi
    
    # Verify Docker Compose
    if docker compose version &> /dev/null; then
        print_info "✓ Docker Compose is available"
        docker compose version
    else
        print_warn "Docker Compose V2 not available, please update Docker"
        exit 1
    fi
}

# Setup Docker network for Traefik
setup_network() {
    print_step "Setting up Docker network..."
    
    if docker network ls | grep -q "proxy"; then
        print_info "Network 'proxy' already exists"
    else
        print_info "Creating network 'proxy'..."
        docker network create proxy
        print_info "✓ Network 'proxy' created"
    fi
    
    docker network ls | grep proxy
}

# Create deployment directories
create_directories() {
    print_step "Creating deployment directories..."
    
    local dirs=(
        "/opt/arx-engine"
        "/opt/arx-engine-staging"
        "/opt/arx-engine-dev"
    )
    
    for dir in "${dirs[@]}"; do
        if [ -d "$dir" ]; then
            print_info "Directory $dir already exists"
        else
            print_info "Creating directory $dir..."
            mkdir -p "$dir"
            print_info "✓ Created $dir"
        fi
    done
    
    # Set permissions (assuming non-root user 'deploy' will deploy)
    if id "deploy" &>/dev/null; then
        print_info "Setting ownership to 'deploy' user..."
        chown -R deploy:deploy /opt/arx-engine*
    else
        print_warn "User 'deploy' not found. You may need to set permissions manually."
        print_info "Example: chown -R youruser:youruser /opt/arx-engine*"
    fi
}

# Display system information
show_info() {
    print_step "System Information"
    echo ""
    echo "OS: $(uname -s)"
    echo "Kernel: $(uname -r)"
    echo "Architecture: $(uname -m)"
    echo ""
    
    if command -v docker &> /dev/null; then
        echo "Docker: $(docker --version)"
        echo "Docker Compose: $(docker compose version)"
    else
        echo "Docker: Not installed"
    fi
    echo ""
}

# Verify setup
verify_setup() {
    print_step "Verifying setup..."
    
    local errors=0
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed"
        ((errors++))
    else
        print_info "✓ Docker is installed"
    fi
    
    # Check Docker Compose
    if ! docker compose version &> /dev/null; then
        print_error "Docker Compose is not available"
        ((errors++))
    else
        print_info "✓ Docker Compose is available"
    fi
    
    # Check network
    if ! docker network ls | grep -q "proxy"; then
        print_error "Network 'proxy' does not exist"
        ((errors++))
    else
        print_info "✓ Network 'proxy' exists"
    fi
    
    # Check directories
    for dir in /opt/arx-engine /opt/arx-engine-staging /opt/arx-engine-dev; do
        if [ ! -d "$dir" ]; then
            print_error "Directory $dir does not exist"
            ((errors++))
        else
            print_info "✓ Directory $dir exists"
        fi
    done
    
    echo ""
    if [ $errors -eq 0 ]; then
        print_info "✓ All checks passed! Server is ready for deployment."
    else
        print_error "Setup verification failed with $errors error(s)"
        exit 1
    fi
}

# Parse command line arguments
INSTALL_DOCKER=false
SETUP_NETWORK=false
CREATE_DIRS=false
DO_ALL=false

if [ $# -eq 0 ]; then
    usage
fi

while [[ $# -gt 0 ]]; do
    case $1 in
        --install-docker)
            INSTALL_DOCKER=true
            shift
            ;;
        --setup-network)
            SETUP_NETWORK=true
            shift
            ;;
        --create-dirs)
            CREATE_DIRS=true
            shift
            ;;
        --all)
            DO_ALL=true
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

# If --all is specified, enable all options
if [ "$DO_ALL" = true ]; then
    INSTALL_DOCKER=true
    SETUP_NETWORK=true
    CREATE_DIRS=true
fi

# Main execution
echo ""
print_info "Arx Engine Server Setup"
echo ""

show_info

check_root

if [ "$INSTALL_DOCKER" = true ]; then
    install_docker
fi

if [ "$SETUP_NETWORK" = true ]; then
    setup_network
fi

if [ "$CREATE_DIRS" = true ]; then
    create_directories
fi

echo ""
verify_setup

echo ""
print_info "Setup complete!"
echo ""
print_info "Next steps:"
echo "  1. Configure Traefik reverse proxy (if not already done)"
echo "  2. Set up GitHub Actions secrets and variables"
echo "  3. Push to main/develop branch or run workflow manually"
echo "  4. Or use the deploy.sh script for manual deployment"
echo ""

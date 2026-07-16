#!/bin/bash

# Dejavu API Deployment Script
# This script helps with manual deployment and service management

set -e

# Configuration
APP_NAME="dejavu-transcription-api"
DEPLOY_DIR="/opt/dejavu/backend"
SERVICE_NAME="dejavu-transcription-api.service"
BINARY_PATH="$DEPLOY_DIR/target/release/$APP_NAME"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_rust() {
    log_info "Checking Rust installation..."
    if ! command -v rustc &> /dev/null; then
        log_error "Rust is not installed. Please install Rust first:"
        echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    log_success "Rust is installed: $(rustc --version)"
}

build_application() {
    log_info "Building application in release mode..."
    cd "$DEPLOY_DIR"
    
    # Clean previous builds
    cargo clean
    
    # Build with optimizations
    RUST_LOG=info cargo build --release
    
    if [ -f "$BINARY_PATH" ]; then
        log_success "Binary built successfully"
        ls -la "$BINARY_PATH"
    else
        log_error "Failed to build binary"
        exit 1
    fi
}

setup_systemd_service() {
    log_info "Setting up systemd service..."
    
    # Create systemd service file
    sudo tee /etc/systemd/system/$SERVICE_NAME > /dev/null << 'EOF'
[Unit]
Description=Dejavu API Rust Backend
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=simple
User=root
Group=root
WorkingDirectory=/opt/dejavu/backend
Environment=RUST_LOG=info
EnvironmentFile=/opt/dejavu/backend/.env
ExecStart=/opt/dejavu/backend/target/release/dejavu-transcription-api
ExecReload=/bin/kill -HUP $MAINPID
KillMode=mixed
KillSignal=SIGINT
TimeoutStopSec=5
Restart=always
RestartSec=10

# Resource limits
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF
    
    # Set proper permissions
    sudo chown -R $USER:$USER "$DEPLOY_DIR"
    chmod +x "$BINARY_PATH"
    
    # Reload systemd and enable service
    sudo systemctl daemon-reload
    sudo systemctl enable $SERVICE_NAME
    
    log_success "Systemd service configured"
}

start_service() {
    log_info "Starting $SERVICE_NAME..."
    sudo systemctl stop $SERVICE_NAME 2>/dev/null || true
    sudo systemctl start $SERVICE_NAME
    sleep 5
    
    if sudo systemctl is-active --quiet $SERVICE_NAME; then
        log_success "Service started successfully"
    else
        log_error "Service failed to start"
        show_logs
        exit 1
    fi
}

stop_service() {
    log_info "Stopping $SERVICE_NAME..."
    sudo systemctl stop $SERVICE_NAME
    log_success "Service stopped"
}

restart_service() {
    log_info "Restarting $SERVICE_NAME..."
    sudo systemctl restart $SERVICE_NAME
    sleep 5
    
    if sudo systemctl is-active --quiet $SERVICE_NAME; then
        log_success "Service restarted successfully"
    else
        log_error "Service failed to restart"
        show_logs
        exit 1
    fi
}

show_status() {
    log_info "Service status:"
    sudo systemctl status $SERVICE_NAME --no-pager -l
}

show_logs() {
    log_info "Recent logs:"
    sudo journalctl -u $SERVICE_NAME --no-pager -l -n 30
}

follow_logs() {
    log_info "Following logs (Ctrl+C to exit):"
    sudo journalctl -u $SERVICE_NAME -f
}

test_api() {
    log_info "Testing API health..."
    
    # Get port from .env file
    if [ -f "$DEPLOY_DIR/.env" ]; then
        APP_PORT=$(grep APP_PORT "$DEPLOY_DIR/.env" | cut -d'=' -f2)
    fi
    APP_PORT=${APP_PORT:-3001}
    
    sleep 3
    if curl -f "http://localhost:$APP_PORT/health" 2>/dev/null; then
        log_success "API is responding on port $APP_PORT"
    else
        log_warning "API health check failed or endpoint not available"
    fi
    
    # Show listening ports
    log_info "Listening ports:"
    sudo netstat -tlnp | grep ":$APP_PORT" || log_warning "No process found listening on port $APP_PORT"
}

run_migrations() {
    log_info "Running database migrations..."
    cd "$DEPLOY_DIR"
    
    # Check if sea-orm-cli is installed
    if ! command -v sea-orm-cli &> /dev/null; then
        log_info "Installing sea-orm-cli..."
        cargo install sea-orm-cli
    fi
    
    # Run migrations if migration files exist
    if [ -d "migration" ] || [ -d "migrations" ]; then
        sea-orm-cli migrate up || log_warning "Migration failed or not needed"
        log_success "Migrations completed"
    else
        log_warning "No migration directory found"
    fi
}

full_deploy() {
    log_info "Starting full deployment..."
    check_rust
    build_application
    setup_systemd_service
    run_migrations
    start_service
    test_api
    log_success "Deployment completed successfully!"
}

show_help() {
    echo "Dejavu API Deployment Script"
    echo ""
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  deploy      - Full deployment (build, setup, start)"
    echo "  build       - Build the application"
    echo "  start       - Start the service"
    echo "  stop        - Stop the service"
    echo "  restart     - Restart the service"
    echo "  status      - Show service status"
    echo "  logs        - Show recent logs"
    echo "  follow      - Follow logs in real-time"
    echo "  test        - Test API health"
    echo "  migrate     - Run database migrations"
    echo "  setup       - Setup systemd service only"
    echo "  help        - Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 deploy          # Full deployment"
    echo "  $0 restart         # Restart service"
    echo "  $0 logs            # View recent logs"
    echo "  $0 follow          # Follow logs"
}

# Main script logic
case "${1:-help}" in
    deploy)
        full_deploy
        ;;
    build)
        check_rust
        build_application
        ;;
    start)
        start_service
        test_api
        ;;
    stop)
        stop_service
        ;;
    restart)
        restart_service
        test_api
        ;;
    status)
        show_status
        ;;
    logs)
        show_logs
        ;;
    follow)
        follow_logs
        ;;
    test)
        test_api
        ;;
    migrate)
        run_migrations
        ;;
    setup)
        setup_systemd_service
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        log_error "Unknown command: $1"
        show_help
        exit 1
        ;;
esac

#!/usr/bin/env bash
# arcli-backend installer script
# Usage: curl -sSL https://raw.githubusercontent.com/YOUR_ORG/arcli-backend/main/install.sh | bash
# Or: wget -qO- https://raw.githubusercontent.com/YOUR_ORG/arcli-backend/main/install.sh | bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO_OWNER="${GITHUB_REPO_OWNER:-bobchevalieragility}"  # Set your GitHub org/user here
REPO_NAME="${GITHUB_REPO_NAME:-arcli-backend}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
CONFIG_DIR="${CONFIG_DIR:-$HOME/.arcli-backend}"
BINARY_NAME="backend"

# Function to print colored messages
info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Function to detect OS
detect_os() {
    local os
    case "$(uname -s)" in
        Linux*)     os="linux" ;;
        Darwin*)    os="macos" ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *)          error "Unsupported operating system: $(uname -s)" ;;
    esac
    echo "$os"
}

# Function to detect architecture
detect_arch() {
    local arch
    case "$(uname -m)" in
        x86_64|amd64)   arch="amd64" ;;
        aarch64|arm64)  arch="arm64" ;;
        *)              error "Unsupported architecture: $(uname -m)" ;;
    esac
    echo "$arch"
}

# Function to get the latest release tag
get_latest_release() {
    local latest_release

    # Try using GitHub API
    if command -v curl > /dev/null 2>&1; then
        latest_release=$(curl -sSL "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest" |
            grep '"tag_name":' |
            sed -E 's/.*"([^"]+)".*/\1/')
    elif command -v wget > /dev/null 2>&1; then
        latest_release=$(wget -qO- "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest" |
            grep '"tag_name":' |
            sed -E 's/.*"([^"]+)".*/\1/')
    else
        error "Neither curl nor wget is available. Please install one of them."
    fi

    if [ -z "$latest_release" ]; then
        error "Failed to fetch latest release information"
    fi

    echo "$latest_release"
}

# Function to download file
download_file() {
    local url="$1"
    local output="$2"

    if command -v curl > /dev/null 2>&1; then
        curl -sSL -o "$output" "$url" || error "Failed to download from $url"
    elif command -v wget > /dev/null 2>&1; then
        wget -qO "$output" "$url" || error "Failed to download from $url"
    else
        error "Neither curl nor wget is available. Please install one of them."
    fi
}

# Main installation function
main() {
    echo ""
    info "arcli-backend installer"
    echo ""

    # Detect system
    local os=$(detect_os)
    local arch=$(detect_arch)
    info "Detected system: $os-$arch"

    # Get latest release
    local version=$(get_latest_release)
    info "Latest version: $version"

    # Construct binary name based on OS and architecture
    local binary_suffix=""
    if [ "$os" = "windows" ]; then
        binary_suffix=".exe"
    fi

    local artifact_name="${BINARY_NAME}-${os}-${arch}${binary_suffix}"
    local binary_download_url="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${version}/${artifact_name}"

    info "Download URL: $binary_download_url"

    # Create install directory if it doesn't exist
    if [ ! -d "$INSTALL_DIR" ]; then
        info "Creating installation directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR" || error "Failed to create directory: $INSTALL_DIR"
    fi

    local binary_install_path="${INSTALL_DIR}/backend${binary_suffix}"

    # Download binary
    info "Downloading ${BINARY_NAME}..."
    download_file "$binary_download_url" "$binary_install_path"

    # Make binary executable (not needed on Windows)
    if [ "$os" != "windows" ]; then
        chmod +x "$binary_install_path" || error "Failed to make binary executable"
    fi

    # Create user config directory if it doesn't exist
    if [ ! -d "$CONFIG_DIR" ]; then
        info "Creating config directory: $CONFIG_DIR"
        mkdir -p "$CONFIG_DIR" || error "Failed to create directory: $CONFIG_DIR"

	# Download default config file
	local config_download_url="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${version}/config.toml"
	local config_install_path="${CONFIG_DIR}/config.toml"
	info "Downloading config.toml..."
	download_file "$config_download_url" "$config_install_path"
    fi

    # Download default config file
    local wrapper_download_url="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${version}/backend.sh"
    local wrapper_install_path="${CONFIG_DIR}/backend.sh"
    info "Downloading backend.sh..."
    download_file "$wrapper_download_url" "$wrapper_install_path"

    echo ""
    success "arcli-backend ${version} installed successfully!"
    echo ""

    # Check if install directory is in PATH
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        warning "Installation directory is not in your PATH"
        echo ""
        echo "To use ${BINARY_NAME}, either:"
        echo "  1. Add the following line to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo ""
        echo "     export PATH=\"\$PATH:$INSTALL_DIR\""
        echo ""
        echo "  2. Or use the full path: $binary_install_path"
        echo ""
        echo "After updating your shell profile, run: source ~/.bashrc (or ~/.zshrc)"
    fi

    echo -e "${GREEN}!!!IMPORTANT!!! Add the following line to your shell profile (~/.zshrc or ~/.bashrc):${NC}"
    echo "source ~/.arcli-backend/backend.sh"
}

# Run main function
main


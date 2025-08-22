#!/bin/bash
# scripts/containerManagement/update-container.sh

set -euo pipefail

NAMESPACE="media-management"
IMAGE_NAME="media-management-service"
IMAGE_TAG="latest"
FULL_IMAGE_NAME="${IMAGE_NAME}:${IMAGE_TAG}"

# Colors for better readability
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print separator
print_separator() {
    local char="${1:-─}"
    local width="${2:-$(tput cols 2>/dev/null || echo 80)}"
    printf "%*s\n" "$width" '' | tr ' ' "$char"
}

# Function to print status with color
print_status() {
    local status="$1"
    local message="$2"
    if [ "$status" = "ok" ]; then
        echo -e "✅ ${GREEN}$message${NC}"
    elif [ "$status" = "warning" ]; then
        echo -e "⚠️  ${YELLOW}$message${NC}"
    else
        echo -e "❌ ${RED}$message${NC}"
    fi
}

echo "🔄 Updating Media Management Service container..."
print_separator "="

echo -e "${CYAN}🦀 Building new Rust Docker image...${NC}"
eval "$(minikube docker-env)"
docker build -t "$FULL_IMAGE_NAME" .
print_status "ok" "Docker image built successfully"

print_separator
echo -e "${CYAN}🔄 Restarting deployment to pick up new image...${NC}"
kubectl rollout restart deployment/media-management-service -n "$NAMESPACE"

print_separator
echo -e "${CYAN}⏳ Waiting for rollout to complete...${NC}"
kubectl rollout status deployment/media-management-service -n "$NAMESPACE" --timeout=120s

print_separator "="
print_status "ok" "Media Management Service updated successfully"
echo -e "${CYAN}🌍 Access at: http://media-management.local/api/v1/media-management/health${NC}"

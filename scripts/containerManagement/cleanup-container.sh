#!/bin/bash
# scripts/containerManagement/cleanup-container.sh

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

echo "🧹 Cleaning up Media Management Service resources..."
print_separator "="

# Check if minikube is running
if ! minikube status >/dev/null 2>&1; then
    print_status "error" "Minikube is not running. Please start it first with: minikube start"
    exit 1
fi
print_status "ok" "Minikube is running"

print_separator
echo -e "${CYAN}🛑 Deleting deployment...${NC}"
kubectl delete deployment media-management-service -n "$NAMESPACE" --ignore-not-found
print_status "ok" "Deployment deletion completed"

print_separator
echo -e "${CYAN}🌐 Deleting service...${NC}"
kubectl delete service media-management-service -n "$NAMESPACE" --ignore-not-found
print_status "ok" "Service deletion completed"

print_separator
echo -e "${CYAN}📥 Deleting ingress...${NC}"
kubectl delete ingress media-management-ingress -n "$NAMESPACE" --ignore-not-found
print_status "ok" "Ingress deletion completed"

print_separator
echo -e "${CYAN}🔒 Deleting network policy...${NC}"
kubectl delete networkpolicy media-management-network-policy -n "$NAMESPACE" --ignore-not-found
print_status "ok" "Network policy deletion completed"

print_separator
echo -e "${CYAN}🛡️  Deleting pod disruption budget...${NC}"
kubectl delete poddisruptionbudget media-management-pdb -n "$NAMESPACE" --ignore-not-found
print_status "ok" "Pod disruption budget deletion completed"

print_separator
echo -e "${CYAN}⚙️  Deleting configmap...${NC}"
kubectl delete configmap media-management-config -n "$NAMESPACE" --ignore-not-found
print_status "ok" "ConfigMap deletion completed"

print_separator
echo -e "${CYAN}🔐 Deleting secret...${NC}"
kubectl delete secret media-management-secrets -n "$NAMESPACE" --ignore-not-found
print_status "ok" "Secret deletion completed"

print_separator
echo -e "${CYAN}📂 Deleting namespace...${NC}"
kubectl delete namespace "$NAMESPACE" --ignore-not-found
print_status "ok" "Namespace deletion completed"

print_separator
echo -e "${CYAN}🔗 Removing /etc/hosts entry...${NC}"
if grep -q "media-management.local" /etc/hosts; then
  sed -i "/media-management.local/d" /etc/hosts
  print_status "ok" "Removed media-management.local from /etc/hosts"
else
  print_status "ok" "/etc/hosts entry was not found"
fi

print_separator
echo -e "${CYAN}🐳 Cleaning up Docker image...${NC}"
eval "$(minikube docker-env)"

# More robust image detection
IMAGE_COUNT=$(docker images --format "{{.Repository}}:{{.Tag}}" | grep -c "^${FULL_IMAGE_NAME}$" || echo "0")
echo "🔍 Found $IMAGE_COUNT matching images for $FULL_IMAGE_NAME"

if [ "$IMAGE_COUNT" -gt 0 ]; then
  echo -e "${YELLOW}🗑️  Removing Docker image: $FULL_IMAGE_NAME${NC}"
  if docker rmi "$FULL_IMAGE_NAME"; then
    print_status "ok" "Docker image deleted successfully"
  else
    print_status "warning" "Failed to delete Docker image (may be in use)"
  fi
else
  print_status "ok" "Docker image was not found (already cleaned up)"
fi

print_separator "="
print_status "ok" "Cleanup completed successfully"

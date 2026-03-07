#!/bin/bash
# scripts/containerManagement/cleanup-container.sh
# Delete all media-management-service resources from minikube

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_common.sh"

# Parse flags
REMOVE_IMAGE=false
for arg in "$@"; do
  case "$arg" in
  --remove-image) REMOVE_IMAGE=true ;;
  esac
done

print_separator "="
echo -e "${CYAN}Cleaning up media-management-service...${NC}"
print_separator "="

# Delete namespace (removes all resources within it)
print_separator "-"
echo -e "${CYAN}Deleting namespace '$NAMESPACE'...${NC}"
if kubectl get namespace "$NAMESPACE" >/dev/null 2>&1; then
  kubectl delete namespace "$NAMESPACE" --wait=true
  print_status "ok" "Namespace '$NAMESPACE' deleted"
else
  print_status "warning" "Namespace '$NAMESPACE' does not exist"
fi

# Optionally remove image
if $REMOVE_IMAGE; then
  print_separator "-"
  echo -e "${CYAN}Removing image from Minikube...${NC}"
  minikube ssh "docker rmi -f $FULL_IMAGE" 2>/dev/null || true
  print_status "ok" "Image removed from Minikube"
fi

print_separator "="
print_status "ok" "Cleanup complete!"
echo ""
echo "To redeploy: ./scripts/containerManagement/deploy-container.sh"
echo ""
echo "Options:"
echo "  --remove-image  Also remove the Docker image from Minikube"

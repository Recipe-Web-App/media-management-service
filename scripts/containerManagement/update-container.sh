#!/bin/bash
# scripts/containerManagement/update-container.sh
# Rebuild and redeploy media-management-service in minikube

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_common.sh"
navigate_to_project_root
start_timer

# Parse flags
ROLLBACK_ON_FAILURE=false
for arg in "$@"; do
  case "$arg" in
  --rollback-on-failure) ROLLBACK_ON_FAILURE=true ;;
  esac
done

print_separator "="
echo -e "${CYAN}Updating media-management-service...${NC}"
print_separator "="

# --- Check deployment exists ---
if ! check_namespace_exists; then
  print_status "error" "Namespace '$NAMESPACE' does not exist. Run deploy-container.sh first."
  exit 1
fi

# --- Record current revision for rollback ---
CURRENT_REVISION=$(kubectl rollout history "deployment/$SERVICE_NAME" -n "$NAMESPACE" \
  -o jsonpath='{.metadata.generation}' 2>/dev/null || echo "unknown")
print_status "info" "Current deployment revision: $CURRENT_REVISION"

# --- Build new image ---
print_separator "-"
echo -e "${CYAN}Rebuilding Docker image: ${FULL_IMAGE}${NC}"
docker build -t "$FULL_IMAGE" .
print_status "ok" "Docker image rebuilt"

# --- Load into minikube ---
print_separator "-"
echo -e "${CYAN}Removing old image from Minikube...${NC}"
minikube ssh "docker rmi -f $FULL_IMAGE" 2>/dev/null || true
print_status "ok" "Old image removed"

print_separator "-"
echo -e "${CYAN}Loading new image into Minikube...${NC}"
minikube image load "$FULL_IMAGE"
print_status "ok" "New image loaded"

# --- Rollout restart ---
print_separator "-"
echo -e "${CYAN}Triggering rollout restart...${NC}"
kubectl rollout restart "deployment/$SERVICE_NAME" -n "$NAMESPACE"
print_status "ok" "Rollout restart triggered"

# --- Wait for rollout ---
print_separator "-"
echo -e "${CYAN}Waiting for rollout to complete...${NC}"
if ! kubectl rollout status "deployment/$SERVICE_NAME" -n "$NAMESPACE" --timeout=120s; then
  print_status "error" "Rollout failed!"
  show_failure_diagnostics

  if $ROLLBACK_ON_FAILURE; then
    print_separator "-"
    echo -e "${YELLOW}Rolling back to previous revision...${NC}"
    kubectl rollout undo "deployment/$SERVICE_NAME" -n "$NAMESPACE"
    kubectl rollout status "deployment/$SERVICE_NAME" -n "$NAMESPACE" --timeout=60s || true
    print_status "warning" "Rolled back to previous revision"
  else
    echo ""
    echo "  To rollback manually:"
    echo "    kubectl rollout undo deployment/$SERVICE_NAME -n $NAMESPACE"
  fi
  show_elapsed
  exit 1
fi

# --- Status ---
print_separator "="
echo -e "${CYAN}Current Pod Status:${NC}"
print_separator "-"
kubectl get pods -n "$NAMESPACE"

print_separator "="
print_status "ok" "Update complete!"
show_elapsed

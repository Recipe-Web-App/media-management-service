#!/bin/bash
# scripts/containerManagement/deploy-container.sh
# Deploy media-management-service to minikube using Kustomize

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_common.sh"
navigate_to_project_root
start_timer
enable_error_trap

# Parse flags
DRY_RUN=false
for arg in "$@"; do
  case "$arg" in
  --dry-run) DRY_RUN=true ;;
  esac
done

print_separator "="
echo -e "${CYAN}Deploying media-management-service to Minikube${NC}"
print_separator "="

# --- Prerequisites ---
print_separator "-"
echo -e "${CYAN}Checking prerequisites...${NC}"
check_prerequisites

# --- Minikube ---
print_separator "-"
echo -e "${CYAN}Checking Minikube status...${NC}"
ensure_minikube_running

# --- Dry run ---
if $DRY_RUN; then
  print_separator "-"
  echo -e "${CYAN}Dry run — previewing manifests...${NC}"
  kustomize build "$OVERLAY_PATH" | kubectl apply --dry-run=client -f -
  print_separator "="
  print_status "ok" "Dry run complete (no changes applied)"
  show_elapsed
  exit 0
fi

# --- Build Docker image ---
print_separator "-"
echo -e "${CYAN}Building Docker image in Minikube: ${FULL_IMAGE}${NC}"
use_minikube_docker
docker build -t "$FULL_IMAGE" .
print_status "ok" "Docker image built in Minikube"
unset_minikube_docker

# --- Clean deploy (delete existing namespace) ---
print_separator "-"
echo -e "${CYAN}Preparing namespace...${NC}"
kubectl delete namespace "$NAMESPACE" --ignore-not-found --wait=true 2>/dev/null || true
ensure_namespace

# --- Create secrets from .env.secrets ---
print_separator "-"
echo -e "${CYAN}Configuring secrets...${NC}"
create_secrets

# --- Apply manifests ---
print_separator "-"
echo -e "${CYAN}Applying Kustomize manifests...${NC}"
kustomize build "$OVERLAY_PATH" | kubectl apply -f -
print_status "ok" "Kustomize manifests applied"

# --- Check PVC ---
print_separator "-"
echo -e "${CYAN}Checking storage...${NC}"
sleep 5
check_pvc_status

# --- Wait for ready ---
print_separator "-"
echo -e "${CYAN}Waiting for pods to be ready...${NC}"
wait_for_ready 120

# --- Status ---
print_separator "="
echo -e "${CYAN}Deployment Status:${NC}"
print_separator "-"
kubectl get pods -n "$NAMESPACE"

print_separator "="
echo -e "${CYAN}Access Information:${NC}"
print_separator "-"
print_access_info

print_separator "="
print_status "ok" "Deployment complete!"
show_elapsed

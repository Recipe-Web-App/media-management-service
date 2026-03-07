#!/bin/bash
# scripts/containerManagement/_common.sh
# Shared constants and utilities for all deployment management scripts

set -euo pipefail

# --- Constants ---
readonly SERVICE_NAME="media-management-service"
readonly NAMESPACE="media-management"
readonly IMAGE_NAME="media-management-service"
readonly IMAGE_TAG="dev"
readonly FULL_IMAGE="${IMAGE_NAME}:${IMAGE_TAG}"
export OVERLAY_PATH="k8s/overlays/local"
readonly HEALTH_ENDPOINT="/api/v1/media-management/health"
readonly READY_ENDPOINT="/api/v1/media-management/ready"
readonly SERVICE_PORT=3000
readonly SECRET_NAME="media-management-secrets"
export DEFAULT_REPLICAS=1

# --- Terminal formatting ---
COLUMNS=$(tput cols 2>/dev/null || echo 80)
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly CYAN='\033[0;36m'
readonly NC='\033[0m'

print_separator() {
  local char="${1:-=}"
  local width="${COLUMNS:-80}"
  printf '%*s\n' "$width" '' | tr ' ' "$char"
}

print_status() {
  local status="$1"
  local message="$2"
  case "$status" in
  ok) echo -e "${GREEN}[OK]${NC} $message" ;;
  warning) echo -e "${YELLOW}[WARN]${NC} $message" ;;
  error) echo -e "${RED}[ERROR]${NC} $message" ;;
  info) echo -e "${CYAN}[INFO]${NC} $message" ;;
  esac
}

# --- Utilities ---
command_exists() {
  command -v "$1" >/dev/null 2>&1
}

navigate_to_project_root() {
  local script_dir
  script_dir="$(cd "$(dirname "${BASH_SOURCE[1]}")" && pwd)"
  PROJECT_ROOT="$(cd "$script_dir/../.." && pwd)"
  cd "$PROJECT_ROOT"
}

# --- Timing ---
TIMER_START=0

start_timer() {
  TIMER_START=$(date +%s)
}

show_elapsed() {
  local elapsed=$(($(date +%s) - TIMER_START))
  local min=$((elapsed / 60))
  local sec=$((elapsed % 60))
  print_status "info" "Elapsed: ${min}m ${sec}s"
}

# --- Error handling ---
show_failure_diagnostics() {
  print_separator "-"
  echo -e "${YELLOW}Diagnostics:${NC}"

  echo -e "${CYAN}Deployment describe (last 20 lines):${NC}"
  kubectl describe "deployment/$SERVICE_NAME" -n "$NAMESPACE" 2>/dev/null | tail -20 || true
  echo ""

  echo -e "${CYAN}Pod status:${NC}"
  kubectl get pods -n "$NAMESPACE" -l "app=$SERVICE_NAME" 2>/dev/null || true
  echo ""

  echo -e "${CYAN}Recent pod logs:${NC}"
  local pod
  pod=$(kubectl get pods -n "$NAMESPACE" -l "app=$SERVICE_NAME" \
    -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")
  if [ -n "$pod" ]; then
    kubectl logs -n "$NAMESPACE" "$pod" --tail=20 2>/dev/null || true
  else
    echo "  No pods found"
  fi
}

on_error() {
  local exit_code=$?
  local line_no=$1
  print_status "error" "Command failed at line $line_no (exit code: $exit_code)"
  if kubectl get namespace "$NAMESPACE" &>/dev/null; then
    show_failure_diagnostics
  fi
}

enable_error_trap() {
  trap 'on_error $LINENO' ERR
}

# --- Prerequisite checks ---
check_prerequisites() {
  local required_cmds=("minikube" "kubectl" "docker" "kustomize")
  local all_ok=true

  for cmd in "${required_cmds[@]}"; do
    if command_exists "$cmd"; then
      print_status "ok" "$cmd is installed"
    else
      print_status "error" "$cmd is not installed"
      all_ok=false
    fi
  done

  if ! $all_ok; then
    echo "Please resolve the above issues before proceeding."
    exit 1
  fi
}

# --- Minikube helpers ---
ensure_minikube_running() {
  if ! minikube status >/dev/null 2>&1; then
    echo -e "${YELLOW}Starting Minikube...${NC}"
    minikube start
  else
    print_status "ok" "Minikube is already running"
  fi
}

use_minikube_docker() {
  eval "$(minikube docker-env)"
}

unset_minikube_docker() {
  eval "$(minikube docker-env --unset)"
}

# --- Kubernetes helpers ---
ensure_namespace() {
  if kubectl get namespace "$NAMESPACE" >/dev/null 2>&1; then
    print_status "ok" "Namespace '$NAMESPACE' already exists"
  else
    kubectl create namespace "$NAMESPACE"
    print_status "ok" "Namespace '$NAMESPACE' created"
  fi
}

check_namespace_exists() {
  if ! kubectl get namespace "$NAMESPACE" >/dev/null 2>&1; then
    return 1
  fi
  return 0
}

wait_for_ready() {
  local timeout="${1:-120}"
  kubectl wait --namespace="$NAMESPACE" \
    --for=condition=Available "deployment/$SERVICE_NAME" \
    --timeout="${timeout}s"
}

# --- Health checks ---
check_pvc_status() {
  local pvc_status
  pvc_status=$(kubectl get pvc -n "$NAMESPACE" \
    -o jsonpath='{.items[0].status.phase}' 2>/dev/null || echo "Unknown")
  if [ "$pvc_status" = "Bound" ]; then
    print_status "ok" "PVC is Bound"
  else
    print_status "warning" "PVC status: $pvc_status (may need a StorageClass or provisioner)"
  fi
}

check_secrets() {
  if kubectl get secret "$SECRET_NAME" -n "$NAMESPACE" &>/dev/null; then
    print_status "ok" "Secret '$SECRET_NAME' exists"
  else
    print_status "warning" "Secret '$SECRET_NAME' not found — DB password and auth credentials will be missing"
    echo "  Create .env.secrets from .env.secrets.example and re-run deploy"
  fi
}

create_secrets() {
  local secrets_file="$PROJECT_ROOT/.env.secrets"
  if [ ! -f "$secrets_file" ]; then
    print_status "error" ".env.secrets not found"
    echo "  Copy the example and fill in real values:"
    echo "    cp .env.secrets.example .env.secrets"
    exit 1
  fi

  # Build --from-literal args from .env.secrets (skip comments and blank lines)
  local -a secret_args=()
  while IFS='=' read -r key value; do
    [[ -z "$key" || "$key" == \#* ]] && continue
    secret_args+=("--from-literal=${key}=${value}")
  done <"$secrets_file"

  if [ ${#secret_args[@]} -eq 0 ]; then
    print_status "error" ".env.secrets contains no key=value pairs"
    exit 1
  fi

  kubectl create secret generic "$SECRET_NAME" -n "$NAMESPACE" "${secret_args[@]}" --dry-run=client -o yaml \
    | kubectl apply -f -
  print_status "ok" "Secret '$SECRET_NAME' created from .env.secrets"
}

# --- Access info ---
print_access_info() {
  echo "  Namespace: $NAMESPACE"
  echo "  Image: $FULL_IMAGE"
  echo ""
  echo "  Port forward to access the service:"
  echo "    kubectl port-forward -n $NAMESPACE svc/$SERVICE_NAME $SERVICE_PORT:$SERVICE_PORT"
  echo ""
  echo "  Then access:"
  echo "    Health:    http://localhost:$SERVICE_PORT$HEALTH_ENDPOINT"
  echo "    Readiness: http://localhost:$SERVICE_PORT$READY_ENDPOINT"
  echo ""
  echo "  View logs:"
  echo "    kubectl logs -n $NAMESPACE -l app=$SERVICE_NAME -f"
}

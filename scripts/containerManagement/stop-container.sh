#!/bin/bash
# scripts/containerManagement/stop-container.sh
# Scale down media-management-service deployment to 0

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_common.sh"

print_separator "="
echo -e "${CYAN}Stopping media-management-service...${NC}"
print_separator "="

# Check namespace
if ! check_namespace_exists; then
  print_status "warning" "Namespace '$NAMESPACE' does not exist. Nothing to stop."
  exit 0
fi

# Scale down
print_separator "-"
echo -e "${CYAN}Scaling deployment to 0...${NC}"
kubectl scale "deployment/$SERVICE_NAME" -n "$NAMESPACE" --replicas=0
print_status "ok" "Scale down command issued"

# Wait for termination
print_separator "-"
echo -e "${CYAN}Waiting for pods to terminate...${NC}"
kubectl wait --namespace="$NAMESPACE" \
  --for=delete pod \
  --selector="app=$SERVICE_NAME" \
  --timeout=60s 2>/dev/null || true

# Status
print_separator "-"
echo -e "${CYAN}Current Pod Status:${NC}"
kubectl get pods -n "$NAMESPACE" 2>/dev/null || echo "No pods running"

print_separator "="
print_status "ok" "Containers stopped!"
echo ""
echo "To start again: ./scripts/containerManagement/start-container.sh"

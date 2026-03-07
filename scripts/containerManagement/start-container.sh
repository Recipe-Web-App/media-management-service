#!/bin/bash
# scripts/containerManagement/start-container.sh
# Scale up media-management-service deployment

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_common.sh"

REPLICAS="${1:-$DEFAULT_REPLICAS}"

print_separator "="
echo -e "${CYAN}Starting media-management-service...${NC}"
print_separator "="

# Check namespace
if ! check_namespace_exists; then
  print_status "error" "Namespace '$NAMESPACE' does not exist. Run deploy-container.sh first."
  exit 1
fi

# Scale up
print_separator "-"
echo -e "${CYAN}Scaling deployment to $REPLICAS replica(s)...${NC}"
kubectl scale "deployment/$SERVICE_NAME" -n "$NAMESPACE" --replicas="$REPLICAS"
print_status "ok" "Scale command issued"

# Wait for ready
print_separator "-"
echo -e "${CYAN}Waiting for pods to be ready...${NC}"
wait_for_ready 90

# Status
print_separator "-"
echo -e "${CYAN}Current Pod Status:${NC}"
kubectl get pods -n "$NAMESPACE"

print_separator "="
echo -e "${CYAN}Access Information:${NC}"
print_separator "-"
print_access_info

print_separator "="
print_status "ok" "Containers started!"
echo ""
echo "Usage: $0 [replicas]  (default: $DEFAULT_REPLICAS)"

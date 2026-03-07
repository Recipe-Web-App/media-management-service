#!/bin/bash
# scripts/containerManagement/get-container-status.sh
# Display comprehensive status of media-management-service deployment

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_common.sh"

print_separator "="
echo -e "${CYAN}Media Management Service - Status Dashboard${NC}"
print_separator "="

# Prerequisites
print_separator "-"
echo -e "${CYAN}Prerequisites:${NC}"
for cmd in kubectl minikube docker kustomize jq; do
  if command_exists "$cmd"; then
    print_status "ok" "$cmd is available"
  else
    print_status "warning" "$cmd is not installed"
  fi
done

# Minikube status
print_separator "-"
echo -e "${CYAN}Minikube Status:${NC}"
if minikube status >/dev/null 2>&1; then
  print_status "ok" "Minikube is running"
  MINIKUBE_IP=$(minikube ip 2>/dev/null || echo "N/A")
  echo "  IP: $MINIKUBE_IP"
else
  print_status "error" "Minikube is not running"
  exit 1
fi

# Namespace
print_separator "-"
echo -e "${CYAN}Namespace Status:${NC}"
if ! check_namespace_exists; then
  print_status "error" "Namespace '$NAMESPACE' does not exist"
  echo ""
  echo "Run ./scripts/containerManagement/deploy-container.sh to deploy"
  exit 1
fi
print_status "ok" "Namespace '$NAMESPACE' exists"

# Deployments
print_separator "-"
echo -e "${CYAN}Deployments:${NC}"
kubectl get deployments -n "$NAMESPACE" -o wide 2>/dev/null || echo "No deployments found"

# Pods
print_separator "-"
echo -e "${CYAN}Pods:${NC}"
kubectl get pods -n "$NAMESPACE" -o wide 2>/dev/null || echo "No pods found"

# Pod restart counts
print_separator "-"
echo -e "${CYAN}Pod Restart Counts:${NC}"
kubectl get pods -n "$NAMESPACE" \
  -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{range .status.containerStatuses[*]}{.restartCount}{" restarts"}{end}{"\n"}{end}' \
  2>/dev/null || echo "Unable to get restart counts"

# Services
print_separator "-"
echo -e "${CYAN}Services:${NC}"
kubectl get services -n "$NAMESPACE" 2>/dev/null || echo "No services found"

# ConfigMaps
print_separator "-"
echo -e "${CYAN}ConfigMaps:${NC}"
kubectl get configmaps -n "$NAMESPACE" 2>/dev/null || echo "No configmaps found"

# Secrets
print_separator "-"
echo -e "${CYAN}Secrets:${NC}"
kubectl get secrets -n "$NAMESPACE" 2>/dev/null || echo "No secrets found"

# PVCs
print_separator "-"
echo -e "${CYAN}Persistent Volume Claims:${NC}"
kubectl get pvc -n "$NAMESPACE" 2>/dev/null || echo "No PVCs found"
check_pvc_status

# Health check via kubectl exec
print_separator "-"
echo -e "${CYAN}Health Check (via kubectl exec):${NC}"

API_POD=$(kubectl get pods -n "$NAMESPACE" -l "app=$SERVICE_NAME" \
  -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")

if [ -n "$API_POD" ]; then
  echo "  Testing $HEALTH_ENDPOINT..."
  HEALTH_RESPONSE=$(kubectl exec -n "$NAMESPACE" "$API_POD" -- \
    curl -s "http://localhost:${SERVICE_PORT}${HEALTH_ENDPOINT}" 2>/dev/null || echo '{"error": "failed"}')

  if command_exists jq; then
    echo "$HEALTH_RESPONSE" | jq -r '
      if .status then
        "  Status: \(.status)"
      else
        "  Error: \(.error // "Unknown error")"
      end
    ' 2>/dev/null || echo "  Response: $HEALTH_RESPONSE"
  else
    echo "  Response: $HEALTH_RESPONSE"
  fi

  echo ""
  echo "  Testing $READY_ENDPOINT..."
  READY_RESPONSE=$(kubectl exec -n "$NAMESPACE" "$API_POD" -- \
    curl -s "http://localhost:${SERVICE_PORT}${READY_ENDPOINT}" 2>/dev/null || echo '{"error": "failed"}')

  if command_exists jq; then
    echo "$READY_RESPONSE" | jq -r '
      if .status then
        "  Status: \(.status)"
      else
        "  Error: \(.error // "Unknown error")"
      end
    ' 2>/dev/null || echo "  Response: $READY_RESPONSE"
  else
    echo "  Response: $READY_RESPONSE"
  fi
else
  print_status "warning" "No pod available for health check"
fi

# Recent events
print_separator "-"
echo -e "${CYAN}Recent Events (last 10):${NC}"
kubectl get events -n "$NAMESPACE" --sort-by='.lastTimestamp' 2>/dev/null | tail -10 || echo "No events found"

# Recent logs
print_separator "-"
echo -e "${CYAN}Recent Logs (last 10 lines):${NC}"
kubectl logs -n "$NAMESPACE" -l "app=$SERVICE_NAME" --tail=10 2>/dev/null || echo "No logs available"

# Quick commands
print_separator "="
echo -e "${CYAN}Quick Commands:${NC}"
echo "  Port forward:  kubectl port-forward -n $NAMESPACE svc/$SERVICE_NAME $SERVICE_PORT:$SERVICE_PORT"
echo "  Follow logs:   kubectl logs -n $NAMESPACE -l app=$SERVICE_NAME -f"
echo "  Exec into pod: kubectl exec -n $NAMESPACE -it \$(kubectl get pods -n $NAMESPACE -l app=$SERVICE_NAME -o jsonpath='{.items[0].metadata.name}') -- /bin/bash"
print_separator "="

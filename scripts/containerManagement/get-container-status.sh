#!/bin/bash
# scripts/containerManagement/get-container-status.sh

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

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
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

# Function to print separator
print_separator() {
    local char="${1:-─}"
    local width="${2:-$(tput cols 2>/dev/null || echo 80)}"
    printf "%*s\n" "$width" '' | tr ' ' "$char"
}

echo "📊 Media Management Service Status"
print_separator "="

# Check prerequisites
echo ""
echo -e "${CYAN}🔧 Prerequisites Check:${NC}"
if ! command_exists kubectl; then
    print_status "error" "kubectl is not installed or not in PATH"
    exit 1
else
    print_status "ok" "kubectl is available"
fi

if ! command_exists minikube; then
    print_status "warning" "minikube is not installed (may not be needed for remote clusters)"
else
    if minikube status >/dev/null 2>&1; then
        print_status "ok" "minikube is running"
    else
        print_status "warning" "minikube is not running"
    fi
fi

print_separator
echo ""
echo -e "${CYAN}🔍 Namespace Status:${NC}"
if kubectl get namespace "$NAMESPACE" >/dev/null 2>&1; then
    print_status "ok" "Namespace '$NAMESPACE' exists"
    # Get namespace details
    NAMESPACE_AGE=$(kubectl get namespace "$NAMESPACE" -o jsonpath='{.metadata.creationTimestamp}' | xargs -I {} date -d {} "+%Y-%m-%d %H:%M:%S" 2>/dev/null || echo "unknown")
    RESOURCE_COUNT=$(kubectl get all -n "$NAMESPACE" --no-headers 2>/dev/null | wc -l || echo "unknown")
    echo "   📅 Created: $NAMESPACE_AGE, Resources: $RESOURCE_COUNT"
else
    print_status "error" "Namespace '$NAMESPACE' does not exist"
    echo -e "${YELLOW}💡 Run ./scripts/containerManagement/deploy-container.sh to deploy${NC}"
    exit 1
fi

print_separator
echo ""
echo -e "${CYAN}📦 Deployment Status:${NC}"
if kubectl get deployment media-management-service -n "$NAMESPACE" >/dev/null 2>&1; then
    kubectl get deployment media-management-service -n "$NAMESPACE"

    # Check deployment readiness
    READY_REPLICAS=$(kubectl get deployment media-management-service -n "$NAMESPACE" -o jsonpath='{.status.readyReplicas}' 2>/dev/null || echo "0")
    DESIRED_REPLICAS=$(kubectl get deployment media-management-service -n "$NAMESPACE" -o jsonpath='{.spec.replicas}' 2>/dev/null || echo "0")

    if [ "$READY_REPLICAS" = "$DESIRED_REPLICAS" ] && [ "$READY_REPLICAS" != "0" ]; then
        print_status "ok" "Deployment is ready ($READY_REPLICAS/$DESIRED_REPLICAS replicas)"
    else
        print_status "warning" "Deployment not fully ready ($READY_REPLICAS/$DESIRED_REPLICAS replicas)"
    fi
else
    print_status "error" "Deployment not found"
fi

print_separator
echo ""
echo -e "${CYAN}🎯 Pods Status:${NC}"
if kubectl get pods -l app=media-management-service -n "$NAMESPACE" >/dev/null 2>&1; then
    kubectl get pods -l app=media-management-service -n "$NAMESPACE"

    # Check pod health
    POD_STATUS=$(kubectl get pods -l app=media-management-service -n "$NAMESPACE" -o jsonpath='{.items[0].status.phase}' 2>/dev/null || echo "Unknown")
    if [ "$POD_STATUS" = "Running" ]; then
        print_status "ok" "Pod is running"

        # Check readiness
        READY=$(kubectl get pods -l app=media-management-service -n "$NAMESPACE" -o jsonpath='{.items[0].status.conditions[?(@.type=="Ready")].status}' 2>/dev/null || echo "Unknown")
        if [ "$READY" = "True" ]; then
            print_status "ok" "Pod is ready"
        else
            print_status "warning" "Pod is not ready"
        fi
    else
        print_status "warning" "Pod status: $POD_STATUS"
    fi
else
    print_status "error" "No pods found"
fi

print_separator
echo ""
echo -e "${CYAN}🌐 Service Status:${NC}"
if kubectl get service media-management-service -n "$NAMESPACE" >/dev/null 2>&1; then
    kubectl get service media-management-service -n "$NAMESPACE"
    print_status "ok" "Service exists"
    # Get service details
    SERVICE_TYPE=$(kubectl get service media-management-service -n "$NAMESPACE" -o jsonpath='{.spec.type}' 2>/dev/null || echo "unknown")
    CLUSTER_IP=$(kubectl get service media-management-service -n "$NAMESPACE" -o jsonpath='{.spec.clusterIP}' 2>/dev/null || echo "unknown")
    PORT=$(kubectl get service media-management-service -n "$NAMESPACE" -o jsonpath='{.spec.ports[0].port}' 2>/dev/null || echo "unknown")
    echo "   🌐 Type: $SERVICE_TYPE, Cluster IP: $CLUSTER_IP, Port: $PORT"
else
    print_status "error" "Service not found"
fi

print_separator
echo ""
echo -e "${CYAN}📥 Ingress Status:${NC}"
if kubectl get ingress media-management-ingress -n "$NAMESPACE" >/dev/null 2>&1; then
    kubectl get ingress media-management-ingress -n "$NAMESPACE"
    print_status "ok" "Ingress exists"
    # Get ingress details
    INGRESS_HOST=$(kubectl get ingress media-management-ingress -n "$NAMESPACE" -o jsonpath='{.spec.rules[0].host}' 2>/dev/null || echo "unknown")
    INGRESS_IP=$(kubectl get ingress media-management-ingress -n "$NAMESPACE" -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || echo "unknown")
    INGRESS_CLASS=$(kubectl get ingress media-management-ingress -n "$NAMESPACE" -o jsonpath='{.spec.ingressClassName}' 2>/dev/null || echo "unknown")
    echo "   🌍 Host: $INGRESS_HOST, IP: $INGRESS_IP, Class: $INGRESS_CLASS"

    # Show detailed ingress rules
    echo "   📋 Ingress rules:"
    RULE_COUNT=$(kubectl get ingress media-management-ingress -n "$NAMESPACE" -o jsonpath='{.spec.rules}' | grep -o '"host"' | wc -l 2>/dev/null || echo "0")
    if [ "$RULE_COUNT" -gt 0 ]; then
        kubectl get ingress media-management-ingress -n "$NAMESPACE" -o jsonpath='{range .spec.rules[*]}      Rule: Host={.host}{"\n"}{end}' 2>/dev/null || echo "      Unable to parse rules"
    else
        echo "      Rule: Host=* (catch-all)"
    fi

    # Show path mappings
    echo "   🛤️  Path mappings:"
    kubectl get ingress media-management-ingress -n "$NAMESPACE" -o jsonpath='{range .spec.rules[*].http.paths[*]}      {.path} -> {.backend.service.name}:{.backend.service.port.number}{.backend.service.port.name} ({.pathType}){"\n"}{end}' 2>/dev/null || echo "      Unable to parse path mappings"

    # Show TLS configuration if present
    TLS_HOSTS=$(kubectl get ingress media-management-ingress -n "$NAMESPACE" -o jsonpath='{.spec.tls[*].hosts[*]}' 2>/dev/null || echo "")
    if [ -n "$TLS_HOSTS" ]; then
        echo "   🔒 TLS enabled for hosts: $TLS_HOSTS"
        TLS_SECRET=$(kubectl get ingress media-management-ingress -n "$NAMESPACE" -o jsonpath='{.spec.tls[0].secretName}' 2>/dev/null || echo "unknown")
        echo "   🔐 TLS secret: $TLS_SECRET"
    else
        echo "   ⚠️  TLS not configured"
    fi
else
    print_status "error" "Ingress not found"
fi

print_separator
echo ""
echo -e "${CYAN}⚙️  ConfigMap Status:${NC}"
if kubectl get configmap media-management-config -n "$NAMESPACE" >/dev/null 2>&1; then
    print_status "ok" "ConfigMap exists"
    # Show key count
    KEY_COUNT=$(kubectl get configmap media-management-config -n "$NAMESPACE" -o jsonpath='{.data}' | jq -r 'keys | length' 2>/dev/null || echo "unknown")
    echo "   📋 Configuration keys: $KEY_COUNT"
    echo "   📄 ConfigMap contents:"
    kubectl get configmap media-management-config -n "$NAMESPACE" -o jsonpath='{.data}' | jq -r 'to_entries[] | "      \(.key): \(.value)"' 2>/dev/null || echo "      Unable to parse config data"
else
    print_status "error" "ConfigMap not found"
fi

print_separator
echo ""
echo -e "${CYAN}🔐 Secret Status:${NC}"
if kubectl get secret media-management-secrets -n "$NAMESPACE" >/dev/null 2>&1; then
    print_status "ok" "Secret exists"
    # Show key count and names (without revealing values)
    KEY_COUNT=$(kubectl get secret media-management-secrets -n "$NAMESPACE" -o jsonpath='{.data}' | jq -r 'keys | length' 2>/dev/null || echo "unknown")
    echo "   🔑 Secret keys: $KEY_COUNT"
    echo "   🔐 Secret key names:"
    kubectl get secret media-management-secrets -n "$NAMESPACE" -o jsonpath='{.data}' | jq -r 'keys[]' 2>/dev/null | sed 's/^/      /' || echo "      Unable to parse secret keys"
    # Show secret metadata
    SECRET_TYPE=$(kubectl get secret media-management-secrets -n "$NAMESPACE" -o jsonpath='{.type}' 2>/dev/null || echo "unknown")
    echo "   📝 Secret type: $SECRET_TYPE"
else
    print_status "error" "Secret not found"
fi

print_separator
echo ""
echo -e "${CYAN}🔒 Network Policy Status:${NC}"
if kubectl get networkpolicy media-management-network-policy -n "$NAMESPACE" >/dev/null 2>&1; then
    print_status "ok" "Network Policy exists"

    # Show pod selector
    POD_SELECTOR=$(kubectl get networkpolicy media-management-network-policy -n "$NAMESPACE" -o jsonpath='{.spec.podSelector.matchLabels}' 2>/dev/null || echo "{}")
    echo "   🎯 Applies to pods: $POD_SELECTOR"

    # Show ingress rules details
    echo "   📥 Ingress rules (incoming traffic):"
    kubectl get networkpolicy media-management-network-policy -n "$NAMESPACE" -o jsonpath='{range .spec.ingress[*]}      - Port: {.ports[0].port}/{.ports[0].protocol} from {.from[0].namespaceSelector.matchLabels.name}{.from[0].podSelector.matchLabels.app}{"\n"}{end}' 2>/dev/null || echo "      Unable to parse ingress rules"

    # Show egress rules details
    echo "   📤 Egress rules (outgoing traffic):"
    kubectl get networkpolicy media-management-network-policy -n "$NAMESPACE" -o jsonpath='{range .spec.egress[*]}      - Port: {.ports[0].port}/{.ports[0].protocol} to {.to[0].namespaceSelector.matchLabels.name}{.to[0].podSelector.matchLabels.app}{"\n"}{end}' 2>/dev/null || echo "      Unable to parse egress rules"

    # Show policy types
    POLICY_TYPES=$(kubectl get networkpolicy media-management-network-policy -n "$NAMESPACE" -o jsonpath='{.spec.policyTypes[*]}' 2>/dev/null || echo "unknown")
    echo "   📋 Policy types: $POLICY_TYPES"
else
    print_status "error" "Network Policy not found"
fi

print_separator
echo ""
echo -e "${CYAN}🛡️  Pod Disruption Budget Status:${NC}"
if kubectl get poddisruptionbudget media-management-pdb -n "$NAMESPACE" >/dev/null 2>&1; then
    print_status "ok" "Pod Disruption Budget exists"
    # Get PDB details
    MIN_AVAILABLE=$(kubectl get poddisruptionbudget media-management-pdb -n "$NAMESPACE" -o jsonpath='{.spec.minAvailable}' 2>/dev/null || echo "unknown")
    CURRENT_HEALTHY=$(kubectl get poddisruptionbudget media-management-pdb -n "$NAMESPACE" -o jsonpath='{.status.currentHealthy}' 2>/dev/null || echo "unknown")
    echo "   🛡️  Min available: $MIN_AVAILABLE, Currently healthy: $CURRENT_HEALTHY"
else
    print_status "error" "PDB not found"
fi

print_separator
echo ""
echo -e "${CYAN}💾 Persistent Storage Status:${NC}"
if kubectl get pvc media-storage-pvc -n "$NAMESPACE" >/dev/null 2>&1; then
    kubectl get pvc media-storage-pvc -n "$NAMESPACE"
    print_status "ok" "PVC exists"

    # Get PVC details
    PVC_STATUS=$(kubectl get pvc media-storage-pvc -n "$NAMESPACE" -o jsonpath='{.status.phase}' 2>/dev/null || echo "unknown")
    PVC_CAPACITY=$(kubectl get pvc media-storage-pvc -n "$NAMESPACE" -o jsonpath='{.status.capacity.storage}' 2>/dev/null || echo "unknown")
    PVC_REQUESTED=$(kubectl get pvc media-storage-pvc -n "$NAMESPACE" -o jsonpath='{.spec.resources.requests.storage}' 2>/dev/null || echo "unknown")
    PVC_STORAGE_CLASS=$(kubectl get pvc media-storage-pvc -n "$NAMESPACE" -o jsonpath='{.spec.storageClassName}' 2>/dev/null || echo "default")
    PVC_ACCESS_MODES=$(kubectl get pvc media-storage-pvc -n "$NAMESPACE" -o jsonpath='{.spec.accessModes[*]}' 2>/dev/null || echo "unknown")
    PV_NAME=$(kubectl get pvc media-storage-pvc -n "$NAMESPACE" -o jsonpath='{.spec.volumeName}' 2>/dev/null || echo "unknown")

    echo "   📊 Status: $PVC_STATUS"
    echo "   📦 Requested: $PVC_REQUESTED, Allocated: $PVC_CAPACITY"
    echo "   🏷️  Storage Class: $PVC_STORAGE_CLASS"
    echo "   🔐 Access Modes: $PVC_ACCESS_MODES"
    echo "   🗄️  Persistent Volume: $PV_NAME"

    if [ "$PVC_STATUS" = "Bound" ]; then
        print_status "ok" "PVC is bound and ready"

        # Get Persistent Volume details if available
        if [ "$PV_NAME" != "unknown" ] && kubectl get pv "$PV_NAME" >/dev/null 2>&1; then
            PV_RECLAIM_POLICY=$(kubectl get pv "$PV_NAME" -o jsonpath='{.spec.persistentVolumeReclaimPolicy}' 2>/dev/null || echo "unknown")
            PV_STATUS=$(kubectl get pv "$PV_NAME" -o jsonpath='{.status.phase}' 2>/dev/null || echo "unknown")
            echo "   🗃️  PV Status: $PV_STATUS, Reclaim Policy: $PV_RECLAIM_POLICY"
        fi
    else
        print_status "warning" "PVC is not bound (Status: $PVC_STATUS)"

        # Show PVC events for troubleshooting
        echo "   🔍 Recent PVC events:"
        kubectl describe pvc media-storage-pvc -n "$NAMESPACE" 2>/dev/null | grep -A 10 "Events:" | tail -n +2 | head -5 | sed 's/^/      /' || echo "      No events found"
    fi

    # Check if storage class exists and get details
    if [ "$PVC_STORAGE_CLASS" != "default" ] && kubectl get storageclass "$PVC_STORAGE_CLASS" >/dev/null 2>&1; then
        SC_PROVISIONER=$(kubectl get storageclass "$PVC_STORAGE_CLASS" -o jsonpath='{.provisioner}' 2>/dev/null || echo "unknown")
        SC_RECLAIM_POLICY=$(kubectl get storageclass "$PVC_STORAGE_CLASS" -o jsonpath='{.reclaimPolicy}' 2>/dev/null || echo "unknown")
        SC_BINDING_MODE=$(kubectl get storageclass "$PVC_STORAGE_CLASS" -o jsonpath='{.volumeBindingMode}' 2>/dev/null || echo "unknown")
        echo "   🏪 StorageClass Details:"
        echo "      Provisioner: $SC_PROVISIONER"
        echo "      Reclaim Policy: $SC_RECLAIM_POLICY"
        echo "      Binding Mode: $SC_BINDING_MODE"
    fi
else
    print_status "error" "PVC not found"
    echo "   ⚠️  Media files will be stored in ephemeral storage and lost on pod restart"
fi

print_separator
echo ""
echo -e "${CYAN}🔗 Volume Mount Status:${NC}"
if kubectl get pods -l app=media-management-service -n "$NAMESPACE" >/dev/null 2>&1; then
    POD_NAME=$(kubectl get pods -l app=media-management-service -n "$NAMESPACE" -o jsonpath='{.items[0].metadata.name}' 2>/dev/null)
    if [ -n "$POD_NAME" ]; then
        # Check volume mounts
        MOUNT_INFO=$(kubectl get pod "$POD_NAME" -n "$NAMESPACE" -o jsonpath='{.spec.containers[0].volumeMounts[?(@.name=="media-storage")]}' 2>/dev/null || echo "")
        if [ -n "$MOUNT_INFO" ]; then
            MOUNT_PATH=$(echo "$MOUNT_INFO" | grep -o '"mountPath":"[^"]*"' | cut -d'"' -f4 2>/dev/null || echo "unknown")
            print_status "ok" "Media storage mounted at: $MOUNT_PATH"

            # Test mount accessibility (if pod is running)
            POD_STATUS=$(kubectl get pod "$POD_NAME" -n "$NAMESPACE" -o jsonpath='{.status.phase}' 2>/dev/null || echo "Unknown")
            if [ "$POD_STATUS" = "Running" ]; then
                # Check if mount is writable
                MOUNT_TEST=$(kubectl exec "$POD_NAME" -n "$NAMESPACE" -- test -w "$MOUNT_PATH" 2>/dev/null && echo "writable" || echo "not writable")
                echo "   ✍️  Mount status: $MOUNT_TEST"

                # Check disk usage if possible
                DISK_USAGE=$(kubectl exec "$POD_NAME" -n "$NAMESPACE" -- df -h "$MOUNT_PATH" 2>/dev/null | tail -1 | awk '{print $2" total, "$3" used, "$4" available ("$5" used)"}' 2>/dev/null || echo "unable to check")
                echo "   💽 Disk usage: $DISK_USAGE"
            else
                print_status "warning" "Pod not running - cannot test mount accessibility"
            fi
        else
            print_status "error" "Media storage volume mount not found in pod spec"
        fi
    else
        print_status "error" "No pod found to check volume mounts"
    fi
else
    print_status "error" "No pods found"
fi

print_separator
echo ""
echo -e "${CYAN}🐳 Docker Image Status:${NC}"
if command_exists minikube && minikube status >/dev/null 2>&1; then
    eval "$(minikube docker-env)"
    if docker images --format "{{.Repository}}:{{.Tag}}" | grep -q "^${FULL_IMAGE_NAME}$"; then
        IMAGE_INFO=$(docker images --format "{{.Repository}}:{{.Tag}}\t{{.ID}}\t{{.Size}}" | grep "^${FULL_IMAGE_NAME}")
        print_status "ok" "Docker image exists in Minikube"
        echo "   🏷️  Image: $IMAGE_INFO"
    else
        print_status "warning" "Docker image not found in Minikube"
        echo "   🔍 Available images:"
        docker images --format "   {{.Repository}}:{{.Tag}}" | head -5
    fi
else
    print_status "warning" "Cannot check Docker images (Minikube not available)"
fi

print_separator
echo ""
echo -e "${CYAN}🌍 Access URLs:${NC}"
echo "  Health Check: http://media-management.local/api/v1/media-management/health"
echo "  Readiness Check: http://media-management.local/api/v1/media-management/ready"

print_separator
echo ""
echo -e "${CYAN}🔗 Connectivity Test:${NC}"
if command_exists curl; then
    # Test health endpoint
    if curl -s -f -m 5 http://media-management.local/api/v1/media-management/health >/dev/null 2>&1; then
        print_status "ok" "Health endpoint responding"

        # Get health response
        HEALTH_RESPONSE=$(curl -s -m 5 http://media-management.local/api/v1/media-management/health 2>/dev/null || echo "")
        if [ -n "$HEALTH_RESPONSE" ]; then
            echo "   💓 Health: $(echo "$HEALTH_RESPONSE" | jq -r '.status' 2>/dev/null || echo "unknown")"
        fi
    else
        print_status "error" "Health endpoint not responding"
    fi

    # Test readiness endpoint
    if curl -s -f -m 5 http://media-management.local/api/v1/media-management/ready >/dev/null 2>&1; then
        print_status "ok" "Readiness endpoint responding"
    else
        print_status "warning" "Readiness endpoint not responding"
    fi

    # Check /etc/hosts entry
    if grep -q "media-management.local" /etc/hosts 2>/dev/null; then
        print_status "ok" "/etc/hosts entry exists for media-management.local"
    else
        print_status "warning" "/etc/hosts entry missing for media-management.local"
        echo -e "   ${YELLOW}💡 Run the deploy script to add it automatically${NC}"
    fi
else
    print_status "warning" "curl not available - cannot test connectivity"
fi

print_separator
echo ""
echo -e "${CYAN}📊 Summary:${NC}"
# Check overall service health
STORAGE_HEALTHY=true

# Check pod status
POD_RUNNING=$(kubectl get pods -l app=media-management-service -n "$NAMESPACE" -o jsonpath='{.items[0].status.phase}' 2>/dev/null | grep -q "Running" && echo "true" || echo "false")

# Check connectivity
if [ "$POD_RUNNING" = "true" ]; then
    ENDPOINT_ACCESSIBLE=$(curl -s -f -m 5 http://media-management.local/api/v1/media-management/health >/dev/null 2>&1 && echo "true" || echo "false")
else
    ENDPOINT_ACCESSIBLE="false"
fi

# Check storage status
PVC_BOUND=$(kubectl get pvc media-storage-pvc -n "$NAMESPACE" -o jsonpath='{.status.phase}' 2>/dev/null | grep -q "Bound" && echo "true" || echo "false")

if [ "$PVC_BOUND" != "true" ]; then
    STORAGE_HEALTHY=false
fi

# Overall assessment
if [ "$POD_RUNNING" = "true" ] && [ "$ENDPOINT_ACCESSIBLE" = "true" ] && [ "$STORAGE_HEALTHY" = "true" ]; then
    print_status "ok" "Service is fully operational with persistent storage"
elif [ "$POD_RUNNING" = "true" ] && [ "$ENDPOINT_ACCESSIBLE" = "true" ] && [ "$STORAGE_HEALTHY" = "false" ]; then
    print_status "warning" "Service is running but storage may not persist (PVC not bound)"
elif [ "$POD_RUNNING" = "true" ] && [ "$ENDPOINT_ACCESSIBLE" = "false" ]; then
    print_status "warning" "Service is running but not accessible via ingress"
else
    print_status "error" "Service is not running properly"
fi

# Storage summary
if [ "$STORAGE_HEALTHY" = "true" ]; then
    PVC_CAPACITY=$(kubectl get pvc media-storage-pvc -n "$NAMESPACE" -o jsonpath='{.status.capacity.storage}' 2>/dev/null || echo "unknown")
    echo "   💾 Persistent storage: $PVC_CAPACITY allocated and bound"
else
    echo "   ⚠️  Persistent storage: Not available - files will be lost on restart"
fi

print_separator
echo ""
echo -e "${CYAN}🔧 Quick Actions:${NC}"
echo "  Deploy:  ./scripts/containerManagement/deploy-container.sh"
echo "  Update:  ./scripts/containerManagement/update-container.sh"
echo "  Restart: ./scripts/containerManagement/stop-container.sh && ./scripts/containerManagement/start-container.sh"
echo "  Cleanup: ./scripts/containerManagement/cleanup-container.sh"

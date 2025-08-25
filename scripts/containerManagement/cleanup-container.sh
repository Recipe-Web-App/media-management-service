#!/bin/bash
# scripts/containerManagement/cleanup-container.sh

set -euo pipefail

NAMESPACE="media-management"
IMAGE_NAME="media-management-service"
IMAGE_TAG="latest"
FULL_IMAGE_NAME="${IMAGE_NAME}:${IMAGE_TAG}"

# Command line options
DELETE_PVC="prompt"  # Default: prompt user

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --delete-pvc)
            DELETE_PVC="yes"
            shift
            ;;
        --keep-pvc)
            DELETE_PVC="no"
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --delete-pvc    Automatically delete PVC without prompting"
            echo "  --keep-pvc      Keep PVC and media files (skip deletion)"
            echo "  --help, -h      Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                    # Interactive mode (default)"
            echo "  $0 --keep-pvc         # Clean up everything except media files"
            echo "  $0 --delete-pvc       # Delete everything including media files"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

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
echo -e "${CYAN}💾 Persistent Volume Claim Cleanup...${NC}"
# Check if PVC exists and show storage info
if kubectl get pvc media-storage-pvc -n "$NAMESPACE" >/dev/null 2>&1; then
    PVC_CAPACITY=$(kubectl get pvc media-storage-pvc -n "$NAMESPACE" -o jsonpath='{.status.capacity.storage}' 2>/dev/null || echo "Unknown")
    PVC_STATUS=$(kubectl get pvc media-storage-pvc -n "$NAMESPACE" -o jsonpath='{.status.phase}' 2>/dev/null || echo "Unknown")
    PVC_USED=$(kubectl get pods -l app=media-management-service -n "$NAMESPACE" -o jsonpath='{.items[0].metadata.name}' 2>/dev/null | xargs -I {} kubectl exec {} -n "$NAMESPACE" -- df -h /app/media 2>/dev/null | tail -1 | awk '{print $3}' 2>/dev/null || echo "unknown")

    echo "   📊 PVC Status: $PVC_STATUS"
    echo "   📦 Storage: $PVC_CAPACITY"
    echo "   💾 Used Space: $PVC_USED"

    # Determine PVC deletion action based on flags and user input
    SHOULD_DELETE_PVC="false"

    if [ "$DELETE_PVC" = "yes" ]; then
        SHOULD_DELETE_PVC="true"
        print_status "warning" "⚠️  PVC will be automatically deleted (--delete-pvc flag specified)"
    elif [ "$DELETE_PVC" = "no" ]; then
        SHOULD_DELETE_PVC="false"
        print_status "ok" "PVC will be preserved (--keep-pvc flag specified)"
    else
        # Interactive mode - prompt user
        print_status "warning" "⚠️  PVC contains persistent media files that will be PERMANENTLY DELETED"
        echo ""
        echo -e "${YELLOW}❓ Do you want to delete the Persistent Volume Claim and all stored media files?${NC}"
        echo -e "   ${RED}⚠️  This action is IRREVERSIBLE - all uploaded media will be lost forever${NC}"
        echo -e "   ${GREEN}ℹ️  Choose 'n' to keep media files for future deployments${NC}"
        echo ""

        while true; do
            echo -ne "${CYAN}Delete PVC and all media files? (y/N): ${NC}"
            read -r CONFIRM_DELETE

            case $CONFIRM_DELETE in
                [Yy]|[Yy][Ee][Ss])
                    SHOULD_DELETE_PVC="true"
                    break
                    ;;
                [Nn]|[Nn][Oo]|"")
                    SHOULD_DELETE_PVC="false"
                    break
                    ;;
                *)
                    echo -e "${RED}Please answer 'y' (yes) or 'n' (no)${NC}"
                    ;;
            esac
        done
    fi

    # Execute PVC deletion or preservation
    if [ "$SHOULD_DELETE_PVC" = "true" ]; then
        echo ""
        print_status "warning" "Proceeding with PVC deletion..."

        kubectl delete pvc media-storage-pvc -n "$NAMESPACE" --ignore-not-found

        # Wait for PVC deletion (it might take time due to finalizers)
        echo "   ⏳ Waiting for PVC deletion to complete..."
        kubectl wait --for=delete pvc/media-storage-pvc -n "$NAMESPACE" --timeout=90s 2>/dev/null || true

        print_status "ok" "PVC deletion completed - all media files have been permanently deleted"
    else
        print_status "ok" "PVC preserved - media files will be available for future deployments"
        echo "   💡 Tip: The PVC will remain and can be reused when you deploy again"
        echo "   🔄 To delete PVC later, run: kubectl delete pvc media-storage-pvc -n $NAMESPACE"
    fi
else
    print_status "ok" "PVC was not found (already cleaned up)"
fi

print_separator
echo -e "${CYAN}🏷️  Cleaning up StorageClass (if created)...${NC}"
# Only delete the storage class if it was created by our deployment
if kubectl get storageclass media-storage-class >/dev/null 2>&1; then
    # Check if any other PVCs are using this storage class
    USAGE_COUNT=$(kubectl get pvc --all-namespaces -o jsonpath='{range .items[*]}{.spec.storageClassName}{"\n"}{end}' | grep -c "^media-storage-class$" 2>/dev/null || echo "0")

    if [ "$USAGE_COUNT" -eq 0 ]; then
        kubectl delete storageclass media-storage-class --ignore-not-found
        print_status "ok" "StorageClass deleted (not in use by other PVCs)"
    else
        print_status "warning" "StorageClass kept (still used by $USAGE_COUNT other PVCs)"
    fi
else
    print_status "ok" "StorageClass was not found (using cluster default)"
fi

print_separator
echo -e "${CYAN}📂 Deleting namespace...${NC}"
if [ "${SHOULD_DELETE_PVC:-}" = "false" ]; then
  print_status "warning" "Namespace NOT deleted so PVC/media files are preserved. To delete namespace later run: kubectl delete namespace $NAMESPACE"
else
  kubectl delete namespace "$NAMESPACE" --ignore-not-found
  print_status "ok" "Namespace deletion completed"
fi

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

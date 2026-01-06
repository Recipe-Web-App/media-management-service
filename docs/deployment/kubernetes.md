# Kubernetes Deployment Guide

This guide covers deploying the Media Management Service to Kubernetes using the provided container management scripts
and manifests.

## Overview

The service is designed for production Kubernetes deployment with:

- **Multi-stage Docker builds** for optimized container images
- **Comprehensive K8s manifests** with security best practices
- **Automated deployment scripts** for easy management
- **Health and readiness probes** for reliable operations
- **ConfigMaps and Secrets** for secure configuration management

## Prerequisites

### Required Tools

```bash
# Verify required tools are installed
minikube version    # Local Kubernetes cluster
kubectl version     # Kubernetes CLI
docker version      # Container runtime
jq --version       # JSON processing tool
```

### Environment Setup

1. **Configure production environment variables:**

   ```bash
   # Copy and customize production environment file
   cp .env.example .env.prod
   # Edit .env.prod with your production settings
   ```

2. **Start Minikube (for local deployment):**

   ```bash
   minikube start
   minikube addons enable ingress
   ```

## Deployment Architecture

### Kubernetes Resources

| Resource                | Purpose                     | Security Features                    |
| ----------------------- | --------------------------- | ------------------------------------ |
| **ConfigMap**           | Non-sensitive configuration | Environment variable injection       |
| **Secret**              | Database passwords          | Base64 encoded, limited access       |
| **Deployment**          | Application pods            | Non-root user, read-only filesystem  |
| **Service**             | Internal networking         | ClusterIP for internal access        |
| **HTTPRoute**           | External access             | Gateway API routing, TLS ready       |
| **NetworkPolicy**       | Network security            | Restricts pod-to-pod communication   |
| **PodDisruptionBudget** | High availability           | Ensures minimum replica availability |

### Container Security

- **Non-root execution**: Runs as user `media` (UID 10001)
- **Read-only root filesystem**: Enhanced security posture
- **Resource limits**: CPU and memory constraints
- **Health checks**: Liveness and readiness probes

## Deployment Scripts

### Quick Start

```bash
# Full deployment (builds image, applies all manifests)
./scripts/containerManagement/deploy-container.sh

# Verify deployment status
./scripts/containerManagement/get-container-status.sh

# Test the service
curl http://sous-chef-proxy.local/api/v1/media-management/health
```

### Script Reference

#### `deploy-container.sh`

**Purpose**: Complete deployment from scratch

**What it does:**

1. Validates prerequisites (minikube, kubectl, docker, jq)
2. Starts minikube if not running
3. Loads environment variables from `.env.prod`
4. Builds Docker image inside minikube
5. Creates/updates ConfigMap from environment variables
6. Creates/updates Secret with database password
7. Applies all Kubernetes manifests
8. Waits for deployment to be ready
9. Sets up `/etc/hosts` entry for local access

**Usage:**

```bash
./scripts/containerManagement/deploy-container.sh
```

#### `get-container-status.sh`

**Purpose**: Comprehensive deployment status check

**Shows:**

- Prerequisites verification
- Namespace and resource status
- Pod health and logs
- Service and HTTPRoute configuration
- ConfigMap and secret details
- Network policy and PDB status
- Docker image information
- Connectivity tests

**Usage:**

```bash
./scripts/containerManagement/get-container-status.sh
```

#### `start-container.sh`

**Purpose**: Start existing deployment (scale up)

**Usage:**

```bash
./scripts/containerManagement/start-container.sh
```

#### `stop-container.sh`

**Purpose**: Stop deployment without removal (scale down)

**Usage:**

```bash
./scripts/containerManagement/stop-container.sh
```

#### `update-container.sh`

**Purpose**: Rebuild image and restart deployment

**What it does:**

1. Builds new Docker image
2. Restarts deployment to pick up new image
3. Waits for rollout completion

**Usage:**

```bash
./scripts/containerManagement/update-container.sh
```

#### `cleanup-container.sh`

**Purpose**: Complete cleanup of all resources

**What it removes:**

- Deployment and pods
- Service and HTTPRoute
- ConfigMap and secrets
- Network policy and PDB
- Namespace (optional)
- Docker images
- `/etc/hosts` entries

**Usage:**

```bash
./scripts/containerManagement/cleanup-container.sh
```

## Configuration Management

### Environment Variables

The deployment uses a two-tier configuration approach:

#### ConfigMap (Non-sensitive data)

```yaml
# Applied from .env.prod via envsubst
data:
  MEDIA_SERVICE_SERVER_HOST: "0.0.0.0"
  MEDIA_SERVICE_SERVER_PORT: "3000"
  POSTGRES_HOST: "${POSTGRES_HOST}"
  POSTGRES_DB: "${POSTGRES_DB}"
  POSTGRES_SCHEMA: "${POSTGRES_SCHEMA}"
  OAUTH2_SERVICE_ENABLED: "true"
  OAUTH2_CLIENT_ID: "${OAUTH2_CLIENT_ID}"
  OAUTH2_SERVICE_BASE_URL: "${OAUTH2_SERVICE_BASE_URL}"
  OAUTH2_INTROSPECTION_ENABLED: "false"
  OAUTH2_SERVICE_TO_SERVICE_ENABLED: "true"
  MEDIA_SERVICE_MIDDLEWARE_METRICS_ENABLED: "true"
  MEDIA_SERVICE_MIDDLEWARE_METRICS_ENDPOINT_ENABLED: "true"
  # ... other non-sensitive config
```

#### Secret (Sensitive data)

```yaml
# Applied from .env.prod via envsubst
stringData:
  MEDIA_MANAGEMENT_DB_PASSWORD: "${MEDIA_MANAGEMENT_DB_PASSWORD}"
  OAUTH2_CLIENT_SECRET: "${OAUTH2_CLIENT_SECRET}"
  JWT_SECRET: "${JWT_SECRET}"
```

### Production Environment File (`.env.prod`)

Required variables for deployment:

```bash
# Database Configuration
POSTGRES_HOST=recipe-database-service.recipe-database.svc.cluster.local
POSTGRES_PORT=5432
POSTGRES_DB=recipe_database
POSTGRES_SCHEMA=recipe_manager
MEDIA_MANAGEMENT_DB_USER=media_management_db_user
MEDIA_MANAGEMENT_DB_PASSWORD=your_secure_password

# OAuth2 Authentication Configuration
OAUTH2_SERVICE_ENABLED=true
OAUTH2_CLIENT_ID=recipe-service-client
OAUTH2_CLIENT_SECRET=your_oauth2_client_secret
OAUTH2_SERVICE_BASE_URL=http://auth-service.auth.svc.cluster.local/api/v1/auth
JWT_SECRET=your_jwt_secret_at_least_32_characters_long
OAUTH2_INTROSPECTION_ENABLED=false
OAUTH2_SERVICE_TO_SERVICE_ENABLED=true

# Metrics Configuration
MEDIA_SERVICE_MIDDLEWARE_METRICS_ENABLED=true
MEDIA_SERVICE_MIDDLEWARE_METRICS_ENDPOINT_ENABLED=true

# Runtime Mode (automatically set to production in container)
RUN_MODE=production
```

## Networking

### Service Configuration

- **Type**: ClusterIP (internal access only)
- **Port**: 3000
- **Target**: Container port 3000

### HTTPRoute Configuration (Gateway API)

- **Hosts**: `sous-chef-proxy.local`, `media-management.local` (for local development)
- **Path**: `/api/v1/media-management` (prefix-based routing)
- **Backend**: media-management-service:3000
- **Gateway**: Kong gateway in `kong` namespace

### Network Policies

Restricts network traffic to:

- **Ingress**: Only from Gateway API controller (Kong)
- **Egress**:
  - Database service (PostgreSQL)
  - OAuth2 authentication service
  - DNS resolution
  - Prometheus metrics scraping (if external)

## Storage

### Persistent Storage

- **Mount Path**: `/app/media`
- **Type**: emptyDir (for development) or PersistentVolume (for production)
- **Access**: ReadWriteOnce

### Storage Classes

For production deployment, configure appropriate storage class:

```yaml
# Example PVC for production
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: media-storage
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 100Gi
  storageClassName: fast-ssd
```

## Monitoring & Observability

### Health Checks

- **Liveness Probe**: `/api/v1/media-management/health`
- **Readiness Probe**: `/api/v1/media-management/ready`
- **Check Interval**: 30 seconds
- **Timeout**: 5 seconds

### Logging

- **Format**: JSON (structured for log aggregation)
- **Level**: Info (configurable via ConfigMap)
- **Output**: stdout/stderr (collected by cluster logging)

### Metrics

- **Framework**: Prometheus metrics
- **Export**: Prometheus text format
- **Endpoint**: `/metrics` (available at root path, not under API prefix)
- **Metrics Categories**:
  - HTTP request/response metrics (duration, size, errors)
  - Business metrics (uploads, processing, storage)
  - System metrics (authentication, rate limiting)
  - Error tracking and classification

## Troubleshooting

### Common Issues

#### Deployment Fails

```bash
# Check pod status
kubectl get pods -n media-management

# Check pod logs
kubectl logs -n media-management deployment/media-management-service

# Check events
kubectl get events -n media-management --sort-by='.lastTimestamp'
```

#### Service Not Accessible

```bash
# Verify service
kubectl get svc -n media-management

# Check HTTPRoute (Gateway API)
kubectl get httproute -n media-management

# Test from inside cluster
kubectl run debug --image=busybox --rm -it -- sh
# wget -qO- http://media-management-service.media-management:3000/api/v1/media-management/health
```

#### ConfigMap/Secret Issues

```bash
# Check ConfigMap
kubectl describe configmap media-management-config -n media-management

# Check Secret (without exposing values)
kubectl describe secret media-management-secrets -n media-management

# Verify environment variables in pod
kubectl exec -n media-management deployment/media-management-service -- env | grep MEDIA

# Check OAuth2 configuration
kubectl exec -n media-management deployment/media-management-service -- env | grep OAUTH2
kubectl exec -n media-management deployment/media-management-service -- env | grep JWT
```

### Debug Commands

```bash
# Get comprehensive status
./scripts/containerManagement/get-container-status.sh

# Follow logs in real-time
kubectl logs -n media-management deployment/media-management-service -f

# Execute into running container
kubectl exec -n media-management deployment/media-management-service -it -- sh

# Port forward for direct access
kubectl port-forward -n media-management deployment/media-management-service 3000:3000
```

## Production Considerations

### Security Hardening

- Use specific image tags (not `latest`)
- Implement Pod Security Standards
- Configure RBAC with least privilege
- Use network policies to restrict traffic
- Enable audit logging

### High Availability

- Multiple replicas with anti-affinity rules
- Pod Disruption Budgets
- Horizontal Pod Autoscaler
- Resource requests and limits

### Storage

- Use persistent volumes for media files
- Implement backup strategies
- Consider distributed storage solutions

### Monitoring

- Integrate with Prometheus/Grafana
- Set up alerting for critical metrics
- Implement distributed tracing (planned)
- Monitor resource utilization
- Configure Prometheus scraping:

  ```yaml
  apiVersion: v1
  kind: ServiceMonitor
  metadata:
    name: media-management-metrics
  spec:
    selector:
      matchLabels:
        app: media-management-service
    endpoints:
      - port: http
        path: /metrics
        interval: 15s
  ```

## See Also

- [Docker Deployment Guide](docker.md)
- [Environment Setup Guide](../development/environment-setup.md)
- [Architecture Overview](../architecture/system-overview.md)

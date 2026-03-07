# Media Management Service - Tooling & Developer Experience

Modern project tooling for the service rewrite: Lefthook, Taskfile, devcontainers, Renovate, build optimization, and Kustomize.

## 1. Lefthook (Git Hooks)

Replaces pre-commit. Lefthook is faster (Go-based), runs hooks in parallel, and has a simpler config.

### `lefthook.yml`

```yaml
pre-commit:
  parallel: true
  commands:
    fmt:
      glob: "*.rs"
      run: cargo fmt --all -- --check
    clippy:
      glob: "*.rs"
      run: cargo clippy --all-targets -- -D warnings
    deny:
      run: cargo deny check

pre-push:
  commands:
    test:
      run: cargo test --lib
```

No coverage check in hooks (too slow). Coverage runs in CI only.

Install: `cargo install lefthook` or `brew install lefthook`, then `lefthook install`.

## 2. Taskfile (go-task)

Replaces manual cargo commands with a cross-platform task runner. Better UX
than Makefiles, dependency tracking, and file-based caching.

### `Taskfile.yml`

```yaml
version: '3'

vars:
  BINARY_NAME: media-management-service

tasks:
  default:
    desc: Show available tasks
    cmds: [task --list]

  build:
    desc: Build the project
    cmds: [cargo build]
    sources: [src/**/*.rs, Cargo.toml, Cargo.lock]

  build:release:
    desc: Build optimized release binary
    cmds: [cargo build --release]

  run:
    desc: Run the service locally
    cmds: [cargo run]
    env:
      RUN_MODE: local
      RUST_LOG: debug

  test:
    desc: Run all tests
    cmds: [cargo test]

  test:unit:
    desc: Run unit tests only
    cmds: [cargo test --lib]

  test:integration:
    desc: Run integration tests only
    cmds: ["cargo test --test '*'"]

  lint:
    desc: Run all linters
    cmds:
      - cargo fmt --all -- --check
      - cargo clippy --all-targets -- -D warnings
      - cargo deny check

  fmt:
    desc: Format code
    cmds: [cargo fmt --all]

  check:
    desc: Quick compile check
    cmds: [cargo check]

  clean:
    desc: Clean build artifacts
    cmds: [cargo clean]

  coverage:
    desc: Generate HTML coverage report
    cmds: [cargo llvm-cov --html]

  docker:build:
    desc: Build Docker image
    cmds: [docker build -t {{.BINARY_NAME}}:dev .]

  docker:run:
    desc: Run Docker container locally
    deps: [docker:build]
    cmds:
      - docker run --rm -p 3000:3000 --env-file .env.local {{.BINARY_NAME}}:dev

  k8s:local:
    desc: Generate k8s manifests for local
    cmds: [kustomize build k8s/overlays/local]

  k8s:prod:
    desc: Generate k8s manifests for production
    cmds: [kustomize build k8s/overlays/prod]

  setup:
    desc: Install development dependencies
    cmds:
      - rustup component add llvm-tools-preview
      - cargo install cargo-llvm-cov cargo-deny lefthook
      - lefthook install
```

Install go-task: `brew install go-task` or `sh -c "$(curl --location https://taskfile.dev/install.sh)" -- -d -b /usr/local/bin`

## 3. Devcontainer

Standardized dev environment for VS Code and GitHub Codespaces.

### `.devcontainer/devcontainer.json`

```json
{
  "name": "Media Management Service",
  "image": "mcr.microsoft.com/devcontainers/rust:1-bookworm",
  "features": {
    "ghcr.io/devcontainers/features/common-utils:2": {},
    "ghcr.io/devcontainers-contrib/features/go-task:1": {}
  },
  "customizations": {
    "vscode": {
      "extensions": ["rust-lang.rust-analyzer", "tamasfe.even-better-toml", "serayuzgur.crates", "vadimcn.vscode-lldb"],
      "settings": {
        "rust-analyzer.check.command": "clippy",
        "rust-analyzer.check.extraArgs": ["--all-targets"],
        "editor.formatOnSave": true
      }
    }
  },
  "postCreateCommand": "task setup",
  "forwardPorts": [3000],
  "remoteUser": "vscode"
}
```

PostgreSQL and other services can be added via docker-compose for integration testing.

## 4. Renovate

Automated dependency updates with smart grouping and auto-merge for safe changes.

### `renovate.json`

```json
{
  "$schema": "https://docs.renovatebot.com/renovate-schema.json",
  "extends": ["config:recommended", "group:allNonMajor", ":separateMajorReleases"],
  "cargo": {
    "enabled": true
  },
  "lockFileMaintenance": {
    "enabled": true,
    "schedule": ["before 5am on monday"]
  },
  "labels": ["dependencies"],
  "prConcurrentLimit": 5,
  "packageRules": [
    {
      "matchManagers": ["cargo"],
      "matchUpdateTypes": ["patch", "minor"],
      "groupName": "Rust minor/patch updates",
      "automerge": true
    },
    {
      "matchManagers": ["github-actions"],
      "groupName": "GitHub Actions updates",
      "automerge": true
    }
  ]
}
```

## 5. Cargo Build Optimization

Addresses three pain points: build speed, Docker build speed, and target/ directory eating SSD space.

### `.cargo/config.toml`

```toml
# Use mold linker for 2-10x faster linking
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.aarch64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[build]
jobs = 4    # Limit parallel rustc processes to save RAM

[net]
git-fetch-with-cli = true    # More reliable git fetches in CI
```

### Cargo Profile Settings (in Cargo.toml)

```toml
[profile.dev]
opt-level = 0
debug = "line-tables-only"       # ~50% smaller debug info vs full
split-debuginfo = "unpacked"     # Don't create huge .dSYM bundles
codegen-units = 256              # Max parallelism, less RAM per unit
incremental = true               # Fast rebuilds

[profile.release]
opt-level = 3
lto = "thin"                     # Good optimization without slow fat LTO
strip = true                     # Remove debug symbols from binary
codegen-units = 1                # Best optimization
panic = "abort"                  # Smaller binary, no unwinding

[profile.ci]
inherits = "dev"
incremental = false              # No benefit in CI (cold cache)
codegen-units = 16               # Balance speed and RAM in CI
```

### Why These Settings Help

| Setting                        | Effect                                          |
| ------------------------------ | ----------------------------------------------- |
| mold linker                    | 2-10x faster linking vs default ld              |
| `debug = "line-tables-only"`   | ~50% smaller target/ directory                  |
| `split-debuginfo = "unpacked"` | Avoids huge bundled debug info files            |
| `codegen-units = 256`          | Each unit smaller, less RAM per rustc process   |
| `jobs = 4`                     | Prevents 16+ parallel rustc from exhausting RAM |
| `strip = true`                 | Release binary goes from ~50MB to ~5MB          |
| `lto = "thin"`                 | Good optimization without 10-minute fat LTO     |

### Target Directory and Backups

The `target/` directory easily reaches 5-10GB. Ensure it is:

1. In `.gitignore` (already standard)
2. Excluded from backup software (Time Machine, Backblaze, rsync)
3. Optionally relocated via `CARGO_TARGET_DIR` env var to a non-backed-up location:

   ```bash
   # In .env.local or shell profile
   export CARGO_TARGET_DIR=/tmp/cargo-target/media-management-service
   ```

### Docker Build Optimization

Use `cargo-chef` to separate dependency compilation (cached) from app compilation (rebuilds on code changes).

```dockerfile
# Stage 1: Compute dependency recipe
FROM rust:1.85-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Build dependencies only (this layer is cached)
FROM rust:1.85-bookworm AS cook
RUN cargo install cargo-chef
WORKDIR /app
COPY --from=chef /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Build application (only this rebuilds on code changes)
FROM cook AS builder
COPY . .
RUN cargo build --release

# Stage 4: Minimal runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
RUN useradd -r -s /bin/false media
COPY --from=builder /app/target/release/media-management-service /usr/local/bin/
USER media
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/api/v1/media-management/health || exit 1
CMD ["media-management-service"]
```

**Layer caching behavior:**

- Dependency changes (Cargo.toml/Cargo.lock): Stages 1-3 rebuild (~5 min)
- Code changes only: Only Stage 3 rebuilds (~30 sec)
- No changes: Fully cached (~5 sec)

## 6. Kustomize (Kubernetes)

Replaces raw k8s YAML with composable bases and overlays. No templating language (unlike Helm), just patching.

### Directory Structure

```text
k8s/
├── base/
│   ├── kustomization.yaml
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── pvc.yaml
│   ├── networkpolicy.yaml
│   └── poddisruptionbudget.yaml
└── overlays/
    ├── local/
    │   ├── kustomization.yaml
    │   ├── configmap.yaml
    │   └── patches/
    │       └── deployment-patch.yaml
    └── prod/
        ├── kustomization.yaml
        ├── configmap.yaml
        └── patches/
            └── deployment-patch.yaml
```

### `base/kustomization.yaml`

```yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

commonLabels:
  app: media-management-service
  part-of: recipe-web-app

resources:
  - deployment.yaml
  - service.yaml
  - pvc.yaml
  - networkpolicy.yaml
  - poddisruptionbudget.yaml
```

### `overlays/local/kustomization.yaml`

```yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

resources:
  - ../../base

patches:
  - path: patches/deployment-patch.yaml

configMapGenerator:
  - name: media-management-config
    literals:
      - RUN_MODE=local
      - MEDIA_SERVICE_SERVER_PORT=3000
      - POSTGRES_HOST=recipe-database.local
      - POSTGRES_PORT=30382
      - POSTGRES_DB=recipe_database
      - POSTGRES_SCHEMA=recipe_manager
      - MEDIA_SERVICE_STORAGE_BASE_PATH=/app/media
      - OAUTH2_SERVICE_ENABLED=false
      - RUST_LOG=debug
```

### `overlays/local/patches/deployment-patch.yaml`

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: media-management-service
spec:
  replicas: 1
  template:
    spec:
      containers:
        - name: media-management-service
          resources:
            requests:
              memory: "128Mi"
              cpu: "100m"
            limits:
              memory: "256Mi"
              cpu: "500m"
```

### `overlays/prod/kustomization.yaml`

```yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

resources:
  - ../../base

patches:
  - path: patches/deployment-patch.yaml

configMapGenerator:
  - name: media-management-config
    literals:
      - RUN_MODE=production
      - MEDIA_SERVICE_SERVER_PORT=3000
      - POSTGRES_HOST=recipe-database-service.recipe-database.svc.cluster.local
      - POSTGRES_PORT=5432
      - POSTGRES_DB=recipe_database
      - POSTGRES_SCHEMA=recipe_manager
      - MEDIA_SERVICE_STORAGE_BASE_PATH=/app/media
      - OAUTH2_SERVICE_ENABLED=true
      - OAUTH2_INTROSPECTION_ENABLED=true
      - API_GATEWAY_URL=http://auth-service-service.auth-service.svc.cluster.local/api/v1/auth
      - RUST_LOG=info
```

### `overlays/prod/patches/deployment-patch.yaml`

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: media-management-service
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: media-management-service
          resources:
            requests:
              memory: "256Mi"
              cpu: "250m"
            limits:
              memory: "512Mi"
              cpu: "1000m"
```

### Usage

```bash
# Preview local manifests
task k8s:local

# Apply to cluster
kustomize build k8s/overlays/local | kubectl apply -f -

# Preview prod manifests
task k8s:prod
```

## 7. CI Pipeline Notes

GitHub Actions workflow improvements for the rewrite:

- **sccache**: Shared compilation cache across CI runs
- **cargo-binstall**: Install tools as pre-built binaries (skip compilation)
- **CI cargo profile**: `[profile.ci]` with no incremental, balanced codegen-units
- **Layer caching**: `actions/cache` for `~/.cargo/registry`, `~/.cargo/git`, and `target/`
- **Docker BuildKit**: `--cache-from` for layer reuse across builds

These will be implemented in Phase 8 (Tooling & DX).

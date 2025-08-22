# Development Environment Setup

This guide covers setting up a local development environment for the Media Management Service.

## Overview

The service supports two runtime modes:

- **Local Mode**: Uses `.env.local` file for configuration, pretty logging, relative storage paths
- **Production Mode**: Uses environment variables only, JSON logging, absolute container paths

This guide focuses on **Local Mode** setup for development.

## Prerequisites

### Required Software

1. **Rust 1.70+**

   ```bash
   # Install via rustup
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env

   # Verify installation
   rustc --version
   cargo --version
   ```

2. **PostgreSQL 14+**

   ```bash
   # macOS (via Homebrew)
   brew install postgresql@14
   brew services start postgresql@14

   # Ubuntu/Debian
   sudo apt-get install postgresql-14 postgresql-client-14
   sudo systemctl start postgresql

   # Create development database
   createdb recipe_database
   ```

3. **FFmpeg** (required for video processing)

   ```bash
   # macOS
   brew install ffmpeg

   # Ubuntu/Debian
   sudo apt-get install ffmpeg

   # Verify installation
   ffmpeg -version
   ```

4. **Pre-commit** (for code quality)

   ```bash
   # Install via pip
   pip install pre-commit

   # Or via Homebrew (macOS)
   brew install pre-commit
   ```

5. **Development Tools** (optional but recommended)

   ```bash
   # Install additional Rust tools
   cargo install cargo-watch    # Auto-rebuild on file changes
   cargo install cargo-llvm-cov # Code coverage
   cargo install cargo-deny     # License and dependency checking
   ```

## Environment Configuration

### 1. Environment File Setup

```bash
# Copy example to local development file
cp .env.example .env.local

# Edit with your local settings
nano .env.local  # or use your preferred editor
```

### 2. Configure `.env.local`

Update the following variables for your local environment:

```bash
# Runtime Mode (automatically detected)
RUN_MODE=local

# Database Configuration
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
POSTGRES_DB=recipe_database
POSTGRES_SCHEMA=recipe_manager
MEDIA_MANAGEMENT_DB_USER=your_username
MEDIA_MANAGEMENT_DB_PASSWORD=your_password

# Local Storage Paths
MEDIA_SERVICE_STORAGE_BASE_PATH=./media
MEDIA_SERVICE_STORAGE_TEMP_PATH=./media/temp

# Development Logging
MEDIA_SERVICE_LOGGING_LEVEL=debug
MEDIA_SERVICE_LOGGING_FORMAT=pretty
```

### 3. Database Setup

```bash
# Connect to PostgreSQL
psql -h localhost -U your_username -d recipe_database

# Create schema (if not exists)
CREATE SCHEMA IF NOT EXISTS recipe_manager;

# Grant permissions
GRANT ALL PRIVILEGES ON SCHEMA recipe_manager TO your_username;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA recipe_manager TO your_username;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA recipe_manager TO your_username;
```

### 4. Storage Directory Setup

```bash
# Create local media directories
mkdir -p media/temp

# Set appropriate permissions
chmod 755 media
chmod 755 media/temp
```

## Project Setup

### 1. Clone and Build

```bash
# Clone the repository
git clone <repository-url>
cd media-management-service

# Build dependencies
cargo build
```

### 2. Pre-commit Hooks

```bash
# Install pre-commit hooks
pre-commit install

# Verify setup
pre-commit run --all-files
```

### 3. Run Development Server

```bash
# Start in development mode (with auto-reload)
cargo watch -x run

# Or run once
cargo run
```

The service will start in **Local Mode** by default and display:

```text
INFO Starting Media Management Service
INFO Runtime mode: local
INFO Local mode: using .env.local file for configuration
INFO Configuration loaded: server will bind to 0.0.0.0:3000
```

## Development Workflow

### Code Quality Checks

```bash
# Format code
cargo fmt --all

# Lint code (warnings as errors)
cargo clippy --all-targets --all-features -- -D warnings

# Quick compile check
cargo check

# Run all quality checks
pre-commit run --all-files
```

### Testing

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test integration

# Run tests with output visible
cargo test -- --nocapture

# Generate code coverage
cargo llvm-cov
cargo llvm-cov --html  # Generate HTML report
```

### Database Development

```bash
# Check database connectivity
cargo run --bin check-db  # (if implemented)

# Run database migrations (when implemented)
# cargo run --bin migrate

# Reset database for testing
dropdb recipe_database && createdb recipe_database
```

## Environment Variables Reference

### Server Configuration

| Variable                               | Description             | Default     | Example     |
| -------------------------------------- | ----------------------- | ----------- | ----------- |
| `MEDIA_SERVICE_SERVER_HOST`            | Server bind address     | `0.0.0.0`   | `127.0.0.1` |
| `MEDIA_SERVICE_SERVER_PORT`            | Server port             | `3000`      | `8080`      |
| `MEDIA_SERVICE_SERVER_MAX_UPLOAD_SIZE` | Max upload size (bytes) | `104857600` | `52428800`  |

### Database Configuration

| Variable                       | Description       | Required | Example           |
| ------------------------------ | ----------------- | -------- | ----------------- |
| `POSTGRES_HOST`                | Database hostname | Yes      | `localhost`       |
| `POSTGRES_PORT`                | Database port     | Yes      | `5432`            |
| `POSTGRES_DB`                  | Database name     | Yes      | `recipe_database` |
| `POSTGRES_SCHEMA`              | Schema name       | Yes      | `recipe_manager`  |
| `MEDIA_MANAGEMENT_DB_USER`     | Database username | Yes      | `postgres`        |
| `MEDIA_MANAGEMENT_DB_PASSWORD` | Database password | Yes      | `password123`     |

### Storage Configuration

| Variable                              | Description               | Default        | Local Example      |
| ------------------------------------- | ------------------------- | -------------- | ------------------ |
| `MEDIA_SERVICE_STORAGE_BASE_PATH`     | Media files directory     | `./media`      | `./dev-media`      |
| `MEDIA_SERVICE_STORAGE_TEMP_PATH`     | Temporary files directory | `./media/temp` | `./dev-media/temp` |
| `MEDIA_SERVICE_STORAGE_MAX_FILE_SIZE` | Max file size (bytes)     | `524288000`    | `104857600`        |

### Logging Configuration

| Variable                       | Description | Local Default | Options                                   |
| ------------------------------ | ----------- | ------------- | ----------------------------------------- |
| `MEDIA_SERVICE_LOGGING_LEVEL`  | Log level   | `debug`       | `trace`, `debug`, `info`, `warn`, `error` |
| `MEDIA_SERVICE_LOGGING_FORMAT` | Log format  | `pretty`      | `pretty`, `json`                          |

### Runtime Mode

| Variable   | Description  | Default | Options               |
| ---------- | ------------ | ------- | --------------------- |
| `RUN_MODE` | Runtime mode | `local` | `local`, `production` |

## IDE Configuration

### VS Code

Recommended extensions:

- **rust-analyzer**: Rust language server
- **CodeLLDB**: Debugging support
- **Even Better TOML**: TOML file support
- **GitLens**: Enhanced Git integration

Settings (`.vscode/settings.json`):

```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "files.watcherExclude": {
    "**/target/**": true
  }
}
```

### IntelliJ IDEA

1. Install **Rust plugin**
2. Configure **Rust toolchain**: Settings → Languages & Frameworks → Rust
3. Enable **format on save**: Settings → Tools → Actions on Save

## Troubleshooting

### Common Issues

#### Database Connection Failed

```bash
# Check PostgreSQL is running
pg_ctl status

# Verify connection
psql -h localhost -U your_username -d recipe_database -c "SELECT 1;"

# Check .env.local configuration
grep POSTGRES .env.local
```

#### Permission Denied on Media Directory

```bash
# Check directory permissions
ls -la media/

# Fix permissions
chmod 755 media
chmod 755 media/temp

# For development, ensure user owns the directory
sudo chown -R $USER:$USER media/
```

#### FFmpeg Not Found

```bash
# Verify FFmpeg installation
which ffmpeg
ffmpeg -version

# Install if missing (see Prerequisites section)
```

#### Rust Compilation Errors

```bash
# Update Rust toolchain
rustup update

# Clean and rebuild
cargo clean
cargo build

# Check for conflicting dependencies
cargo tree --duplicates
```

### Debug Mode

Enable additional debugging:

```bash
# Set Rust backtrace
export RUST_BACKTRACE=1

# Enable SQL query logging (if using SQLx)
export SQLX_LOGGING=true

# Run with debug output
RUST_LOG=debug cargo run
```

### Performance Profiling

```bash
# Install profiling tools
cargo install cargo-profiler

# Profile application
cargo profiler --release

# Memory usage analysis
cargo install cargo-valgrind
cargo valgrind run
```

## Development Tips

### Auto-reload Development

```bash
# Install cargo-watch
cargo install cargo-watch

# Run with auto-reload on file changes
cargo watch -x run

# Run tests on file changes
cargo watch -x test

# Combine multiple commands
cargo watch -x check -x test -x run
```

### Database Schema Changes

When modifying database schemas:

1. Update domain entities in `src/domain/entities/`
2. Update repository implementations in `src/infrastructure/persistence/`
3. Run tests to verify changes: `cargo test`
4. Document schema changes in architecture docs

### Testing Strategies

```bash
# Test specific module
cargo test domain::entities::media

# Test with different log levels
RUST_LOG=trace cargo test

# Run only fast tests (exclude slow integration tests)
cargo test --lib

# Run tests in parallel
cargo test -- --test-threads=4
```

## See Also

- [Kubernetes Deployment Guide](../deployment/kubernetes.md)
- [Docker Guide](../deployment/docker.md)
- [Architecture Overview](../architecture/system-overview.md)
- [API Documentation](../api/) (planned)

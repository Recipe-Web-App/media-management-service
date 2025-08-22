# Contributing to Media Management Service

Thank you for your interest in contributing to the Media Management Service! This document provides guidelines and
information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)
- [Code Quality Standards](#code-quality-standards)
- [Testing Requirements](#testing-requirements)
- [Documentation Guidelines](#documentation-guidelines)
- [Architecture Guidelines](#architecture-guidelines)

## Code of Conduct

This project adheres to a professional and inclusive environment. We expect all contributors to be respectful and
constructive in their interactions.

## Getting Started

### Prerequisites

Before contributing, ensure you have the required tools installed:

- **Rust 1.70+** - Latest stable Rust toolchain
- **PostgreSQL 14+** - Database for local development
- **FFmpeg** - Required for media processing
- **Pre-commit** - For automated code quality checks

See the [Environment Setup Guide](docs/development/environment-setup.md) for detailed installation instructions.

### Development Setup

1. **Fork and clone the repository**

   ```bash
   git clone https://github.com/your-username/media-management-service.git
   cd media-management-service
   ```

2. **Set up local environment**

   ```bash
   # Copy and configure environment file
   cp .env.example .env.local
   # Edit .env.local with your local database settings

   # Install dependencies
   cargo build

   # Set up pre-commit hooks
   pre-commit install
   ```

3. **Verify setup**

   ```bash
   # Run tests
   cargo test

   # Run quality checks
   pre-commit run --all-files

   # Start development server
   cargo run
   ```

## Development Workflow

### Branch Naming

Use descriptive branch names with the following patterns:

- `feat/feature-name` - New features
- `fix/bug-description` - Bug fixes
- `docs/documentation-update` - Documentation changes
- `refactor/component-name` - Code refactoring
- `test/test-description` - Test additions/improvements
- `chore/maintenance-task` - Maintenance tasks

### Making Changes

1. **Create a feature branch**

   ```bash
   git checkout -b feat/your-feature-name
   ```

2. **Make your changes**
   - Follow the [Code Quality Standards](#code-quality-standards)
   - Write tests for new functionality
   - Update documentation as needed

3. **Test your changes**

   ```bash
   # Run all tests
   cargo test

   # Run quality checks
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings
   pre-commit run --all-files
   ```

4. **Commit your changes**
   - Follow the [Commit Guidelines](#commit-guidelines)
   - Use conventional commit format

## Commit Guidelines

This project uses **Conventional Commits** for consistent commit messages and automated changelog generation.

### Commit Message Format

```text
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Conventional Commit Types

| Type       | Description      | Example                                    |
| ---------- | ---------------- | ------------------------------------------ |
| `feat`     | New feature      | `feat: add media upload endpoint`          |
| `fix`      | Bug fix          | `fix: resolve database connection timeout` |
| `docs`     | Documentation    | `docs: update API documentation`           |
| `style`    | Code formatting  | `style: fix rustfmt formatting`            |
| `refactor` | Code refactoring | `refactor: extract media validation logic` |
| `test`     | Test changes     | `test: add integration tests for upload`   |
| `chore`    | Maintenance      | `chore: update dependencies`               |
| `ci`       | CI/CD changes    | `ci: add GitHub Actions workflow`          |
| `build`    | Build system     | `build: update Docker configuration`       |
| `perf`     | Performance      | `perf: optimize image processing pipeline` |
| `revert`   | Revert changes   | `revert: undo database migration changes`  |

### Commit Examples

```bash
# Feature with scope
feat(api): add health check endpoint

# Breaking change
feat!: change media upload response format

BREAKING CHANGE: The upload response now returns a different JSON structure

# Bug fix with issue reference
fix: resolve memory leak in image processing

Closes #123

# Documentation update
docs: add deployment guide for Kubernetes

# Refactoring
refactor(storage): extract file validation into separate module
```

### Commit Best Practices

- **Use imperative mood**: "add feature" not "added feature"
- **Keep subject line under 50 characters**
- **Capitalize subject line**
- **No period at end of subject line**
- **Use body to explain what and why, not how**
- **Reference issues and PRs when applicable**

## Pull Request Process

### Before Submitting

1. **Ensure all tests pass**

   ```bash
   cargo test
   cargo clippy --all-targets --all-features -- -D warnings
   cargo fmt --all -- --check
   ```

2. **Update documentation**
   - Update relevant documentation files
   - Add/update inline code documentation
   - Update API documentation if applicable

3. **Test deployment** (if applicable)

   ```bash
   # Test local deployment
   ./scripts/containerManagement/deploy-container.sh
   ```

### Submitting the PR

1. **Create pull request** using the provided template
2. **Fill out all sections** of the PR template
3. **Request appropriate reviewers**
4. **Link related issues** using GitHub keywords (e.g., "Fixes #123")

### PR Requirements

- [ ] **Tests**: All tests pass and new code is covered
- [ ] **Code Quality**: Passes clippy with no warnings
- [ ] **Documentation**: Updated as needed
- [ ] **Conventional Commits**: Follows commit message standards
- [ ] **Architecture**: Follows Clean Architecture principles
- [ ] **Security**: No security vulnerabilities introduced

## Code Quality Standards

### Rust Guidelines

- **Strict linting**: Code must pass `cargo clippy --all-targets --all-features -- -D warnings`
- **Formatting**: Use `cargo fmt --all` for consistent formatting
- **Error handling**: Use `Result<T, E>` and `?` operator, avoid panics in library code
- **Documentation**: All public APIs must have documentation comments
- **Testing**: Aim for 80%+ code coverage, domain layer should have 90%+

### Code Organization

- **Clean Architecture**: Follow the established layered architecture
- **Separation of Concerns**: Keep domain logic separate from infrastructure
- **Dependency Injection**: Use traits for external dependencies
- **Single Responsibility**: Each module/function should have one clear purpose

### Security Practices

- **Input Validation**: Validate all external inputs
- **Path Security**: Use secure file path handling
- **Secret Management**: Never commit secrets or keys
- **Least Privilege**: Design with minimal required permissions

## Testing Requirements

### Test Categories

1. **Unit Tests** (`cargo test --lib`)
   - Test individual functions and modules
   - Mock external dependencies
   - Focus on domain logic

2. **Integration Tests** (`cargo test --test integration`)
   - Test component interactions
   - Use real implementations where possible
   - Test HTTP endpoints end-to-end

3. **Property Tests** (using `proptest`)
   - Test value objects with generated inputs
   - Verify invariants hold across input ranges

### Test Organization

```rust
// Unit tests in each module
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_works_correctly() {
        // Test implementation
    }
}

// Integration tests in tests/ directory
// tests/integration/media_tests.rs
```

### Test Utilities

- Use `MediaBuilder` for creating test entities
- Use `InMemoryMediaRepository` for isolated testing
- Use `TestApp` for HTTP endpoint testing
- Mock external services using `mockall`

## Documentation Guidelines

### Types of Documentation

1. **Code Documentation**

   ````rust
   /// Brief description of the function
   ///
   /// # Arguments
   ///
   /// * `param` - Description of parameter
   ///
   /// # Returns
   ///
   /// Description of return value
   ///
   /// # Errors
   ///
   /// When this function returns an error
   ///
   /// # Examples
   ///
   /// ```
   /// let result = function_name(param);
   /// ```
   pub fn function_name(param: Type) -> Result<ReturnType, Error> {
       // Implementation
   }
   ````

2. **Architecture Documentation**
   - Add ADRs for significant architectural decisions
   - Update system overview for major changes
   - Document design patterns and rationale

3. **API Documentation**
   - Document all HTTP endpoints
   - Include request/response examples
   - Document error responses

4. **Deployment Documentation**
   - Update deployment guides for configuration changes
   - Document new environment variables
   - Include troubleshooting information

### Documentation Standards

- **Keep it current**: Update docs with code changes
- **Be comprehensive**: Include examples and edge cases
- **Use clear language**: Write for different audiences
- **Link appropriately**: Cross-reference related documentation

## Architecture Guidelines

### Clean Architecture Principles

1. **Dependency Direction**
   - Domain layer has no external dependencies
   - Application layer depends only on domain
   - Infrastructure adapts external systems to domain interfaces

2. **Layer Responsibilities**

   ```text
   Domain:         Business entities, value objects, business rules
   Application:    Use cases, DTOs, application services
   Infrastructure: Database, HTTP, file system, external APIs
   Presentation:   HTTP handlers, routes, request/response mapping
   ```

3. **Testing Strategy**
   - Domain: Pure unit tests, no mocks needed
   - Application: Use mocks for repositories and external services
   - Infrastructure: Integration tests with real implementations
   - Presentation: HTTP endpoint tests

### Design Patterns

- **Repository Pattern**: Abstract data access behind traits
- **Dependency Injection**: Use traits and constructor injection
- **Error Handling**: Use domain-specific error types
- **Validation**: Validate at service boundaries

### Performance Considerations

- **Async/Await**: Use async for all I/O operations
- **Streaming**: Handle large files with streaming
- **Connection Pooling**: Use database connection pools
- **Caching**: Implement appropriate caching strategies

## Release Process

### Version Numbering

This project follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Release Steps

1. **Update version** in `Cargo.toml`
2. **Update CHANGELOG.md** with release notes
3. **Create release tag** with conventional format
4. **Create GitHub release** with release notes

## Getting Help

### Resources

- **Documentation**: Check `docs/` directory for guides
- **Issues**: Search existing issues before creating new ones
- **Discussions**: Use GitHub Discussions for questions
- **Code Review**: Request reviews from maintainers

### Contact

- Create an issue for bugs or feature requests
- Use GitHub Discussions for general questions
- Tag maintainers for urgent issues

## License

By contributing to this project, you agree that your contributions will be licensed under the project's
[MIT OR Apache-2.0](LICENSE) license.

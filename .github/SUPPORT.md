# Support

Thank you for using the Media Management Service! This document provides resources to help you get support.

## Documentation

Before asking for help, please check our documentation:

### Primary Documentation

- **[README.md](../README.md)** - Complete feature overview, setup instructions, and API documentation
- **[CLAUDE.md](../CLAUDE.md)** - Development commands, architecture overview, and developer guide
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Contribution guidelines and development workflow
- **[SECURITY.md](SECURITY.md)** - Security features, best practices, and vulnerability reporting

### Code Examples

- **[`.env.example`](../.env.example)** - Configuration examples
- **[Docker Compose](../docker-compose.yml)** - Local development setup
- **[Kubernetes Manifests](../k8s/)** - K8s deployment configurations

## Getting Help

### 1. Search Existing Resources

Before creating a new issue, please search:

- [Existing Issues](https://github.com/Recipe-Web-App/media-management-service/issues) - Someone may have already asked
- [Closed Issues](https://github.com/Recipe-Web-App/media-management-service/issues?q=is%3Aissue+is%3Aclosed) - Your question
  may already be answered
- [Discussions](https://github.com/Recipe-Web-App/media-management-service/discussions) - Community Q&A

### 2. GitHub Discussions (Recommended for Questions)

For general questions, use [GitHub Discussions](https://github.com/Recipe-Web-App/media-management-service/discussions):

**When to use Discussions:**

- "How do I...?" questions
- Configuration help
- Best practice advice
- Integration questions
- Media processing questions
- Architecture discussions
- Troubleshooting (non-bug)

**Categories:**

- **Q&A** - Ask questions and get answers
- **Ideas** - Share feature ideas and proposals
- **Show and Tell** - Share your implementations
- **General** - Everything else

### 3. GitHub Issues (For Bugs and Features)

Use [GitHub Issues](https://github.com/Recipe-Web-App/media-management-service/issues/new/choose) for:

- Bug reports
- Feature requests
- Performance issues
- Documentation problems
- Security vulnerabilities (low severity - use Security Advisories for critical)

**Issue Templates:**

- **Bug Report** - Report unexpected behavior
- **Feature Request** - Suggest new functionality
- **Performance Issue** - Report performance problems
- **Documentation** - Documentation improvements
- **Security Vulnerability** - Low-severity security issues

### 4. Security Issues

**IMPORTANT:** For security vulnerabilities, use:

- [GitHub Security Advisories](https://github.com/Recipe-Web-App/media-management-service/security/advisories/new) (private)
- See [SECURITY.md](SECURITY.md) for details

**Never report security issues publicly through issues or discussions.**

## Common Questions

### Setup and Configuration

**Q: How do I get started?**
A: See the Quick Start section in [README.md](../README.md#quick-start) and [CLAUDE.md](../CLAUDE.md#development-setup)

**Q: What environment variables are required?**
A: Check [`.env.example`](../.env.example) for all configuration options

**Q: Can I run without PostgreSQL?**
A: PostgreSQL is required for metadata storage. See [CLAUDE.md](../CLAUDE.md#architecture-overview) for database requirements.

**Q: How do I configure storage paths?**
A: Set `MEDIA_SERVICE_STORAGE_BASE_PATH` environment variable. See [CLAUDE.md](../CLAUDE.md#runtime-modes)

### Media Processing

**Q: Which file formats are supported?**
A: Images (JPEG, PNG, WebP, AVIF), Videos (common formats via FFmpeg). See [README.md](../README.md#features)

**Q: How do I configure image processing?**
A: Media processing is automatic. AVIF is the primary format with WebP fallback.

**Q: What's the maximum file size?**
A: Configure via `MEDIA_SERVICE_MAX_FILE_SIZE` environment variable

**Q: How does content deduplication work?**
A: Files are stored using SHA-256 content hashing, preventing duplicate storage automatically

### API Usage

**Q: How do I upload files?**
A: Use `POST /api/v1/media-management/media/` for direct uploads or presigned URLs for better UX. See [CLAUDE.md](../CLAUDE.md#api-structure)

**Q: How do I implement pagination?**
A: Use cursor-based pagination with `GET /media/?cursor=&limit=`. See [CLAUDE.md](../CLAUDE.md#get-media---list-media-with-cursor-based-pagination)

**Q: How do I get file status?**
A: Use `GET /media/{id}/status` to check upload and processing status

**Q: What authentication is required?**
A: JWT-based OAuth2 authentication via the auth-service. See [CLAUDE.md](../CLAUDE.md#oauth2-testing)

### Troubleshooting

**Q: Service fails to start?**

- Check logs: `docker logs <container-name>` or `kubectl logs`
- Verify environment variables
- Check PostgreSQL connectivity
- Verify storage paths exist and are writable
- Review [CLAUDE.md](../CLAUDE.md#health-check-system)

**Q: File uploads fail?**

- Check file size limits
- Verify file type is allowed
- Check storage directory permissions
- Review disk space availability
- Check logs for specific errors

**Q: Health checks failing?**

- Verify database connectivity: `SELECT 1` query must succeed
- Check storage path accessibility
- Review health check timeout (2 seconds default)
- See [CLAUDE.md](../CLAUDE.md#health-check-system)

**Q: Performance issues?**

- Check database connection pool settings
- Verify storage I/O performance
- Review concurrent upload limits
- See [Performance Issue Template](.github/ISSUE_TEMPLATE/performance_issue.yml)

**Q: CORS errors?**

- Configure `CORS_ALLOWED_ORIGINS` environment variable
- Check request Origin header
- Review middleware configuration

### Deployment

**Q: How do I deploy to Kubernetes?**
A: Use the deployment scripts: `./scripts/containerManagement/deploy-container.sh`. See [CLAUDE.md](../CLAUDE.md#container-deployment)

**Q: How do I configure persistent storage?**
A: The service uses Kubernetes PersistentVolumeClaims (50Gi default). See [CLAUDE.md](../CLAUDE.md#storage-strategy)

**Q: How do I check deployment status?**
A: Run `./scripts/containerManagement/get-container-status.sh`

### Development

**Q: How do I contribute?**
A: See [CONTRIBUTING.md](CONTRIBUTING.md) for complete guidelines

**Q: How do I run tests?**
A: Run `cargo test` or see [CLAUDE.md](../CLAUDE.md#testing--quality) for test commands

**Q: What's the code structure?**
A: See Architecture Overview in [CLAUDE.md](../CLAUDE.md#architecture-overview)

**Q: How do I run code coverage?**
A: Run `cargo llvm-cov` or `cargo llvm-cov --html` for HTML reports. See [CLAUDE.md](../CLAUDE.md#code-coverage)

## Response Times

We aim to:

- Acknowledge issues/discussions within 48 hours
- Respond to questions within 1 week
- Fix critical bugs as priority
- Review PRs within 1-2 weeks

Note: This is a community project. Response times may vary.

## Commercial Support

This is an open-source project. Commercial support is not currently available.

## Community Guidelines

When asking for help:

- **Be specific** - Include exact error messages, versions, configurations
- **Provide context** - What were you trying to do? What happened instead?
- **Include details** - Environment, deployment method, relevant logs
- **Be patient** - Maintainers and community volunteers help in their free time
- **Be respectful** - Follow the [Code of Conduct](CODE_OF_CONDUCT.md)
- **Search first** - Check if your question was already answered
- **Give back** - Help others when you can

## Bug Report Best Practices

When reporting bugs, include:

- Rust version (`rustc --version`)
- Deployment environment (Docker/K8s/Local)
- Exact error messages
- Steps to reproduce
- Expected vs actual behavior
- Relevant configuration (redact secrets!)
- Logs (redact sensitive info!)

Use the [Bug Report Template](.github/ISSUE_TEMPLATE/bug_report.yml) - it helps ensure you provide all needed information.

## Additional Resources

### Rust Resources

- [Rust Documentation](https://doc.rust-lang.org/)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Axum Documentation](https://docs.rs/axum/latest/axum/)

### Related Projects

- [image-rs](https://github.com/image-rs/image) - Image processing library
- [SQLx](https://github.com/launchbadge/sqlx) - Async SQL toolkit
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [Tokio](https://tokio.rs/) - Async runtime

## Still Need Help?

If you can't find an answer:

1. Check [Discussions](https://github.com/Recipe-Web-App/media-management-service/discussions)
2. Ask a new question in [Q&A](https://github.com/Recipe-Web-App/media-management-service/discussions/new?category=q-a)
3. For bugs, create an [Issue](https://github.com/Recipe-Web-App/media-management-service/issues/new/choose)

We're here to help!

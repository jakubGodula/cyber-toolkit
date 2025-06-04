# CI/CD Pipeline Documentation

This document describes the Continuous Integration and Continuous Deployment (CI/CD) pipeline setup for the cyber-toolkit project.

## Overview

The CI/CD pipeline is built using GitHub Actions and includes:

- **Automated Testing**: Runs on multiple Rust versions (stable, beta, nightly)
- **Code Quality**: Format checking and linting with clippy
- **Security**: Dependency vulnerability scanning
- **Coverage**: Code coverage reporting
- **Release Management**: Automated binary builds and releases
- **Dependency Management**: Automated dependency updates
- **Containerization**: Docker image building and publishing

## Workflows

### 1. Main CI/CD Pipeline (`.github/workflows/ci.yml`)

**Triggers:**
- Push to `master`, `main`, or `develop` branches
- Pull requests to `master`, `main`, or `develop` branches
- Published releases

**Jobs:**

#### Test Suite
- Runs on Ubuntu with Rust stable, beta, and nightly
- Code formatting check (`cargo fmt`)
- Linting with clippy (`cargo clippy`)
- Building and running tests
- Documentation tests
- Caching for faster builds

#### Security Audit
- Uses `cargo-audit` to check for known vulnerabilities
- Runs independently of main test suite

#### Code Coverage
- Generates coverage reports using `cargo-llvm-cov`
- Uploads results to Codecov (requires `CODECOV_TOKEN` secret)

#### Release Binary Builds
- Triggers only on published releases
- Builds for multiple targets:
  - `x86_64-unknown-linux-gnu`
  - `x86_64-unknown-linux-musl`
  - `x86_64-pc-windows-gnu`
  - `x86_64-apple-darwin`
  - `aarch64-apple-darwin`
- Packages and uploads binaries as release assets

#### Docker Image
- Builds and optionally pushes Docker images
- Pushes to registry on releases or master branch commits
- Uses multi-stage build for optimization

### 2. Release Management (`.github/workflows/release.yml`)

**Triggers:**
- Manual workflow dispatch with version input

**Features:**
- Semantic version validation
- Automatic `Cargo.toml` version bumping
- Git tag creation
- Automated changelog generation
- GitHub release creation

**Usage:**
1. Go to Actions tab in GitHub
2. Select "Release" workflow
3. Click "Run workflow"
4. Enter version (e.g., `v1.0.0`)
5. Optionally mark as prerelease

### 3. Dependency Management (`.github/workflows/dependencies.yml`)

**Triggers:**
- Scheduled: Every Monday at 9:00 AM UTC
- Manual workflow dispatch

**Features:**
- Automatic dependency updates
- Security vulnerability scanning
- Automated pull request creation for updates
- Security issue creation for vulnerabilities

## Local Development

### Development Script

Use the provided development script for local testing:

```bash
# Make script executable (first time only)
chmod +x scripts/dev.sh

# Setup development environment
./scripts/dev.sh setup

# Run all checks (formatting, linting, tests, audit)
./scripts/dev.sh check

# Individual commands
./scripts/dev.sh fmt     # Check formatting
./scripts/dev.sh clippy  # Run linting
./scripts/dev.sh test    # Run tests
./scripts/dev.sh audit   # Security audit
./scripts/dev.sh build   # Build release binary
```

### Required Tools

For full local development, install:

```bash
# Core Rust tools (included with rustup)
cargo fmt
cargo clippy

# Additional tools
cargo install cargo-audit      # Security auditing
cargo install cargo-outdated   # Check outdated dependencies
cargo install cargo-edit       # Edit Cargo.toml
cargo install cargo-llvm-cov    # Code coverage
```

### Pre-commit Checks

Before committing, run:

```bash
./scripts/dev.sh check
```

This ensures your code will pass CI checks.

## Repository Secrets

The following secrets need to be configured in the GitHub repository:

### Required for Coverage
- `CODECOV_TOKEN`: Token for uploading coverage reports to Codecov

### Required for Docker Publishing
- `DOCKER_USERNAME`: Docker Hub username
- `DOCKER_PASSWORD`: Docker Hub password or access token

### Automatically Available
- `GITHUB_TOKEN`: Automatically provided by GitHub Actions

## Docker

### Building Locally

```bash
# Build the Docker image
docker build -t cyber-toolkit .

# Run the container
docker run --rm cyber-toolkit

# Run with custom command
docker run --rm cyber-toolkit --version
```

### Multi-stage Build

The Dockerfile uses a multi-stage build:
1. **Builder stage**: Compiles the Rust application
2. **Runtime stage**: Creates minimal runtime image with just the binary

This results in a much smaller final image size.

## Monitoring and Notifications

### GitHub Actions
- All workflow runs are visible in the Actions tab
- Failed runs will show detailed logs
- Email notifications can be configured in GitHub settings

### Codecov
- Coverage reports are available at codecov.io
- Pull requests will show coverage changes

### Security
- Dependabot can be enabled for additional security scanning
- Security advisories will be created for critical vulnerabilities

## Troubleshooting

### Common Issues

1. **Build Failures**
   - Check Rust version compatibility
   - Verify all dependencies are available
   - Review error logs in Actions tab

2. **Test Failures**
   - Run tests locally first: `cargo test`
   - Check for environment-specific issues
   - Verify test data and fixtures

3. **Coverage Issues**
   - Ensure `CODECOV_TOKEN` is set correctly
   - Check that coverage job has necessary permissions

4. **Release Failures**
   - Verify version format (must be semantic versioning)
   - Check that all CI checks pass first
   - Ensure repository has release permissions

### Getting Help

- Check GitHub Actions logs for detailed error messages
- Review this documentation for configuration details
- Consult the official GitHub Actions documentation
- Review Rust/Cargo documentation for tooling issues

## Customization

### Adding New Targets

To add new build targets, edit `.github/workflows/ci.yml`:

```yaml
strategy:
  matrix:
    target:
      - x86_64-unknown-linux-gnu
      - your-new-target-here
```

### Modifying Checks

Adjust the clippy configuration in `ci.yml`:

```yaml
- name: Run clippy
  run: cargo clippy --all-targets --all-features -- -D warnings -A clippy::specific_lint
```

### Changing Schedule

Modify the cron schedule in `dependencies.yml`:

```yaml
schedule:
  - cron: '0 9 * * 1'  # Every Monday at 9 AM UTC
```

## Best Practices

1. **Keep workflows simple**: Avoid overly complex logic in workflow files
2. **Use caching**: Cache dependencies to speed up builds
3. **Fail fast**: Configure jobs to fail quickly on errors
4. **Security first**: Always audit dependencies and scan for vulnerabilities
5. **Documentation**: Keep this documentation updated as the pipeline evolves
6. **Testing**: Test workflow changes on feature branches first
7. **Monitoring**: Regularly review workflow execution times and success rates


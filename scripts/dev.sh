#!/bin/bash

# Development helper script for cyber-toolkit

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to run format check
run_format() {
    print_status "Checking code formatting..."
    if cargo fmt --all -- --check; then
        print_success "Code formatting is correct"
    else
        print_warning "Code formatting issues found. Run 'cargo fmt' to fix."
        return 1
    fi
}

# Function to run clippy
run_clippy() {
    print_status "Running clippy lints..."
    if cargo clippy --all-targets --all-features -- -D warnings; then
        print_success "No clippy issues found"
    else
        print_error "Clippy found issues"
        return 1
    fi
}

# Function to run tests
run_tests() {
    print_status "Running tests..."
    if cargo test --verbose; then
        print_success "All tests passed"
    else
        print_error "Some tests failed"
        return 1
    fi
}

# Function to run security audit
run_audit() {
    print_status "Running security audit..."
    if command_exists cargo-audit; then
        if cargo audit; then
            print_success "No security vulnerabilities found"
        else
            print_warning "Security vulnerabilities detected"
            return 1
        fi
    else
        print_warning "cargo-audit not installed. Run 'cargo install cargo-audit' to enable security auditing."
    fi
}

# Function to build release
build_release() {
    print_status "Building release binary..."
    if cargo build --release; then
        print_success "Release build completed successfully"
        echo "Binary location: target/release/cyber-toolkit"
    else
        print_error "Release build failed"
        return 1
    fi
}

# Function to run all checks
run_all_checks() {
    print_status "Running all development checks..."
    
    local failed=0
    
    run_format || failed=1
    run_clippy || failed=1
    run_tests || failed=1
    run_audit || failed=1
    
    if [ $failed -eq 0 ]; then
        print_success "All checks passed! ✅"
    else
        print_error "Some checks failed! ❌"
        return 1
    fi
}

# Function to setup development environment
setup_dev() {
    print_status "Setting up development environment..."
    
    # Install required tools
    print_status "Installing development tools..."
    
    if ! command_exists cargo-audit; then
        print_status "Installing cargo-audit..."
        cargo install cargo-audit
    fi
    
    if ! command_exists cargo-outdated; then
        print_status "Installing cargo-outdated..."
        cargo install cargo-outdated
    fi
    
    if ! command_exists cargo-edit; then
        print_status "Installing cargo-edit..."
        cargo install cargo-edit
    fi
    
    print_success "Development environment setup complete!"
}

# Function to show help
show_help() {
    echo "Development helper script for cyber-toolkit"
    echo ""
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  fmt        Check code formatting"
    echo "  clippy     Run clippy lints"
    echo "  test       Run tests"
    echo "  audit      Run security audit"
    echo "  build      Build release binary"
    echo "  check      Run all checks (fmt, clippy, test, audit)"
    echo "  setup      Setup development environment"
    echo "  help       Show this help message"
    echo ""
}

# Main script logic
case "${1:-}" in
    "fmt")
        run_format
        ;;
    "clippy")
        run_clippy
        ;;
    "test")
        run_tests
        ;;
    "audit")
        run_audit
        ;;
    "build")
        build_release
        ;;
    "check")
        run_all_checks
        ;;
    "setup")
        setup_dev
        ;;
    "help" | "--help" | "-h")
        show_help
        ;;
    "")
        print_error "No command specified"
        show_help
        exit 1
        ;;
    *)
        print_error "Unknown command: $1"
        show_help
        exit 1
        ;;
esac


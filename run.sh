#!/bin/bash

# CAI - Prompt Manager CLI Tool
# Run script for development and usage

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Function to show usage
show_usage() {
    cat << EOF
🤖 CAI - Prompt Manager CLI Tool

Usage: $0 [OPTIONS] [COMMAND] [ARGS...]

OPTIONS:
    -h, --help     Show this help message
    -b, --build    Build the project before running
    -t, --test     Run tests before executing
    -r, --release  Build and run in release mode
    -c, --clean    Clean build artifacts before building

COMMANDS:
    list                    List all available prompts
    search <query>          Search prompts by keyword
    show <file_name>        Show details of a specific prompt file
    query <file> <subject> <prompt>  Query a specific prompt
    chat                    Start interactive chat mode

EXAMPLES:
    $0 list                                    # List all prompts
    $0 search "debugging"                      # Search for debugging prompts
    $0 show bug_fixing                         # Show bug_fixing prompt file
    $0 query bug_fixing "General" "Error logs" # Query specific prompt
    $0 chat                                    # Start chat mode
    
    $0 --build list                            # Build first, then list
    $0 --test --release chat                   # Test, build release, then chat

ENVIRONMENT VARIABLES:
    OPENROUTER_API_KEY     Required for chat mode
    CAI_PROMPTS_DIR        Default prompts directory (default: ./prompts)
    CAI_LOG_LEVEL          Log level: TRACE, DEBUG, INFO, WARN, ERROR (default: INFO)

For chat mode, get your API key from: https://openrouter.ai/

LOGGING:
    Set CAI_LOG_LEVEL=DEBUG to see detailed operation logs
    Set CAI_LOG_LEVEL=TRACE to see all internal operations
    
    Example: CAI_LOG_LEVEL=DEBUG ./run.sh search "test"
EOF
}

# Function to build the project
build_project() {
    local mode="$1"
    print_info "Building CAI project..."
    
    if [ "$mode" = "release" ]; then
        cargo build --release
        print_success "Release build completed"
    else
        cargo build
        print_success "Debug build completed"
    fi
}

# Function to run tests
run_tests() {
    print_info "Running tests..."
    cargo test
    print_success "All tests passed"
}

# Function to clean build artifacts
clean_project() {
    print_info "Cleaning build artifacts..."
    cargo clean
    print_success "Clean completed"
}

# Function to check if binary exists
check_binary() {
    local mode="$1"
    local binary_path
    
    if [ "$mode" = "release" ]; then
        binary_path="./target/release/cai"
    else
        binary_path="./target/debug/cai"
    fi
    
    if [ ! -f "$binary_path" ]; then
        print_warning "Binary not found at $binary_path"
        print_info "Building project..."
        build_project "$mode"
    fi
    
    echo "$binary_path"
}

# Function to run the CAI binary
run_cai() {
    local binary_path="$1"
    shift  # Remove binary_path from arguments
    
    # Set default prompts directory if not set
    export CAI_PROMPTS_DIR="${CAI_PROMPTS_DIR:-./prompts}"
    
    # Check if prompts directory exists for non-help commands
    if [ $# -gt 0 ] && [ "$1" != "--help" ] && [ "$1" != "-h" ]; then
        local prompts_dir="${CAI_PROMPTS_DIR}"
        
        # Check if --directory is specified in arguments
        for ((i=1; i<=$#; i++)); do
            if [ "${!i}" = "--directory" ] || [ "${!i}" = "-d" ]; then
                ((i++))
                prompts_dir="${!i}"
                break
            fi
        done
        
        if [ ! -d "$prompts_dir" ]; then
            print_warning "Prompts directory not found: $prompts_dir"
            print_info "Creating prompts directory..."
            mkdir -p "$prompts_dir"
            
            # Create a sample prompt file
            cat > "$prompts_dir/sample.yaml" << 'EOF'
name: "Sample Prompts"
description: "Sample prompt collection to get you started"
subjects:
  - name: "General"
    prompts:
      - title: "Welcome prompt"
        content: "Welcome to CAI! This is a sample prompt to help you get started."
        score: 0
        id: "welcome-001"
EOF
            print_success "Created sample prompts directory with example file"
        fi
    fi
    
    print_info "Running CAI: $binary_path $*"
    "$binary_path" "$@"
}

# Parse command line arguments
BUILD=false
TEST=false
RELEASE=false
CLEAN=false
SHOW_HELP=false

# Process flags
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            SHOW_HELP=true
            shift
            ;;
        -b|--build)
            BUILD=true
            shift
            ;;
        -t|--test)
            TEST=true
            shift
            ;;
        -r|--release)
            RELEASE=true
            shift
            ;;
        -c|--clean)
            CLEAN=true
            shift
            ;;
        *)
            break  # Stop processing flags, rest are CAI arguments
            ;;
    esac
done

# Show help if requested
if [ "$SHOW_HELP" = true ]; then
    show_usage
    exit 0
fi

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Cargo.toml not found. Please run this script from the CAI project root directory."
    exit 1
fi

# Verify this is the CAI project
if ! grep -q 'name = "prompt-manager"' Cargo.toml || ! grep -q 'name = "cai"' Cargo.toml; then
    print_error "This doesn't appear to be the CAI project directory."
    exit 1
fi

# Execute requested actions
if [ "$CLEAN" = true ]; then
    clean_project
fi

if [ "$TEST" = true ]; then
    run_tests
fi

if [ "$BUILD" = true ]; then
    if [ "$RELEASE" = true ]; then
        build_project "release"
    else
        build_project "debug"
    fi
fi

# Determine build mode and check binary
if [ "$RELEASE" = true ]; then
    BINARY_PATH=$(check_binary "release")
else
    BINARY_PATH=$(check_binary "debug")
fi

# If no CAI arguments provided, show help
if [ $# -eq 0 ]; then
    show_usage
    echo
    print_info "To see CAI-specific help: $0 --help"
    echo
    print_info "Quick start examples:"
    echo "  $0 list              # List all available prompts"
    echo "  $0 search 'debug'    # Search for debugging-related prompts"
    echo "  $0 chat              # Start interactive chat mode (requires OPENROUTER_API_KEY)"
else
    # Run CAI with the provided arguments
    run_cai "$BINARY_PATH" "$@"
fi
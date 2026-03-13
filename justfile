# Invoice Pilot - Just commands
# A simpler alternative to Makefile for common development tasks

set shell := ["bash", "-c"]

# Display help message
help:
  @echo "Invoice Pilot - Just Commands"
  @echo "=============================="
  @echo ""
  @echo "Quick Start:"
  @echo "  just run           - Build release binary and run TUI"
  @echo "  just dev           - Run in development mode (faster builds)"
  @echo ""
  @echo "Build Commands:"
  @echo "  just build         - Build release binary only"
  @echo "  just check         - Run cargo check (fast compilation check)"
  @echo ""
  @echo "Database:"
  @echo "  just start-db      - Start PostgreSQL database with Docker"
  @echo "  just stop-db       - Stop PostgreSQL database"
  @echo ""
  @echo "Maintenance:"
  @echo "  just clean         - Clean all build artifacts"
  @echo ""
  @echo "First Time Setup:"
  @echo "  1. Create .env file: cp .env.example .env"
  @echo "  2. Edit .env with your Google API credentials"
  @echo "  3. Run: just run"
  @echo ""
  @echo "Note: Set up Google API credentials in .env file first!"

# Start PostgreSQL database with Docker
start-db:
  @echo "Starting PostgreSQL database..."
  cd docker && docker-compose -f compose-postgres.yml --env-file .env up -d
  @echo "Waiting for database to be ready..."
  @sleep 5

# Stop PostgreSQL database
stop-db:
  @echo "Stopping PostgreSQL database..."
  cd docker && docker-compose -f compose-postgres.yml stop
  @echo "Database stopped!"

# Build release binary (with PostgreSQL)
build:
  @echo "Building Invoice Pilot release binary..."
  cargo build --release --features postgres

# Run cargo check (fast compilation check)
check:
  @echo "Running cargo check..."
  cargo check --features postgres

# Clean build artifacts
clean:
  @echo "Cleaning build artifacts..."
  cargo clean
  @echo "Clean complete!"

# Build and run release binary (with PostgreSQL)
run: start-db build
  @echo "Starting Invoice Pilot TUI..."
  ./target/release/invoicepilot

# Run in development mode (with PostgreSQL, faster builds)
dev: start-db
  @echo "Starting Invoice Pilot in development mode..."
  cargo run --features postgres

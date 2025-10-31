.PHONY: run dev build clean help check

# Detect OS for cross-platform support
ifeq ($(OS),Windows_NT)
    BINARY := target/release/invoice-pilot.exe
    RM := del /Q
    RMDIR := rmdir /S /Q
else
    UNAME_S := $(shell uname -s)
    BINARY := target/release/invoice-pilot
    RM := rm -f
    RMDIR := rm -rf
endif

# Default target
help:
	@echo "Invoice Pilot - Make Commands"
	@echo "=============================="
	@echo ""
	@echo "Quick Start:"
	@echo "  make run           - Build release binary and run TUI"
	@echo "  make dev           - Run in development mode (faster builds)"
	@echo ""
	@echo "Build Commands:"
	@echo "  make build         - Build release binary only"
	@echo "  make check         - Run cargo check (fast compilation check)"
	@echo ""
	@echo "Maintenance:"
	@echo "  make clean         - Clean all build artifacts"
	@echo ""
	@echo "First Time Setup:"
	@echo "  1. Create .env file: cp .env.example .env"
	@echo "  2. Edit .env with your Google API credentials"
	@echo "  3. Run: make run"
	@echo ""
	@echo "Note: Set up Google API credentials in .env file first!"

# Build release binary and run
run: build
	@echo "Starting Invoice Pilot TUI..."
	./$(BINARY)

# Run in development mode (faster builds)
dev:
	@echo "Starting Invoice Pilot in development mode..."
	cargo run

# Build release binary
build:
	@echo "Building Invoice Pilot release binary..."
	cargo build --release

# Run cargo check (fast compilation check)
check:
	@echo "Running cargo check..."
	cargo check

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	@echo "Clean complete!"

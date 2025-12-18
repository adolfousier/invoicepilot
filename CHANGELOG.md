# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.23] - 2025-12-18

### Changed
- **OAuth Flow Refactoring**: Consolidated auth URL sending into `perform_oauth_flow` function for cleaner code architecture
- **Browser Failure Handling**: OAuth flow now detects when browser fails to open and sends notification to TUI
- **Auth Popup UI**: Removed emojis from authentication popup for cleaner display, changed text alignment to left for better URL readability
- **Demo Screenshot**: Updated README to use new static demo.png instead of animated GIF

### Added
- **Browser Failure Notifications**: New `__GMAIL_BROWSER_FAILED__` and `__DRIVE_BROWSER_FAILED__` message handlers in TUI to inform users when automatic browser opening fails
- **Enhanced Drive Logging**: Added detailed logging for folder search and creation operations in drive/folder.rs

### Fixed
- **Auth URL Display**: URLs now display with left alignment and no trimming for easier copying
- **OAuth URL Timing**: Auth URLs are now sent to TUI immediately from the OAuth flow rather than after authorization completes

### Technical Improvements
- **Code Cleanup**: Refactored `authorize_gmail` and `authorize_drive` to accept optional channel parameter for URL sending
- **Unified OAuth Handling**: Both Gmail and Drive auth now use consistent pattern with service-specific prefixes (GMAIL_, DRIVE_)

## [0.1.22] - 2025-11-07

### Changed
- **Build Tool Migration**: Replaced Makefile with `just` command runner for cleaner, simpler task management
- **Environment File Location**: `.env` file must now be placed in `docker/` directory for proper Docker and application initialization
- **Documentation**: Updated README with Just installation instructions and correct `.env` placement guidance

### Added
- **justfile**: New command runner with recipes for common tasks (run, dev, build, check, clean, start-db, stop-db, help)
- **Wise API Configuration**: Added optional Wise API settings to `.env.example` for bank integration
- **Just Badge**: Added Just technology badge to README alongside other dependencies

### Removed
- **Makefile**: Deprecated in favor of `just` for better cross-platform compatibility and simpler syntax

## [0.1.21] - 2025-11-06

### Added
- **Ratatui TUI Redesign**: Complete terminal UI overhaul with modern Ratatui framework for better interactivity
- **PostgreSQL Database Integration**: Activity logs now persisted to PostgreSQL database for audit trail
- **Detailed Activity Log Viewer**: Full-screen scrollable popup (50% screen size) showing all processing logs with Up/Down/PageUp/PageDown navigation
- **Real-time Progress Display**: Current processing step displays dynamically updated progress messages from activity log
- **Full-height Calendar Widget**: Enhanced calendar view for date selection in Manual Processing mode
- **Yellow Title Bar**: Updated app title "Invoice Pilot - Interactive Mode" with yellow color (RGB 255, 255, 0) and matching border
- **MPSC Channel Logging**: All operations (Gmail search, attachment download, Drive folder creation, file upload) now send real-time progress messages via unbounded MPSC channel to Activity Log

### Fixed
- **Compilation Errors**: Resolved type mismatches in upload.rs and jobs.rs for optional transaction parameters
- **UI Print Statement Corruption**: Eliminated 58 println!/eprintln! statements that were corrupting TUI display during processing - all now routed through MPSC channel to Activity Log
- **Log Visibility**: Activity log now shows real-time messages from all processing operations instead of static "Initializing..."
- **Dialog Margins**: Added left margin (3 units) to Activity Log panel and detailed logs dialog for better visual separation
- **Manual Processing Dialog**: Changed icon from ⚠️ to ⚡ (lightning bolt) to fix emoji rendering issue

### Changed
- **Processing Flow**: Processing now sends granular progress messages with emojis (🔍, ✓, ⬇️, ⬆️, 🏦) for better visibility
- **Database Structure**: Activity logs stored in PostgreSQL with timestamp and ID fields for better tracking
- **UI Color Scheme**: Title bar changed from cyan (RGB 0, 100, 100) to yellow (RGB 255, 255, 0)
- **Terminal UI Mode**: Default behavior now uses Ratatui interactive mode instead of legacy CLI
- **Legacy Support**: Manual CLI mode still available as subcommand but uses same drive/gmail modules

### Technical Improvements
- **Async Logging**: Log persistence uses tokio::spawn for non-blocking database writes
- **Optional Message Channel**: Drive operations (folder creation, file upload) now accept Option<&mpsc::UnboundedSender<String>> to support both TUI and CLI modes
- **Error Handling**: Comprehensive error messages sent to Activity Log with proper formatting and indentation
- **Code Organization**: Separated concerns between TUI operations (with logging) and CLI operations (with println fallback)

## [0.1.2] - 2025-10-13

### Fixed
- **Environment Loading**: Fixed `.env` file loading to support multiple locations (root, docker/, parent directory)
- **Manual Mode**: Made `FETCH_INVOICES_DAY` optional for manual mode - only required for scheduled mode
- **Docker Compose**: Fixed `env_file` configuration to properly load environment variables in containers
- **Configuration Priority**: Added intelligent .env file discovery (current dir → docker/.env → ../.env)

### Changed
- **Docker Compose**: Updated to use `env_file` instead of volume mounting for environment variables
- **Scheduling Logic**: Enhanced validation to require `FETCH_INVOICES_DAY` only for scheduled execution
- **README**: Removed duplicate sections and improved documentation clarity
- **Features**: Added automatic monthly scheduling to features list

### Technical Improvements
- **Config Module**: Made `fetch_invoices_day` an `Option<u8>` type for flexible configuration
- **Error Messages**: Improved error messaging for missing configuration in scheduled mode
- **Build Configuration**: Updated docker-compose.yml with correct build context and dockerfile path

## [0.1.1] - 2025-10-13

### Added
- **Comprehensive Financial Institution Detection**: Support for banks, brokerages, exchanges, and payment processors
- **Capitalized Folder Names**: Bank/institution folders now use proper capitalization (e.g., "Stripe", "Interactive Brokers", "Coinbase")
- **Expanded Bank Support**: Added support for 100+ European banks, brokerages, and cryptocurrency exchanges
- **Enhanced Keyword Support**: Added "financial", "fiscal", "tributary" keywords for better detection
- **Interactive Brokers Detection**: Full support for Interactive Brokers and other trading platforms
- **Cryptocurrency Exchange Support**: Added Coinbase, Binance, Kraken, and other major exchanges
- **Payment Processor Support**: Added Stripe, PayPal, Adyen, Mollie and other payment services
- **Improved Documentation**: Comprehensive README with contribution guidelines and supported institutions

### Changed
- **Renamed Features**: "Bank detection" → "Financial institution detection"
- **Enhanced Folder Organization**: Proper capitalization for all institution folders
- **Updated Configuration**: Enhanced .env.example with comprehensive examples
- **Improved Search Logic**: Better Gmail search with expanded keyword support

### Technical Improvements
- **Capitalization Logic**: Added title case conversion for detected institution names
- **Enhanced Pattern Matching**: Improved bank name detection from email headers
- **Better Error Handling**: Enhanced error messages for institution detection failures

## [0.1.0] - 2025-10-13

### Added
- **Initial Release**: Basic Gmail invoice fetching and Google Drive upload functionality
- **Dual Account Support**: Separate Gmail and Google Drive account authentication
- **OAuth2 Authentication**: Secure authentication with token caching and auto-refresh
- **Email Search**: Gmail search for invoices with configurable keywords
- **File Upload**: Automatic upload to Google Drive with duplicate detection
- **Scheduled Execution**: Support for monthly scheduled runs
- **Manual Execution**: Support for custom date ranges
- **Configuration Management**: Environment-based configuration with .env support
- **Error Handling**: Comprehensive error handling and logging
- **Filename Prefixing**: Smart filename generation with sender names

### Features
- **Monthly Folder Organization**: Automatic monthly folder creation
- **Duplicate Detection**: Skip existing files to prevent duplicates
- **Token Management**: Automatic token refresh and caching
- **CLI Interface**: User-friendly command-line interface
- **Security**: OAuth2 PKCE flow for enhanced security
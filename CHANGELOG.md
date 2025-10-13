# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
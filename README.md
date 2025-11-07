[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Ratatui](https://img.shields.io/badge/ratatui-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://ratatui.rs)
[![Docker](https://img.shields.io/badge/docker-%23000000.svg?style=for-the-badge&logo=docker&logoColor=white)](https://docker.com)
[![Just](https://img.shields.io/badge/just-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://github.com/casey/just)
[![PostgreSQL](https://img.shields.io/badge/postgresql-%23000000.svg?style=for-the-badge&logo=postgresql&logoColor=white)](https://www.postgresql.org)

[![Invoice Pilot](https://img.shields.io/badge/invoicepilot-7f56da)](https://meetneura.ai) [![Powered by Neura AI](https://img.shields.io/badge/Powered%20by-Neura%20AI-7f56da)](https://meetneura.ai)

# Invoice Pilot

Invoice Pilot is a fully automated invoice and bank statement management tool built with Rust. This project is completely free to use, modify, and distribute under the MIT License.

## Demo

![Demo](src/screenshots/invoice-pilot-demo.gif)

## Table of Contents

- [What It Does](#what-it-does)
- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [How It Works](#how-it-works)
- [Supported Financial Institutions](#supported-financial-institutions)
- [Scheduling](#scheduling)
- [Troubleshooting](#troubleshooting)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)

## What It Does

✅ **Fetches invoices and bank statements from Gmail**  
✅ **Automatically detects financial institutions** (banks, brokerages, exchanges, payment processors)  
✅ **Downloads all attachments** from matching emails  
✅ **Organizes files by institution** in Google Drive with proper capitalization  
✅ **Creates smart filenames** with sender names (e.g., `langfuse-gmbh-invoice-12345.pdf`)  
✅ **Prevents duplicates** by checking existing files  
✅ **Runs manually or on schedule**

## Features

- **Dual Google account support** (separate accounts for Gmail and Drive)
- **OAuth2 authentication with token caching**
- **Automatic token refresh**
- **Gmail search** for invoices/faturas/bank statements with attachments
- **Smart filename prefixing** with sender names
- **Automatic financial institution detection** for banks, brokerages, exchanges, and payment processors
- **Financial institution folder organization** (separate folders per institution with proper capitalization)
- **Google Drive upload** with automatic folder creation
- **Manual and scheduled execution modes** with Docker-based automation
- **Automatic monthly scheduling** - runs on configured day without user interaction
- **Duplicate detection and skipping**
- **Comprehensive error handling and logging**

## Prerequisites

### 1. Rust and Cargo

- Install Rust: <https://rustup.rs/>
- Cargo will be installed automatically with Rust

### 2. Just (Command Runner)

- Install Just: <https://github.com/casey/just#installation>
- Or: `cargo install just@1.43.0`

### 3. Google Cloud Project Setup

You need **TWO** Google Cloud projects (or one project with two OAuth2 clients):

#### For Gmail Account (Account A - Email Source)

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select existing one
3. Enable **Gmail API**:
   - Navigate to "APIs & Services" > "Library"
   - Search for "Gmail API"
   - Click "Enable"
4. Create OAuth2 Credentials:
   - Go to "APIs & Services" > "Credentials"
   - Click "Create Credentials" > "OAuth client ID"
   - Application type: **Desktop app**
   - Name: "invoice pilot - Gmail"
   - Click "Create"
   - Save the **Client ID** and **Client Secret**
5. Configure OAuth consent screen:
   - Go to "APIs & Services" > "OAuth consent screen"
   - Add scope: `https://www.googleapis.com/auth/gmail.readonly`

#### For Google Drive Account (Account B - Storage Destination)

1. Create another project or use the same project
2. Enable **Google Drive API**:
   - Navigate to "APIs & Services" > "Library"
   - Search for "Google Drive API"
   - Click "Enable"
3. Create OAuth2 Credentials:
   - Go to "APIs & Services" > "Credentials"
   - Click "Create Credentials" > "OAuth client ID"
   - Application type: **Desktop app**
   - Name: "invoice pilot - Drive"
   - Click "Create"
   - Save the **Client ID** and **Client Secret**
4. Configure OAuth consent screen:
   - Go to "APIs & Services" > "OAuth consent screen"
   - Add scope: `https://www.googleapis.com/auth/drive.file`

## Installation

### Quick Start with Just

```bash
# Clone the repository
git clone https://github.com/adolfousier/invoicepilot.git
cd invoice-pilot

# Build and run the project
just run

# The binary will be at target/release/invoice-pilot
```

### Manual Build with Cargo

If you prefer not to use `just`, you can build manually:

```bash
cargo build --release
./target/release/invoice-pilot
```

## Configuration

1. Copy the example environment file to the docker directory:

   ```bash
   cp .env.example docker/.env
   ```

   **Important**: The `.env` file must be placed inside the `docker/` directory for the database and application to work correctly.

2. Edit `docker/.env` and fill in your credentials:

   ```env
   # Gmail account credentials (Account A)
   GOOGLE_GMAIL_CLIENT_ID=your-gmail-client-id.apps.googleusercontent.com
   GOOGLE_GMAIL_CLIENT_SECRET=your-gmail-client-secret

   # Drive account credentials (Account B)
   GOOGLE_DRIVE_CLIENT_ID=your-drive-client-id.apps.googleusercontent.com
   GOOGLE_DRIVE_CLIENT_SECRET=your-drive-client-secret

   # Drive folder path (will be created if it doesn't exist)
   GOOGLE_DRIVE_FOLDER_LOCATION=billing/all-expenses/2025

   # Day of month to fetch invoices (1-31)
   FETCH_INVOICES_DAY=5

   # Keywords to search for (comma-separated)
   TARGET_KEYWORDS_TO_FETCH_AND_DOWNLOAD="invoice, invoices, fatura, faturas, statement, bank, extrato, movimientos, financial, fiscal, tributary"
   ```

## Usage

**🚀 Default Mode**: The interactive TUI is now the default and recommended way to use Invoice Pilot. Simply run `cargo run` with no arguments.

### Interactive TUI Mode (Recommended)

The interactive TUI (Terminal User Interface) provides a user-friendly, guided experience for managing your invoice processing:

```bash
cargo run
# or explicitly:
cargo run -- tui
```

#### TUI Features

- **4-Panel Dashboard**: Manual Processing, Authentication, Scheduled Mode, and Activity Log
- **Interactive calendar widget** in Scheduled Mode panel showing current month with highlighted scheduled days
- **Visual menu navigation** with keyboard controls (Tab to switch panels)
- **Real-time progress display** during processing with live updates
- **Interactive date input** with validation (YYYY-MM-DD format)
- **OAuth authentication flow** with URL display in dedicated popup
- **Results summary** with detailed breakdowns by bank/institution
- **Animated authentication status** indicators with progress bars for Gmail and Drive
- **Context-sensitive help** (press ? for help, Esc for setup guide)
- **Error handling** with clear error messages and recovery options

#### TUI Navigation

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch between panels |
| `Enter` | Open configuration popup for current panel |
| `Esc` | Close popup or quit application |
| `Q` | Quit application |
| `?` | Show help or setup guide |

#### Panel-Specific Controls

**Manual Processing Panel:**
- `Enter`: Start processing or configure dates
- `R`: Reset dates and results
- `C`: Cancel processing (when running)

**Authentication Panel:**
- `G`: Authenticate Gmail account
- `D`: Authenticate Google Drive account
- `R`: Reset all authentication tokens

**Scheduled Mode Panel:**
- **Calendar View**: Shows current month with scheduled day highlighted in yellow
- `Enter`: Configure scheduled processing day
- `S`: Trigger manual scheduled run

**Activity Log Panel:**
- Read-only activity feed with timestamps

#### First-Time TUI Setup

1. **Launch TUI**: Run `just dev` (or `cargo run` if you don't have `just` installed)
2. **Setup Guide**: If no `.env` file exists, press `?` for the setup guide
3. **Configure Environment**: Follow the setup guide to create your `.env` file
4. **Authenticate Services**:
   - Navigate to Authentication panel (Tab key)
   - Press `G` to authenticate Gmail
   - Press `D` to authenticate Google Drive
   - Copy the displayed OAuth URL and complete authorization in browser
5. **Configure Processing**:
   - Switch to Manual Processing panel
   - Press `Enter` to set date range
   - Use `Tab` to switch between start/end dates
   - Type dates in YYYY-MM-DD format
6. **Run Processing**: Press `Enter` in Manual Processing panel to start

#### TUI Workflow

1. **Authentication**: Set up Gmail and Drive access (one-time setup)
2. **Date Configuration**: Specify the date range for invoice search
3. **Processing**: Monitor real-time progress as emails are searched and files uploaded
4. **Results Review**: Check the Activity Log and Manual Processing panels for results
5. **Scheduled Setup**: Configure automatic monthly processing in Scheduled Mode panel

#### TUI Error Handling

- **Configuration Errors**: Setup guide appears automatically
- **Authentication Failures**: Clear error messages with retry options
- **Processing Errors**: Detailed error logs in Activity panel
- **Network Issues**: Automatic retry with user notification
- **Invalid Input**: Field validation with helpful error messages

The TUI provides a complete, professional interface for invoice processing with guided setup, real-time feedback, and comprehensive error handling.

### Legacy CLI Mode

For scripting or automation, the original CLI mode is still available:

#### First-Time Setup

On first run, you'll need to authorize both accounts:

```bash
cargo run -- manual
```

This will:

1. Open a browser for Gmail authorization (Account A)
2. Open a browser for Drive authorization (Account B)
3. Cache tokens locally at `~/.config/invoice-pilot/`
4. Fetch invoices from the previous month and upload to Drive

#### Manual Execution

##### Fetch invoices from previous month

```bash
cargo run -- manual
```

##### Fetch invoices from custom date range

```bash
cargo run -- manual --date-range 2024-09-01:2024-10-12
```

### Scheduled Execution

Run on a schedule using systemd timer or cron:

```bash
cargo run -- scheduled
```

This will only execute if today matches `FETCH_INVOICES_DAY` from `.env`.

### Authentication Management

#### Re-authenticate Gmail

```bash
cargo run -- auth gmail
```

#### Re-authenticate Drive

```bash
cargo run -- auth drive
```

#### Clear all tokens

```bash
cargo run -- auth reset
```

## How It Works

### 1. Gmail Search & Fetching

- **Searches Gmail** for emails containing your configured keywords (invoice, fatura, statement, bank, etc.)
- **Downloads ALL attachments** from matching emails
- **Creates smart filenames** with sender names (e.g., `langfuse-gmbh-invoice-12345.pdf`)

### 2. Automatic Financial Institution Detection

- **Identifies banks, brokerages, exchanges, and payment processors** from email content
- **Organizes files by institution** in separate folders with proper capitalization
- **Supports 100+ European banks, Wise, Revolut, Coinbase, Stripe, PayPal, and more**
- **Uses keywords** like "bank", "banco", "statement", "financial", "fiscal", "tributary"

### 3. Google Drive Upload & Organization

- **Creates monthly folders** automatically (e.g., `2025/`, `2024/`)
- **Creates institution-specific folders** (e.g., `Stripe/`, `Wise/`, `Coinbase/`)
- **Uploads files** with proper organization
- **Prevents duplicates** by checking existing files

## Supported Financial Institutions

### Digital Banks & Payment Services

- Wise (formerly TransferWise)
- Revolut
- Nubank
- Bunq
- Monzo
- Starling Bank
- Chime
- PayPal
- Stripe
- Adyen
- Mollie

### Traditional Banks

- Santander
- BBVA
- CaixaBank
- ING
- Deutsche Bank
- HSBC
- Barclays
- And many more European banks

### Brokerages & Trading Platforms

- Interactive Brokers
- Charles Schwab
- E*TRADE
- TD Ameritrade
- Fidelity
- Robinhood
- Webull

### Cryptocurrency Exchanges

- Coinbase
- Binance
- Kraken
- Coinbase Pro
- Binance US

## Automated Execution

If `FETCH_INVOICES_DAY` is set in your `.env` file, Invoice Pilot can run automatically on the specified day of each month. The `scheduled` command will automatically spin up a Docker container to execute the job, ensuring isolation and reliability.

In automated mode, cached OAuth tokens are used, so no user interaction or browser opening is required. The job runs in a container with mounted volumes for configuration and tokens.

To use automated execution:

1. Build the Docker image: `cd docker && docker-compose build`
2. Run the scheduled command: `cargo run -- scheduled` (or `./target/release/invoice-pilot scheduled`)

If you prefer external scheduling, you can still set up systemd timers or cron jobs as described below.

### Option 1: Systemd Timer (Linux)

1. Create the service file `/etc/systemd/system/invoice-pilot.service`:

   ```ini
   [Unit]
   Description=Invoice Pilot

   [Service]
   Type=oneshot
   User=your-username
   WorkingDirectory=/path/to/invoice-pilot
   ExecStart=/path/to/invoice-pilot/target/release/invoice-pilot scheduled
   Environment="PATH=/usr/local/bin:/usr/bin:/bin"
   ```

2. Create the timer file `/etc/systemd/system/invoice-pilot.timer`:

   ```ini
   [Unit]
   Description=Invoice Pilot Monthly Check

   [Timer]
   OnCalendar=*-*-{FETCH_INVOICES_DAY}
   Persistent=true

   [Install]
   WantedBy=timers.target
   ```

   Replace `{FETCH_INVOICES_DAY}` with your configured day (e.g., `05` for the 5th of each month).

3. Enable and start the timer:

   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable invoice-pilot.timer
   sudo systemctl start invoice-pilot.timer

   # Check status
   sudo systemctl status invoice-pilot.timer
   ```

### Option 2: Cron Job

Add to your crontab (`crontab -e`):

```cron
# Run on the configured day of each month at 9 AM
0 9 {FETCH_INVOICES_DAY} * * cd /path/to/invoice-pilot && /path/to/invoice-pilot/target/release/invoice-pilot scheduled >> /var/log/invoice-pilot.log 2>&1
```

Replace `{FETCH_INVOICES_DAY}` with your configured day (e.g., `5` for the 5th of each month).

## Troubleshooting

### Port 8080 Already in Use

The OAuth callback uses port 8080. If it's in use:

- Close any running instances of the tool
- Check for other services using port 8080
- Kill the process: `lsof -ti:8080 | xargs kill -9`

### Authorization Errors

If you get authorization errors:

1. Check that APIs are enabled in Google Cloud Console
2. Verify OAuth2 scopes are configured correctly
3. Re-authenticate: `cargo run -- auth reset`
4. Make sure redirect URI is set to `http://localhost:8080` in Google Cloud Console

### Token Expired

Tokens auto-refresh, but if you encounter issues:

```bash
cargo run -- auth reset
cargo run -- manual
```

### No Invoices or Bank Statements Found

- Check the date range
- Verify your Gmail account has emails matching the search criteria
- Try searching manually in Gmail with the query shown in logs
- Ensure your keywords include terms like "statement", "bank", or specific bank names
- Check if bank statements are being sent to a different email address

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture
```

### Project Structure

```
src/
├── auth/               # OAuth2 authentication
│   ├── oauth.rs        # Base OAuth2 logic
│   ├── gmail_auth.rs   # Gmail-specific auth
│   └── drive_auth.rs   # Drive-specific auth
├── gmail/              # Gmail API client
│   ├── client.rs       # HTTP client
│   ├── search.rs       # Email search with bank detection
│   └── attachment.rs   # Attachment download with sender/bank extraction
├── drive/              # Google Drive API client
│   ├── client.rs       # HTTP client
│   ├── folder.rs       # Folder management
│   └── upload.rs       # File upload
├── scheduler/          # Scheduling logic
│   └── runner.rs       # Date calculations
├── config/             # Configuration
│   └── env.rs          # .env parsing
├── cli/                # CLI interface
│   └── args.rs         # Argument parsing
└── main.rs             # Application entry point
```

## Security Notes

- Never commit `.env` file or tokens to version control
- Tokens are stored in `~/.config/invoice-pilot/` with user-only permissions
- OAuth2 uses PKCE for enhanced security
- All API calls use HTTPS

## Contributing

We welcome contributions from everyone! This is an open-source project, and we value any help you can provide.

### 🤝 How to Contribute

1. **Fork the repository** on GitHub
2. **Create a feature branch**: `git checkout -b feature/amazing-feature`
3. **Make your changes** with tests if applicable
4. **Commit your changes**: `git commit -m 'feat: add amazing feature'`
5. **Push to the branch**: `git push origin feature/amazing-feature`
6. **Open a Pull Request** with a clear description of your changes

### 📝 Development Guidelines

- **Code Style**: Follow Rust best practices and existing code style
- **Tests**: Add tests for new functionality when possible
- **Documentation**: Update documentation for new features
- **Breaking Changes**: Clearly mark breaking changes in PR descriptions

### 🐛 Reporting Issues

- Use the GitHub issue tracker to report bugs
- Include steps to reproduce, expected behavior, and actual behavior
- For feature requests, describe the problem and proposed solution

### 🌟 Suggested Contributions

- **New Bank/Institution Support**: Add detection for new financial institutions
- **Improved Error Handling**: Enhance error messages and recovery
- **Performance Optimizations**: Speed up Gmail search or file uploads
- **Documentation**: Improve README or add usage examples
- **Testing**: Add more comprehensive test coverage

### 📚 Getting Started for Contributors

1. Clone the repository: `git clone https://github.com/adolfousier/invoicepilot.git`
2. Set up development environment: `cargo build`
3. Run existing tests: `cargo test`
4. Make your changes
5. Test thoroughly: `cargo build && cargo test`
6. Submit your pull request

Thank you for contributing to Invoice Pilot! 🚀

## License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

**This software is completely free to use, modify, and distribute for any purpose.**

## Support

For issues, questions, or contributions, please open an issue on GitHub.

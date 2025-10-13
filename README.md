# Invoice Pilot

Invoice Pilot is a **free and open-source** terminal-based automation tool built with Rust. This project is completely free to use, modify, and distribute under the MIT License.

## What It Does

Invoice Pilot is a **fully automated invoice and bank statement management tool** that:

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
- **Manual and scheduled execution modes**
- **Duplicate detection and skipping**
- **Comprehensive error handling and logging**

## Prerequisites

### 1. Rust and Cargo
- Install Rust: https://rustup.rs/
- Cargo will be installed automatically with Rust

### 2. Google Cloud Project Setup

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

## How It Works

Invoice Pilot automatically **fetches invoices and bank statements from Gmail** and **uploads them to Google Drive** with intelligent organization:

### 1. Gmail Invoice & Bank Statement Fetching
- **Searches Gmail** for emails containing your configured keywords (invoice, fatura, statement, bank, etc.)
- **Downloads attachments** from matching emails
- **Detects financial institutions** automatically (banks, brokerages, exchanges, payment processors)
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
- **Prevents duplicates** by checking if files already exist

## Installation

### Using Cargo

```bash
# Clone the repository
git clone <your-repo-url>
cd invoice-agent

# Build the project
cargo build --release

# The binary will be at target/release/invoice-agent
```

### Configuration

1. Copy the example environment file:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` and fill in your credentials:
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

### First-Time Setup

On first run, you'll need to authorize both accounts:

```bash
cargo run -- manual
```

This will:
1. Open a browser for Gmail authorization (Account A)
2. Open a browser for Drive authorization (Account B)
3. Cache tokens locally at `~/.config/invoice-agent/`
4. Fetch invoices from the previous month and upload to Drive

### Manual Execution

#### Fetch invoices from previous month:
```bash
cargo run -- manual
```

#### Fetch invoices from custom date range:
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

#### Re-authenticate Gmail:
```bash
cargo run -- auth gmail
```

#### Re-authenticate Drive:
```bash
cargo run -- auth drive
```

#### Clear all tokens:
```bash
cargo run -- auth reset
```

## Scheduling Options

### Option 1: Systemd Timer (Linux)

1. Create the service file `/etc/systemd/system/invoice-agent.service`:
   ```ini
   [Unit]
   Description=invoice pilot

   [Service]
   Type=oneshot
   User=your-username
   WorkingDirectory=/path/to/invoice-agent
   ExecStart=/path/to/invoice-agent/target/release/invoice-agent scheduled
   Environment="PATH=/usr/local/bin:/usr/bin:/bin"
   ```

2. Create the timer file `/etc/systemd/system/invoice-agent.timer`:
   ```ini
   [Unit]
   Description=invoice pilot Monthly Check

   [Timer]
   OnCalendar=daily
   Persistent=true

   [Install]
   WantedBy=timers.target
   ```

3. Enable and start the timer:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable invoice-agent.timer
   sudo systemctl start invoice-agent.timer

   # Check status
   sudo systemctl status invoice-agent.timer
   ```

### Option 2: Cron Job

Add to your crontab (`crontab -e`):

```cron
# Run daily at 9 AM
0 9 * * * cd /path/to/invoice-agent && /path/to/invoice-agent/target/release/invoice-agent scheduled >> /var/log/invoice-agent.log 2>&1
```

## How It Works

### Gmail Invoice and Bank Statement Processing

1. **Authentication**: Uses OAuth2 with PKCE flow for secure authentication
2. **Token Caching**: Stores access tokens locally and auto-refreshes when expired
3. **Gmail Search**: Searches for emails containing:
   - Keywords: "faturas", "fatura", "invoice", "invoices", "statement", "bank" (customizable)
   - Has attachment
   - Within specified date range
4. **Bank Detection**: Automatically identifies bank statements from email content:
   - Detects European banks: Wise, Revolut, Nubank, Santander, BBVA, CaixaBank, ING, etc.
   - Looks for bank names, "banco", "bank" keywords
   - No additional configuration needed
5. **Download**: Downloads all matching attachments with smart filenames:
   - Extracts sender name from email headers
   - Sanitizes and prepends to filename
   - Example: `17821893723.pdf` → `langfuse-gmbh-17821893723.pdf`
6. **Upload**: Uploads to Google Drive with:
   - Automatic monthly folder creation
   - Automatic bank-specific subfolder creation
   - Duplicate detection (skips if file already exists)
7. **Cleanup**: Removes temporary files after upload

### Bank Statement Processing

1. **Bank Detection**: Automatically identifies bank names from email headers and content
2. **Folder Organization**: Creates separate folders for each detected bank
3. **Smart Filenames**: Uses bank name + sender name + original filename
    - Example: `wise-statement-eur-20241001-2024-10.pdf`
    - Example: `revolut-extract-monthly-september.pdf`
4. **Multi-format**: Supports PDF, CSV, and other document types

## Gmail Search Query

The tool searches Gmail using your configured keywords:
```
(keyword1 OR keyword2 OR keyword3 ...) has:attachment after:YYYY/MM/DD before:YYYY/MM/DD
```

Default keywords: `invoice`, `invoices`, `fatura`, `faturas`, `statement`, `bank`

**Note**: The tool downloads **ALL attachments** from matching emails. This ensures maximum coverage for both invoices and bank statements.

## Folder Structure

Folders are created automatically based on `GOOGLE_DRIVE_FOLDER_LOCATION`:

```
Google Drive (Root)
└── billing
    └── all-expenses
        └── 2025
            ├── langfuse-gmbh-invoice-2024-10.pdf
            ├── aws-statement-november.pdf
            ├── Wise/
            │   └── wise-statement-eur-20241001-2024-10.pdf
            ├── Revolut/
            │   └── revolut-extract-monthly-september.pdf
            └── Santander/
                └── banco-santander-extract-october.pdf
```

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

### Project Structure

```
src/
├── auth/               # OAuth2 authentication
│   ├── oauth.rs        # Base OAuth2 logic
│   ├── gmail_auth.rs   # Gmail-specific auth
│   └── drive_auth.rs   # Drive-specific auth
├── gmail/              # Gmail API client
│   ├── client.rs       # HTTP client
│   ├── search.rs       # Email search
│   └── attachment.rs   # Attachment download with sender extraction
├── drive/              # Google Drive API client
│   ├── client.rs       # HTTP client
│   ├── folder.rs       # Folder management
│   └── upload.rs       # File upload
├── gmail/              # Gmail API client
│   ├── client.rs       # HTTP client
│   ├── search.rs       # Email search with bank detection
│   └── attachment.rs   # Attachment download with sender/bank extraction
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
- Tokens are stored in `~/.config/invoice-agent/` with user-only permissions
- OAuth2 uses PKCE for enhanced security
- All API calls use HTTPS

## License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

**This software is completely free to use, modify, and distribute for any purpose.**

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

1. Clone the repository: `git clone <your-fork-url>`
2. Set up development environment: `cargo build`
3. Run existing tests: `cargo test`
4. Make your changes
5. Test thoroughly: `cargo build && cargo test`
6. Submit your pull request

Thank you for contributing to Invoice Pilot! 🚀

## Automated Invoice Sending for Services

### Setting Up Automated Invoice Receipt

Many services like Wise, Revolut, and other digital banks offer automated invoice and statement sending via email. To enable automatic receipt of these documents:

1. **Check Service Settings**: Go to your service provider's settings (Wise, Revolut, Nubank, etc.)
2. **Enable Email Notifications**: Ensure invoice/statement notifications are enabled
3. **Use Dedicated Email**: Set up a dedicated Gmail account for receiving these documents
4. **Configure Keywords**: Add relevant keywords to your `TARGET_KEYWORDS_TO_FETCH_AND_DOWNLOAD`

### Supported Services

**Digital Banks & Payment Services:**
- Wise (formerly TransferWise)
- Revolut
- Nubank
- Bunq
- Monzo
- Starling Bank
- Chime
- PayPal

**Traditional Banks:**
- Santander
- BBVA
- CaixaBank
- ING
- Deutsche Bank
- HSBC
- Barclays
- And many more European banks

### Email Setup Tips

1. **Forwarding Rules**: Set up email forwarding if statements go to different addresses
2. **Filter Labels**: Use Gmail filters to automatically label bank emails
3. **Regular Checks**: Monitor that emails are being received correctly
4. **Keyword Optimization**: Add service-specific keywords to improve detection

### Example Configuration

```env
# Include financial institution keywords for better detection
TARGET_KEYWORDS_TO_FETCH_AND_DOWNLOAD="invoice, fatura, statement, wise, revolut, nubank, santander, bank, extrato, movimientos, financial, fiscal, tributary, interactive brokers, coinbase, stripe, paypal"
```

## Support

For issues, questions, or contributions, please open an issue on GitHub.

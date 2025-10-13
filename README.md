# Invoice Pilot

Invoice Pilot is an terminal based tool built with Rust that automatically searches on Gmail for invoice attachments and uploads them to Google Drive. Supports both manual and scheduled execution.

## Features

- Dual Google account support (separate accounts for Gmail and Drive)
- OAuth2 authentication with token caching
- Automatic token refresh
- Search Gmail for invoices/faturas with PDF attachments
- Upload PDFs to organized Google Drive folders
- Manual and scheduled execution modes
- Duplicate detection and skipping
- Comprehensive error handling and logging

## Prerequisites

### 1. Google Cloud Project Setup

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
   TARGET_KEYWORDS_TO_FETCH_AND_DOWNLOAD="invoice, invoices, fatura, faturas"
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

1. **Authentication**: Uses OAuth2 with PKCE flow for secure authentication
2. **Token Caching**: Stores access tokens locally and auto-refreshes when expired
3. **Gmail Search**: Searches for emails containing:
   - Keywords: "faturas", "fatura", "invoice", or "invoices"
   - Has attachment
   - File type: PDF
   - Within specified date range
4. **Download**: Downloads all matching PDF attachments
5. **Upload**: Uploads to Google Drive with:
   - Automatic folder creation (nested folders supported)
   - Duplicate detection (skips if file already exists)
6. **Cleanup**: Removes temporary files after upload

## Gmail Search Query

The tool searches Gmail using your configured keywords:
```
(keyword1 OR keyword2 OR keyword3 ...) has:attachment after:YYYY/MM/DD before:YYYY/MM/DD
```

Default keywords: `invoice`, `invoices`, `fatura`, `faturas`

**Note**: The tool downloads **ALL attachments** from matching emails, not just PDFs. This ensures maximum coverage.

## Folder Structure

Folders are created automatically based on `GOOGLE_DRIVE_FOLDER_LOCATION`:

```
Google Drive (Root)
└── billing
    └── all-expenses
        └── 2025
            ├── invoice1.pdf
            ├── invoice2.pdf
            └── ...
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

### No Invoices Found

- Check the date range
- Verify your Gmail account has emails matching the search criteria
- Try searching manually in Gmail with the query shown in logs

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
│   ├── search.rs       # Email search
│   └── attachment.rs   # Attachment download
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
- Tokens are stored in `~/.config/invoice-agent/` with user-only permissions
- OAuth2 uses PKCE for enhanced security
- All API calls use HTTPS

## License

MIT

## Support

For issues, questions, or contributions, please open an issue on GitHub.

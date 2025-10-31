use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Table, Row, Wrap,
    },
    Frame,
};

use crate::app::{App, AuthStatus, FocusedPanel, PopupState};
use log::info;


pub fn draw(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Create main dashboard layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(1),    // Dashboard content
            Constraint::Length(3), // Footer
        ])
        .split(size);

    // Title
    let title = draw_title();
    frame.render_widget(title, chunks[0]);

    // Dashboard with multiple panels
    draw_dashboard(frame, app, chunks[1]);

    // Footer
    let footer = draw_footer(app);
    frame.render_widget(footer, chunks[2]);

    // Handle popups - error popups take precedence over main popups
    if let Some(error) = &app.error_message {
        draw_error_popup(frame, error);
    } else if app.is_popup_open() {
        draw_popup(frame, app);
    }
}

fn draw_title() -> Paragraph<'static> {
    Paragraph::new("🚀 Invoice Pilot - Interactive Mode")
        .style(Style::default().fg(Color::Rgb(0, 100, 100)).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(0, 100, 100))))
}

fn draw_footer(app: &App) -> Paragraph<'static> {
    let panel_name = match app.focused_panel {
        FocusedPanel::Manual => "Manual Processing",
        FocusedPanel::Auth => "Authentication",
        FocusedPanel::Scheduled => "Scheduled Mode",
        FocusedPanel::Logs => "Activity Log",
    };

    let help_key = if app.config.is_none() { "?: Setup" } else { "?: Help" };
    let footer_text = format!(
        "Focused: {} | Tab: Switch Panel | Enter: Configure | {} | ESC/Q: Quit | {}",
        panel_name,
        help_key,
        match app.focused_panel {
            FocusedPanel::Manual => {
                if app.is_processing {
                    "C: Cancel Processing"
                } else {
                    "Enter: Run | R: Reset | Type: Input Dates"
                }
            }
            FocusedPanel::Auth => "G: Gmail Auth | D: Drive Auth | C/R: Clear All",
            FocusedPanel::Scheduled => "Enter: Configure Schedule | S: Manual Trigger",
            FocusedPanel::Logs => "Read-only",
        }
    );

    Paragraph::new(footer_text)
        .style(Style::default().fg(Color::Rgb(180, 80, 0))) // Dark Orange text
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(180, 80, 0))))
}

fn draw_error_popup(frame: &mut Frame, error: &str) {
    let area = centered_rect(60, 20, frame.area());
    frame.render_widget(Clear, area);

    let error_widget = Paragraph::new(error)
        .style(Style::default().fg(Color::Red))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("❌ Error")
                .style(Style::default().fg(Color::Red))
        );

    frame.render_widget(error_widget, area);
}



fn draw_dashboard(frame: &mut Frame, app: &mut App, area: Rect) {
    // Add dark gray background to the entire dashboard
    let background = Paragraph::new("")
        .style(Style::default().bg(Color::Rgb(30, 30, 30))); // Dark Gray
    frame.render_widget(background, area);

    // Create a 2x2 grid layout for the dashboard
    let dashboard_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Top row
            Constraint::Percentage(50), // Bottom row
        ])
        .split(area);

    let top_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Left: Manual processing
            Constraint::Percentage(50), // Right: Auth status
        ])
        .split(dashboard_chunks[0]);

    let bottom_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Left: Scheduled mode
            Constraint::Percentage(50), // Right: Logs/Results
        ])
        .split(dashboard_chunks[1]);

    // Top-left: Manual processing panel
    draw_manual_panel(frame, app, top_row[0]);

    // Top-right: Auth status panel
    draw_auth_panel(frame, app, top_row[1]);

    // Bottom-left: Scheduled mode panel
    draw_scheduled_panel(frame, app, bottom_row[0]);

    // Bottom-right: Logs/Results panel
    draw_logs_panel(frame, app, bottom_row[1]);
}

fn draw_manual_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    // Create the panel block with title at top left
    let panel_block = Block::default()
        .borders(Borders::ALL)
        .title("📅 Manual Processing")
        .title_style(if matches!(app.focused_panel, FocusedPanel::Manual) {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        })
        .border_style(if matches!(app.focused_panel, FocusedPanel::Manual) {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        });

    // Split the inner area for content
    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Date range display
            Constraint::Length(3), // Status
            Constraint::Min(1),    // Progress/Results
        ])
        .split(inner_area);

    // Render the panel border first
    frame.render_widget(panel_block, area);

    // Date range display
    let date_range_text = if app.is_date_input_valid() {
        format!("{} to {}", app.start_date_input, app.end_date_input)
    } else {
        "Not configured".to_string()
    };

    let date_display = Paragraph::new(date_range_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Date Range"))
        .style(if app.is_date_input_valid() {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Red)
        });
    frame.render_widget(date_display, chunks[0]);

    // Status
    let status = if app.is_processing {
        Paragraph::new("🔄 Processing...")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL))
    } else {
        let auth_ok = matches!(app.gmail_auth_status, AuthStatus::Authenticated) &&
                      matches!(app.drive_auth_status, AuthStatus::Authenticated);
        let auto_scheduled = app.fetch_invoices_day.is_some();
        let ready = app.is_date_input_valid() && auth_ok;

        let (status_text, status_style) = if ready {
            ("Ready", Style::default().fg(Color::Green))
        } else if !auth_ok {
            ("Auth Required", Style::default().fg(Color::Red))
        } else if auto_scheduled {
            ("Auto Scheduled", Style::default().fg(Color::Yellow))
        } else {
            ("Configure Dates", Style::default().fg(Color::Red))
        };

        Paragraph::new(status_text)
            .style(status_style)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL))
    };
    frame.render_widget(status, chunks[1]);

    // Progress/Results area
    if app.is_processing {
        // Show progress messages
        let messages: Vec<ListItem> = app.progress_messages
            .iter()
            .rev() // Show newest first
            .take(5) // Show last 5 messages
            .map(|msg| ListItem::new(msg.as_str()))
            .collect();

        let progress_list = List::new(messages)
            .block(Block::default().borders(Borders::ALL).title("Progress"));
        frame.render_widget(progress_list, chunks[2]);
    } else if app.total_processed > 0 {
        // Show results summary
        let summary_text = format!(
            "✅ Complete\n\n{} processed\n{} uploaded\n{} failed",
            app.total_processed,
            app.total_uploaded,
            app.total_failed
        );

        let summary = Paragraph::new(summary_text)
            .style(Style::default().fg(Color::Green))
            .block(Block::default().borders(Borders::ALL).title("Results"));
        frame.render_widget(summary, chunks[2]);
    } else {
        let placeholder = Paragraph::new("")
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(placeholder, chunks[2]);
    }
}

fn draw_auth_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    // Create the panel block with title at top left
    let panel_block = Block::default()
        .borders(Borders::ALL)
        .title("🔐 Authentication")
        .title_style(if matches!(app.focused_panel, FocusedPanel::Auth) {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        })
        .border_style(if matches!(app.focused_panel, FocusedPanel::Auth) {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        });

    // Split the inner area for Gmail and Drive status
    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Gmail status
            Constraint::Length(4), // Drive status
            Constraint::Min(1),    // Empty space
        ])
        .split(inner_area);

    // Render the panel border first
    frame.render_widget(panel_block, area);

    // Gmail status with animated progress bar
    let gmail_widget = create_auth_progress_bar("Gmail", &app.gmail_auth_status, app.animation_counter, false);
    frame.render_widget(gmail_widget, chunks[0]);

    // Drive status with animated progress bar
    let drive_widget = create_auth_progress_bar("Google Drive", &app.drive_auth_status, app.animation_counter, true);
    frame.render_widget(drive_widget, chunks[1]);

    // Empty space
    let empty = Paragraph::new("");
    frame.render_widget(empty, chunks[2]);
}

fn draw_scheduled_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let panel_block = Block::default()
        .borders(Borders::ALL)
        .title("⏰ Automatic Schedule")
        .title_style(if matches!(app.focused_panel, FocusedPanel::Scheduled) {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        })
        .border_style(if matches!(app.focused_panel, FocusedPanel::Scheduled) {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        });

    // Create calendar lines for paragraph rendering
    let calendar_lines = create_calendar_lines(app, area);
    let calendar_paragraph = Paragraph::new(calendar_lines)
        .block(panel_block)
        .style(Style::default().bg(Color::Rgb(30, 30, 30)));

    frame.render_widget(calendar_paragraph, area);

    // Log scheduled job setup when both auth and schedule are configured
    let auth_ok = matches!(app.gmail_auth_status, AuthStatus::Authenticated) &&
                  matches!(app.drive_auth_status, AuthStatus::Authenticated);
    let configured = app.fetch_invoices_day.is_some();

    if auth_ok && configured && !app.scheduled_job_logged {
        if let Some(day) = app.fetch_invoices_day {
            app.add_progress_message(format!("🔄 Automatic job scheduled: Will run on day {} of each month when triggered", day));
            info!("Scheduled job configured: Will run on day {} of each month", day);
            app.scheduled_job_logged = true;
        }
    }
}

fn draw_logs_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    // Create the panel block with title at top left
    let panel_block = Block::default()
        .borders(Borders::ALL)
        .title("📋 Activity Log")
        .title_style(if matches!(app.focused_panel, FocusedPanel::Logs) {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        })
        .border_style(if matches!(app.focused_panel, FocusedPanel::Logs) {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        });

    // Get the inner area for logs
    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    // Render the panel border first
    frame.render_widget(panel_block, area);

    // Logs
    let log_items: Vec<ListItem> = app.progress_messages
        .iter()
        .rev() // Show newest first
        .take(10) // Show last 10 messages
        .map(|msg| ListItem::new(msg.as_str()))
        .collect();

    let logs = List::new(log_items);
    frame.render_widget(logs, inner_area);
}

fn draw_popup(frame: &mut Frame, app: &mut App) {
    match app.popup_state {
        PopupState::DateInput => draw_date_input_popup(frame, app),
        PopupState::ScheduleConfig => draw_schedule_config_popup(frame, app),
        PopupState::AuthConfirm => draw_auth_confirm_popup(frame, app),
        PopupState::GmailAuthUrl => draw_gmail_auth_url_popup(frame, app),
        PopupState::DriveAuthUrl => draw_drive_auth_url_popup(frame, app),
        PopupState::ProcessingConfirm => draw_processing_confirm_popup(frame, app),
        PopupState::Help => draw_help_popup(frame),
        PopupState::SetupGuide => draw_setup_guide_popup(frame),
        PopupState::None => {} // Should not happen
    }
}

fn create_colored_background(frame: &mut Frame, area: Rect, color: Color) {
    // Clear the area first to remove background content
    frame.render_widget(Clear, area);

    // Create a solid colored background with border that fills the entire popup area
    let background = Paragraph::new("")
        .style(Style::default().bg(color))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(background, area);
}

fn draw_date_input_popup(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 50, frame.area());
    create_colored_background(frame, area, Color::Rgb(0, 0, 100)); // Dark Blue

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(6), // Date inputs
            Constraint::Length(3), // Instructions
            Constraint::Length(3), // Controls
        ])
        .split(area);

    // Title
    let title = Paragraph::new("📅 Configure Date Range")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    // Date inputs (compact)
    let date_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[1]);

    // Start date input
    let start_style = if app.date_input_focus {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let start_date = Paragraph::new(format!("Start Date: {}", app.start_date_input))
        .style(start_style)
        .alignment(Alignment::Center);
    frame.render_widget(start_date, date_chunks[0]);

    // End date input
    let end_style = if !app.date_input_focus {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let end_date = Paragraph::new(format!("End Date: {}", app.end_date_input))
        .style(end_style)
        .alignment(Alignment::Center);
    frame.render_widget(end_date, date_chunks[1]);

    // Instructions
    let instructions = Paragraph::new("Enter dates in YYYY-MM-DD format. Use Tab to switch between fields.")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(instructions, chunks[2]);

    // Controls
    let controls = Paragraph::new("Enter: Save | Tab: Switch Field | Esc: Cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    frame.render_widget(controls, chunks[3]);
}

fn draw_schedule_config_popup(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 40, frame.area());
    create_colored_background(frame, area, Color::Rgb(0, 100, 0)); // Dark Green

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Instructions
            Constraint::Length(3), // Day input
            Constraint::Length(3), // Controls
        ])
        .split(area);

    // Title
    let title = Paragraph::new("⏰ Configure Scheduled Mode")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    // Instructions
    let instructions = Paragraph::new("Enter the day of month (1-31) to run invoice processing automatically.")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(instructions, chunks[1]);

    // Day input
    let day_input = Paragraph::new(format!("Day: {}", app.schedule_input))
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    frame.render_widget(day_input, chunks[2]);

    // Controls
    let controls = Paragraph::new("Enter: Save | Esc: Cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    frame.render_widget(controls, chunks[3]);
}

fn draw_auth_confirm_popup(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(80, 35, frame.area());
    create_colored_background(frame, area, Color::Rgb(100, 0, 0)); // Dark Red

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(8), // Content
            Constraint::Length(3), // Controls
        ])
        .split(area);

    // Title
    let title = Paragraph::new("🔐 Authentication Required")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    // Content
    let content = if app.gmail_auth_status == AuthStatus::Authenticating || app.drive_auth_status == AuthStatus::Authenticating {
        "🔄 Authentication in progress...\n\n🌐 Please open the authorization URL shown above in your browser.\n\nComplete the Google OAuth flow and return here.\n\nThe application will automatically detect when authentication is complete.".to_string()
    } else {
        "Authentication is required to access Google services.\n\nPress Enter to start the OAuth flow.\n\nA browser window will open (or URL will be displayed) for you to authorize access.".to_string()
    };

    let content_widget = Paragraph::new(content)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(content_widget, chunks[1]);

    // Controls
    let controls_text = if app.gmail_auth_status == AuthStatus::Authenticating || app.drive_auth_status == AuthStatus::Authenticating {
        "Esc: Close (auth running in background)"
    } else {
        "Enter: Start Authentication | Esc: Cancel"
    };

    let controls = Paragraph::new(controls_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    frame.render_widget(controls, chunks[2]);
}

fn draw_processing_confirm_popup(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(60, 30, frame.area());
    create_colored_background(frame, area, Color::Rgb(100, 100, 0)); // Dark Yellow

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(5), // Content
            Constraint::Length(3), // Controls
        ])
        .split(area);

    // Title
    let title = Paragraph::new("⚠️ Confirm Processing")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    // Content
    let content = if matches!(app.focused_panel, FocusedPanel::Scheduled) {
        "This will process invoices for the previous month.\n\nMake sure authentication is configured and dates are set."
    } else {
        "This will search Gmail for invoices and upload them to Google Drive.\n\nMake sure dates are configured and authentication is set up."
    };

    let content_widget = Paragraph::new(content)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(content_widget, chunks[1]);

    // Controls
    let controls = Paragraph::new("Enter: Start Processing | Esc: Cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    frame.render_widget(controls, chunks[2]);
}

fn draw_help_popup(frame: &mut Frame) {
    let area = centered_rect(80, 60, frame.area());
    create_colored_background(frame, area, Color::Rgb(100, 0, 100)); // Dark Magenta

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(1),    // Content
            Constraint::Length(3), // Controls
        ])
        .split(area);

    // Title
    let title = Paragraph::new("❓ Help & Instructions")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    // Content
    let help_text = r#"Invoice Pilot - Interactive Terminal Application

NAVIGATION:
• Tab/Shift+Tab: Switch between panels
• Enter: Open configuration popup for current panel
• Esc: Close popup or quit application
• Q: Quit application

PANELS:
• Manual Processing: Configure dates and run one-time processing
• Authentication: Set up Gmail and Google Drive access
• Scheduled Mode: Configure automatic monthly processing
• Activity Log: View processing progress and results

SETUP:
1. Configure your .env file with Google API credentials
2. Use Tab to navigate to Authentication panel
3. Press Enter to authenticate Gmail and Drive
4. Configure dates in Manual Processing panel
5. Run processing or set up scheduling

SHORTCUTS:
• Manual Panel: Enter to run processing, R to reset
• Auth Panel: G for Gmail auth, D for Drive auth, R to reset
• Scheduled Panel: S for manual trigger, Enter to configure
• Log Panel: Read-only activity feed"#;

    let content = Paragraph::new(help_text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    frame.render_widget(content, chunks[1]);

    // Controls
    let controls = Paragraph::new("Esc: Close Help")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    frame.render_widget(controls, chunks[2]);
}

fn draw_setup_guide_popup(frame: &mut Frame) {
    let area = centered_rect(85, 70, frame.area());
    create_colored_background(frame, area, Color::Rgb(150, 0, 150)); // Dark Magenta

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(1),    // Content
            Constraint::Length(3), // Controls
        ])
        .split(area);

    // Title
    let title = Paragraph::new("🚀 Setup Required - Invoice Pilot")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    // Content
    let setup_text = r#"⚠️  CONFIGURATION REQUIRED

Before using Invoice Pilot, you need to set up Google API credentials.

📋 SETUP STEPS:

1. 📝 Create a .env file in the project root directory
2. 🔑 Get Google API credentials from Google Cloud Console
3. 📋 Copy the required variables from .env.example
4. 🔄 Fill in your actual API credentials

📄 REQUIRED ENVIRONMENT VARIABLES:

GOOGLE_GMAIL_CLIENT_ID=your-gmail-client-id.apps.googleusercontent.com
GOOGLE_GMAIL_CLIENT_SECRET=your-gmail-client-secret
GOOGLE_DRIVE_CLIENT_ID=your-drive-client-id.apps.googleusercontent.com
GOOGLE_DRIVE_CLIENT_SECRET=your-drive-client-secret
GOOGLE_DRIVE_FOLDER_LOCATION=billing/all-expenses/2025

🔗 GOOGLE CLOUD CONSOLE SETUP:
1. Go to https://console.cloud.google.com/
2. Create a new project or select existing one
3. Enable Gmail API and Google Drive API
4. Create OAuth 2.0 credentials
5. Add your email as a test user
6. Download credentials and add to .env file

📂 The .env.example file contains all required variables with examples.

⚡ Once configured, restart the application and authentication will work!"#;

    let content = Paragraph::new(setup_text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    frame.render_widget(content, chunks[1]);

    // Controls
    let controls = Paragraph::new("Enter: Dismiss | ?: Help")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    frame.render_widget(controls, chunks[2]);
}

fn draw_gmail_auth_url_popup(frame: &mut Frame, app: &mut App) {
    draw_specific_auth_url_popup(frame, app, "Gmail", Color::Rgb(200, 50, 50)); // Red
}

fn draw_drive_auth_url_popup(frame: &mut Frame, app: &mut App) {
    draw_specific_auth_url_popup(frame, app, "Google Drive", Color::Rgb(50, 50, 200)); // Blue
}

fn draw_specific_auth_url_popup(frame: &mut Frame, app: &mut App, service_name: &str, color: Color) {
    let area = centered_rect(90, 70, frame.area());
    create_colored_background(frame, area, color);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(8), // Content (URL or success message)
            Constraint::Length(3), // Instructions
            Constraint::Length(3), // Controls
        ])
        .split(area);

    // Title
    let title = if app.auth_popup_success {
        Paragraph::new(format!("✅ {} Authenticated", service_name))
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
    } else {
        Paragraph::new(format!("🔐 Authenticate {}", service_name))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
    };
    frame.render_widget(title, chunks[0]);

    // Content display
    let content_text = if app.auth_popup_success {
        format!("🎉 {} authentication completed successfully!\n\nYour tokens are cached and ready to use.", service_name)
    } else if let Some(url) = &app.auth_url {
        format!("🌐 {} Authorization URL:\n\n{}", service_name, url)
    } else {
        format!("🔄 Preparing {} authorization URL...\n\nPlease wait while we set up the OAuth flow.", service_name)
    };

    let content_display = Paragraph::new(content_text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(content_display, chunks[1]);

    // Instructions
    let instructions = if app.auth_popup_success {
        format!("📋 Your {} authentication is active.\nYou can now close this popup or clear tokens if needed.", service_name)
    } else {
        format!("📋 INSTRUCTIONS:\n• Copy the URL above\n• Open it in your web browser\n• Complete the Google OAuth flow for {}\n• Return here when done\n• The app will detect completion automatically", service_name)
    };

    let instructions_widget = Paragraph::new(instructions)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(instructions_widget, chunks[2]);

    // Controls
    let controls_text = if app.auth_popup_success {
        "Any Key: Close | C: Clear Tokens"
    } else if app.auth_url.is_some() {
        "Esc: Close | Complete authorization in browser, then return here"
    } else {
        "Esc: Cancel | Waiting for authorization URL..."
    };

    let controls = Paragraph::new(controls_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    frame.render_widget(controls, chunks[3]);
}

fn create_calendar_lines(app: &App, area: Rect) -> Vec<Line<'static>> {
    use chrono::{Datelike, NaiveDate};

    let now = chrono::Utc::now();
    let current_year = now.year();
    let current_month = now.month();

    let month_name = chrono::Month::try_from(current_month as u8)
        .unwrap_or(chrono::Month::January)
        .name();

    let bg_color = Color::Rgb(30, 30, 30);
    let panel_width = area.width.saturating_sub(2) as usize; // Subtract borders
    let col_width = panel_width / 7; // Divide evenly across 7 columns

    let mut lines = Vec::new();

    // Title
    let title = format!("{} {}", month_name, current_year);
    lines.push(Line::from(vec![Span::styled(format!("{:<width$}", title, width = panel_width), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD).bg(bg_color))]));

    // Weekday headers - centered in each column
    let weekdays = vec!["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let weekday_line: Vec<Span> = weekdays.iter()
        .map(|day| Span::styled(format!("{:^width$}", day, width = col_width), Style::default().fg(Color::White).bg(bg_color)))
        .collect();
    lines.push(Line::from(weekday_line));

    // Get calendar data
    let first_of_month = NaiveDate::from_ymd_opt(current_year, current_month, 1).unwrap();
    let last_of_month = if current_month == 12 {
        NaiveDate::from_ymd_opt(current_year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(current_year, current_month + 1, 1).unwrap()
    }.pred_opt().unwrap();

    let first_weekday = first_of_month.weekday().num_days_from_sunday();
    let mut day = 1;

    // Calculate available space for weeks with padding between them
    let available_height = area.height.saturating_sub(4) as usize;
    let num_weeks = 6; // Standard calendar weeks
    let spacing_per_week = if num_weeks > 0 { available_height / num_weeks } else { 1 };

    // Create calendar week rows with spacing
    let mut week_count = 0;
    loop {
        if day > last_of_month.day() && week_count > 0 {
            break;
        }

        let mut week_spans = Vec::new();

        for weekday in 0..7 {
            if (week_count == 0 && weekday < first_weekday) || day > last_of_month.day() {
                week_spans.push(Span::styled(format!("{:<width$}", "", width = col_width), Style::default().bg(bg_color)));
            } else {
                let is_scheduled = app.fetch_invoices_day.map_or(false, |d| d == day as u32);
                let is_today = now.day() == day && now.month() == current_month && now.year() == current_year;

                let style = if is_scheduled && is_today {
                    Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else if is_scheduled {
                    Style::default().fg(Color::Yellow).bg(bg_color).add_modifier(Modifier::BOLD)
                } else if is_today {
                    Style::default().fg(Color::Cyan).bg(bg_color).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White).bg(bg_color)
                };

                week_spans.push(Span::styled(format!("{:^width$}", day, width = col_width), style));
                day += 1;
            }
        }

        // Add the week row
        lines.push(Line::from(week_spans));

        // Add blank lines for spacing to fill height
        for _ in 1..spacing_per_week {
            let blank_line: Vec<Span> = (0..7)
                .map(|_| Span::styled(format!("{:<width$}", "", width = col_width), Style::default().bg(bg_color)))
                .collect();
            lines.push(Line::from(blank_line));
        }

        week_count += 1;
        if week_count >= num_weeks {
            break;
        }
    }

    // Fill any remaining height
    while lines.len() < available_height + 2 {
        let blank_line: Vec<Span> = (0..7)
            .map(|_| Span::styled(format!("{:<width$}", "", width = col_width), Style::default().bg(bg_color)))
            .collect();
        lines.push(Line::from(blank_line));
    }

    lines
}

fn create_auth_progress_bar(title: &str, status: &AuthStatus, animation_counter: u32, is_drive: bool) -> Paragraph<'static> {
    let border_color = match status {
        AuthStatus::Authenticated => Color::Green,
        _ => Color::Red,
    };

    let (label, bar_color, progress_ratio) = match status {
        AuthStatus::NotAuthenticated => ("Not Authenticated", Color::Red, 0.0),
        AuthStatus::Authenticating => {
            // Create animated progress bar with offset for Drive
            let offset = if is_drive { 25 } else { 0 };
            let progress = ((animation_counter + offset) % 100) as f64 / 100.0;
            ("Authenticating...", Color::Yellow, progress)
        }
        AuthStatus::Authenticated => ("Authenticated", Color::Green, 1.0),
        AuthStatus::Error(_) => ("Error", Color::Red, 0.0),
    };

    // Create a visual progress bar using block characters
    let bar_width = 20; // Fixed width for the progress bar
    let filled_width = (bar_width as f64 * progress_ratio) as usize;
    let empty_width = bar_width - filled_width;

    let filled_bar = "█".repeat(filled_width);
    let empty_bar = "░".repeat(empty_width);

    let progress_text = format!("{} [{}{}] {:.0}%", label, filled_bar, empty_bar, progress_ratio * 100.0);

    Paragraph::new(progress_text)
        .style(Style::default().fg(bar_color))
        .block(Block::default().borders(Borders::ALL).title(title.to_string()).border_style(Style::default().fg(border_color)))
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
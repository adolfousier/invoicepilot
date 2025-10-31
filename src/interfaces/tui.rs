use crate::app::{App, AuthStatus, FocusedPanel, PopupState};
use crate::process::jobs;
use crate::interfaces::ui::draw;
use chrono::NaiveDate;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::{io, time::Duration};
use tokio::sync::mpsc;

pub async fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new();

    // Load configuration
    if let Err(e) = app.load_config() {
        app.add_progress_message(format!("Config error: {}", e));
        // Show setup guide for first-time users
        app.open_popup(PopupState::SetupGuide);
    } else {
        app.add_progress_message("Configuration loaded successfully".to_string());
        // Validate existing authentication tokens
        app.validate_existing_tokens();
    }

    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
        std::process::exit(1);
    }

    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    loop {
        terminal.draw(|f| draw(f, app))?;

        // Update animation counter for smooth animations
        app.animation_counter = (app.animation_counter + 1) % 100;

        // Handle async processing updates
        while let Ok(message) = rx.try_recv() {
            if message == "__PROCESSING_COMPLETE__" {
                app.set_processing(false);
                app.processing_step = None;
            } else if message == "__GMAIL_AUTH_SUCCESS__" {
                app.gmail_auth_status = crate::app::AuthStatus::Authenticated;
                app.add_progress_message("Gmail authentication successful".to_string());
                if matches!(app.popup_state, PopupState::GmailAuthUrl) {
                    app.close_popup();
                    // Don't auto-start Drive auth - let user do it manually
                }
            } else if message == "__GMAIL_AUTH_CACHED_SUCCESS__" {
                app.gmail_auth_status = crate::app::AuthStatus::Authenticated;
                app.add_progress_message("Gmail authentication successful (using cached tokens)".to_string());
                app.auth_popup_success = true;
                // Keep popup open to show success and allow user options
                // Don't auto-start Drive auth for cached tokens - let user do it manually
            } else if message == "__GMAIL_AUTH_REFRESH_SUCCESS__" {
                app.gmail_auth_status = crate::app::AuthStatus::Authenticated;
                app.add_progress_message("Gmail authentication successful (tokens refreshed)".to_string());
                app.auth_popup_success = true;
                // Keep popup open to show success and allow user options
                // Don't auto-start Drive auth for refreshed tokens - let user do it manually
            } else if message.starts_with("__GMAIL_AUTH_ERROR__:") {
                let error = message.strip_prefix("__GMAIL_AUTH_ERROR__:").unwrap_or("Unknown error");
                app.gmail_auth_status = crate::app::AuthStatus::Error(error.to_string());
                app.add_progress_message(format!("Gmail authentication failed: {}", error));
                if matches!(app.popup_state, PopupState::GmailAuthUrl) {
                    app.close_popup();
                }
            } else if message.starts_with("__GMAIL_AUTH_URL__:") {
                let url = message.strip_prefix("__GMAIL_AUTH_URL__:").unwrap_or("");
                app.auth_url = Some(url.to_string());
            } else if message == "__DRIVE_AUTH_SUCCESS__" {
                app.drive_auth_status = crate::app::AuthStatus::Authenticated;
                app.add_progress_message("Google Drive authentication successful".to_string());
                if matches!(app.popup_state, PopupState::DriveAuthUrl) {
                    app.close_popup();
                }
            } else if message == "__DRIVE_AUTH_CACHED_SUCCESS__" {
                app.drive_auth_status = crate::app::AuthStatus::Authenticated;
                app.add_progress_message("Google Drive authentication successful (using cached tokens)".to_string());
                app.auth_popup_success = true;
                // Keep popup open to show success and allow user options
            } else if message == "__DRIVE_AUTH_REFRESH_SUCCESS__" {
                app.drive_auth_status = crate::app::AuthStatus::Authenticated;
                app.add_progress_message("Google Drive authentication successful (tokens refreshed)".to_string());
                app.auth_popup_success = true;
                // Keep popup open to show success and allow user options
            } else if message.starts_with("__DRIVE_AUTH_ERROR__:") {
                let error = message.strip_prefix("__DRIVE_AUTH_ERROR__:").unwrap_or("Unknown error");
                app.drive_auth_status = crate::app::AuthStatus::Error(error.to_string());
                app.add_progress_message(format!("Drive authentication failed: {}", error));
                if matches!(app.popup_state, PopupState::DriveAuthUrl) {
                    app.close_popup();
                }
            } else if message.starts_with("__DRIVE_AUTH_URL__:") {
                let url = message.strip_prefix("__DRIVE_AUTH_URL__:").unwrap_or("");
                app.auth_url = Some(url.to_string());
            } else if message.starts_with("__RESULTS__:") {
                // Parse results: processed=5,uploaded=4,failed=1,month=October,folder=Invoices/October
                let results_str = message.strip_prefix("__RESULTS__:").unwrap_or("");
                for part in results_str.split(',') {
                    let kv: Vec<&str> = part.split('=').collect();
                    if kv.len() == 2 {
                        match kv[0] {
                            "processed" => app.total_processed = kv[1].parse().unwrap_or(0),
                            "uploaded" => app.total_uploaded = kv[1].parse().unwrap_or(0),
                            "failed" => app.total_failed = kv[1].parse().unwrap_or(0),
                            "month" => app.billing_month = Some(kv[1].to_string()),
                            "folder" => app.drive_folder = Some(kv[1].to_string()),
                            _ => {}
                        }
                    }
                }
            } else if message.starts_with("__BANK_RESULT__:") {
                // Parse bank results: BankName:uploaded=3,failed=0
                let bank_result = message.strip_prefix("__BANK_RESULT__:").unwrap_or("");
                if let Some(colon_pos) = bank_result.find(':') {
                    let bank_name = &bank_result[..colon_pos];
                    let stats = &bank_result[colon_pos + 1..];
                    let mut uploaded = 0;
                    let mut failed = 0;

                    for part in stats.split(',') {
                        let kv: Vec<&str> = part.split('=').collect();
                        if kv.len() == 2 {
                            match kv[0] {
                                "uploaded" => uploaded = kv[1].parse().unwrap_or(0),
                                "failed" => failed = kv[1].parse().unwrap_or(0),
                                _ => {}
                            }
                        }
                    }

                    app.bank_breakdown.insert(bank_name.to_string(), crate::app::UploadSummary { uploaded, failed });
                }
            } else {
                app.add_progress_message(message);
            }
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Handle global keys
                    match key.code {
                        KeyCode::Tab => {
                            if app.is_popup_open() {
                                // Handle popup-specific tab navigation
                                handle_popup_tab_navigation(app);
                            } else {
                                // Handle panel navigation
                                app.focused_panel = match app.focused_panel {
                                    FocusedPanel::Manual => FocusedPanel::Auth,
                                    FocusedPanel::Auth => FocusedPanel::Scheduled,
                                    FocusedPanel::Scheduled => FocusedPanel::Logs,
                                    FocusedPanel::Logs => FocusedPanel::Manual,
                                };
                            }
                        }
                        KeyCode::BackTab => {
                            if !app.is_popup_open() {
                                app.focused_panel = match app.focused_panel {
                                    FocusedPanel::Manual => FocusedPanel::Logs,
                                    FocusedPanel::Auth => FocusedPanel::Manual,
                                    FocusedPanel::Scheduled => FocusedPanel::Auth,
                                    FocusedPanel::Logs => FocusedPanel::Scheduled,
                                };
                            }
                        }
                        KeyCode::Enter => {
                            if !app.is_popup_open() {
                                // Open popup for current panel
                                match app.focused_panel {
                                    FocusedPanel::Manual => app.open_popup(PopupState::DateInput),
                                    FocusedPanel::Auth => {
                                        // For auth panel, start the first unauthenticated service, or allow re-auth
                                        if matches!(app.gmail_auth_status, AuthStatus::NotAuthenticated) {
                                            start_gmail_auth(app, tx.clone());
                                        } else if matches!(app.drive_auth_status, AuthStatus::NotAuthenticated) {
                                            start_drive_auth(app, tx.clone());
                                        } else {
                                            // All authenticated - allow re-auth of Gmail first
                                            start_gmail_auth(app, tx.clone());
                                        }
                                    }
                                    FocusedPanel::Scheduled => app.open_popup(PopupState::ScheduleConfig),
                                    FocusedPanel::Logs => {} // No popup for logs
                                }
                            } else {
                                // Handle popup confirmation
                                handle_popup_confirm(app, &tx);
                            }
                        }
                        KeyCode::Esc => {
                            if app.is_popup_open() {
                                app.close_popup();
                            } else {
                                break; // Quit
                            }
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if !app.is_popup_open() {
                                break; // Quit
                            }
                        }
                        KeyCode::Char('?') => {
                            if !app.is_popup_open() {
                                if app.config.is_none() {
                                    app.open_popup(PopupState::SetupGuide);
                                } else {
                                    app.open_popup(PopupState::Help);
                                }
                            }
                        }
                        _ => {
                            if app.is_popup_open() {
                                // Handle popup-specific input
                                handle_popup_input(app, key.code, &tx);
                            } else {
                                // Handle panel-specific input
                                match app.focused_panel {
                                    FocusedPanel::Manual => handle_manual_input(app, key.code, tx.clone()),
                                    FocusedPanel::Auth => handle_auth_input(app, key.code, tx.clone()),
                                    FocusedPanel::Scheduled => handle_scheduled_input(app, key.code, tx.clone()),
                                    FocusedPanel::Logs => {} // Logs panel is read-only
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}





fn handle_manual_input(app: &mut App, key_code: KeyCode, tx: mpsc::UnboundedSender<String>) {
    if app.is_processing {
        // Only allow canceling during processing
        if key_code == KeyCode::Char('c') || key_code == KeyCode::Char('C') {
            app.set_processing(false);
            app.add_progress_message("Processing cancelled by user".to_string());
        }
        return;
    }

    match key_code {
        KeyCode::Enter => {
            if app.is_date_input_valid() {
                app.open_popup(PopupState::ProcessingConfirm);
            } else {
                app.open_popup(PopupState::DateInput);
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            // Reset dates
            app.reset_manual_inputs();
        }
        // Handle date input - append to current focused field
        KeyCode::Char(c) => {
            if c.is_ascii_digit() || c == '-' {
                let current_field = if app.date_input_focus {
                    &mut app.start_date_input
                } else {
                    &mut app.end_date_input
                };

                // Only allow input if field is not full (YYYY-MM-DD = 10 chars)
                if current_field.len() < 10 {
                    current_field.push(c);
                }
            }
        }
        KeyCode::Backspace => {
            let current_field = if app.date_input_focus {
                &mut app.start_date_input
            } else {
                &mut app.end_date_input
            };

            current_field.pop();
        }
        _ => {}
    }
}

fn handle_scheduled_input(app: &mut App, key_code: KeyCode, tx: mpsc::UnboundedSender<String>) {
    match key_code {
        KeyCode::Enter => {
            // Open schedule configuration popup
            app.open_popup(PopupState::ScheduleConfig);
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            // Manual trigger for scheduled processing
            app.open_popup(PopupState::ProcessingConfirm);
        }
        _ => {}
    }
}

fn start_scheduled_processing(app: &mut App, tx: mpsc::UnboundedSender<String>) {
    if app.is_processing {
        return; // Already processing
    }

    app.set_processing(true);
    app.add_progress_message("Starting scheduled invoice processing...".to_string());

    // Get previous month date range for scheduled processing
    let (start_date, end_date) = crate::scheduler::runner::get_previous_month_range();

    app.add_progress_message(format!("Processing date range: {} to {}", start_date, end_date));

    // Spawn processing task
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let result = jobs::run_manual_processing(start_date, end_date, &tx_clone).await;
        // Send completion signal
        if let Err(e) = result {
            let _ = tx.send(format!("Scheduled processing error: {}", e));
        }
        let _ = tx.send("__PROCESSING_COMPLETE__".to_string());
    });
}

fn handle_auth_input(app: &mut App, key_code: KeyCode, tx: mpsc::UnboundedSender<String>) {
    match key_code {
        KeyCode::Char('g') | KeyCode::Char('G') => {
            start_gmail_auth(app, tx.clone());
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            start_drive_auth(app, tx.clone());
        }
        KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::Char('c') | KeyCode::Char('C') => {
            app.gmail_auth_status = crate::app::AuthStatus::NotAuthenticated;
            app.drive_auth_status = crate::app::AuthStatus::NotAuthenticated;
            app.scheduled_job_logged = false; // Reset logging flag when auth is cleared
            // Clear tokens
            let _ = crate::auth::gmail_auth::clear_gmail_token();
            let _ = crate::auth::drive_auth::clear_drive_token();
            app.add_progress_message("All authentication tokens cleared".to_string());
        }
        _ => {}
    }
}

fn handle_popup_tab_navigation(app: &mut App) {
    match app.popup_state {
        PopupState::DateInput => {
            // Switch between start date and end date fields
            app.date_input_focus = !app.date_input_focus;
        }
        _ => {
            // Other popups don't have tab navigation
        }
    }
}

fn handle_popup_input(app: &mut App, key_code: KeyCode, tx: &mpsc::UnboundedSender<String>) {
    match app.popup_state {
        PopupState::DateInput => {
            match key_code {
                KeyCode::Tab => {
                    app.date_input_focus = !app.date_input_focus;
                }
                KeyCode::Char(c) => {
                    if c.is_ascii_digit() || c == '-' {
                        if app.date_input_focus {
                            if app.start_date_input.len() < 10 {
                                app.start_date_input.push(c);
                            }
                        } else {
                            if app.end_date_input.len() < 10 {
                                app.end_date_input.push(c);
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    if app.date_input_focus {
                        app.start_date_input.pop();
                    } else {
                        app.end_date_input.pop();
                    }
                }
                _ => {}
            }
        }
        PopupState::ScheduleConfig => {
            match key_code {
                KeyCode::Char(c) => {
                    if c.is_ascii_digit() && app.schedule_input.len() < 2 {
                        app.schedule_input.push(c);
                    }
                }
                KeyCode::Backspace => {
                    app.schedule_input.pop();
                }
                _ => {}
            }
        }
        PopupState::GmailAuthUrl | PopupState::DriveAuthUrl => {
            // Handle clearing tokens in success mode
            if app.auth_popup_success {
                match key_code {
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        // Clear tokens based on which service
                        match app.popup_state {
                            PopupState::GmailAuthUrl => {
                                let _ = crate::auth::gmail_auth::clear_gmail_token();
                                app.gmail_auth_status = AuthStatus::NotAuthenticated;
                                app.scheduled_job_logged = false; // Reset logging flag when auth is cleared
                                app.add_progress_message("Gmail tokens cleared".to_string());
                            }
                            PopupState::DriveAuthUrl => {
                                let _ = crate::auth::drive_auth::clear_drive_token();
                                app.drive_auth_status = AuthStatus::NotAuthenticated;
                                app.scheduled_job_logged = false; // Reset logging flag when auth is cleared
                                app.add_progress_message("Drive tokens cleared".to_string());
                            }
                            _ => {}
                        }
                        app.close_popup();
                    }
                    KeyCode::Esc => {
                        // Close popup on escape
                        app.close_popup();
                    }
                    _ => {
                        // Close popup on any other key press for success messages
                        app.close_popup();
                    }
                }
            }
        }
        _ => {} // Other popups don't need input handling
    }
}

fn handle_popup_confirm(app: &mut App, tx: &mpsc::UnboundedSender<String>) {
    match app.popup_state {
        PopupState::DateInput => {
            // Validate dates
            if app.start_date_input.len() == 10 && app.end_date_input.len() == 10 {
                if let (Ok(_), Ok(_)) = (
                    chrono::NaiveDate::parse_from_str(&app.start_date_input, "%Y-%m-%d"),
                    chrono::NaiveDate::parse_from_str(&app.end_date_input, "%Y-%m-%d")
                ) {
                    app.close_popup();
                    app.add_progress_message("Date range configured successfully".to_string());
                } else {
                    app.set_error("Invalid date format. Use YYYY-MM-DD".to_string());
                }
            } else {
                app.set_error("Please enter complete dates (YYYY-MM-DD)".to_string());
            }
        }
        PopupState::ScheduleConfig => {
            if let Ok(day) = app.schedule_input.parse::<u32>() {
                if day >= 1 && day <= 31 {
                    app.fetch_invoices_day = Some(day);
                    app.scheduled_job_logged = false; // Reset logging flag when schedule changes
                    app.close_popup();
                    app.add_progress_message(format!("Scheduled processing set for day {} of each month", day));
                } else {
                    app.set_error("Day must be between 1 and 31".to_string());
                }
            } else {
                app.set_error("Please enter a valid day number".to_string());
            }
        }
        PopupState::AuthConfirm => {
            // Start authentication based on current panel
            match app.focused_panel {
                FocusedPanel::Auth => {
                    // Check which auth button was pressed by looking at status
                    if matches!(app.gmail_auth_status, AuthStatus::NotAuthenticated) {
                        start_gmail_auth(app, tx.clone());
                    } else if matches!(app.drive_auth_status, AuthStatus::NotAuthenticated) {
                        start_drive_auth(app, tx.clone());
                    }
                }
                _ => {
                    // For other panels, trigger both auths if needed
                    if matches!(app.gmail_auth_status, AuthStatus::NotAuthenticated) {
                        start_gmail_auth(app, tx.clone());
                    }
                    if matches!(app.drive_auth_status, AuthStatus::NotAuthenticated) {
                        start_drive_auth(app, tx.clone());
                    }
                }
            }
            // Keep popup open to show progress
        }
        PopupState::ProcessingConfirm => {
            app.close_popup();
            // Start processing based on current panel
            match app.focused_panel {
                FocusedPanel::Manual => {
                    if app.is_date_input_valid() {
                        start_manual_processing(app, tx.clone());
                    } else {
                        app.set_error("Please configure valid dates first".to_string());
                    }
                }
                FocusedPanel::Scheduled => {
                    start_scheduled_processing(app, tx.clone());
                }
                _ => {}
            }
        }
        PopupState::Help => {
            app.close_popup();
        }
        PopupState::SetupGuide => {
            app.close_popup();
        }
        PopupState::GmailAuthUrl | PopupState::DriveAuthUrl => {
            // Auth URL popups are closed automatically when auth completes
        }
        PopupState::None => {} // Should not happen
    }
}

fn start_gmail_auth(app: &mut App, tx: mpsc::UnboundedSender<String>) {
    let config = app.config.clone();
    if let Some(config) = config {
        app.gmail_auth_status = AuthStatus::Authenticating;
        app.auth_popup_success = false; // Reset success flag
        app.open_popup(PopupState::GmailAuthUrl);
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            match crate::auth::gmail_auth::get_gmail_token_with_url(
                config.gmail_client_id,
                config.gmail_client_secret,
                tx_clone.clone(),
            ).await {
                Ok(_) => {
                    let _ = tx_clone.send("__GMAIL_AUTH_SUCCESS__".to_string());
                }
                Err(e) => {
                    let _ = tx_clone.send(format!("__GMAIL_AUTH_ERROR__:{}", e));
                }
            }
        });
    } else {
        app.set_error("Configuration not loaded - cannot authenticate".to_string());
    }
}

fn start_drive_auth(app: &mut App, tx: mpsc::UnboundedSender<String>) {
    let config = app.config.clone();
    if let Some(config) = config {
        app.drive_auth_status = AuthStatus::Authenticating;
        app.auth_popup_success = false; // Reset success flag
        app.open_popup(PopupState::DriveAuthUrl);
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            match crate::auth::drive_auth::get_drive_token_with_url(
                config.drive_client_id,
                config.drive_client_secret,
                tx_clone.clone(),
            ).await {
                Ok(_) => {
                    let _ = tx_clone.send("__DRIVE_AUTH_SUCCESS__".to_string());
                }
                Err(e) => {
                    let _ = tx_clone.send(format!("__DRIVE_AUTH_ERROR__:{}", e));
                }
            }
        });
    } else {
        app.set_error("Configuration not loaded - cannot authenticate".to_string());
    }
}

fn start_manual_processing(app: &mut App, tx: mpsc::UnboundedSender<String>) {
    if app.is_processing {
        return; // Already processing
    }

    // Parse dates
    let start_date = match NaiveDate::parse_from_str(&app.start_date_input, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => {
            app.set_error("Invalid start date format. Use YYYY-MM-DD".to_string());
            return;
        }
    };

    let end_date = match NaiveDate::parse_from_str(&app.end_date_input, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => {
            app.set_error("Invalid end date format. Use YYYY-MM-DD".to_string());
            return;
        }
    };

    if start_date > end_date {
        app.set_error("Start date cannot be after end date".to_string());
        return;
    }

    app.set_processing(true);
    app.add_progress_message("Starting manual invoice processing...".to_string());

    // Spawn processing task
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let result = jobs::run_manual_processing(start_date, end_date, &tx_clone).await;
        // Send completion signal
        if let Err(e) = result {
            let _ = tx.send(format!("Error: {}", e));
        }
        let _ = tx.send("__PROCESSING_COMPLETE__".to_string());
    });
}

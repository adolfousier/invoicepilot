use chrono::Utc;
use crate::config::env::Config;
use crate::db::DbPool;

#[derive(Debug, Clone, PartialEq)]
pub enum FocusedPanel {
    Manual,
    Auth,
    Scheduled,
    Logs,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PopupState {
    None,
    DateInput,
    ScheduleConfig,
    GmailAuthUrl,
    DriveAuthUrl,
    ProcessingConfirm,
    Help,
    SetupGuide,
    DetailedLogs,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthStatus {
    NotAuthenticated,
    Authenticating,
    Authenticated,
    Error(String),
}

#[derive(Debug)]
pub struct App {
    pub focused_panel: FocusedPanel,
    pub popup_state: PopupState,
    pub config: Option<Config>,
    pub db_pool: Option<DbPool>,

    // Manual mode state
    pub start_date_input: String,
    pub end_date_input: String,
    pub date_input_focus: bool, // true = start date, false = end date
    pub is_processing: bool,
    pub progress_messages: Vec<String>,
    pub processing_step: Option<String>,

    // Results
    pub total_processed: usize,
    pub total_uploaded: usize,
    pub total_failed: usize,
    pub billing_month: Option<String>,
    pub drive_folder: Option<String>,

    // Auth status
    pub gmail_auth_status: AuthStatus,
    pub drive_auth_status: AuthStatus,

    // Scheduled mode
    pub fetch_invoices_day: Option<u32>,
    pub schedule_input: String,

    // Error handling
    pub error_message: Option<String>,
    pub auth_url: Option<String>,

    // Auth popup state
    pub auth_popup_success: bool,

    // Logging state
    pub scheduled_job_logged: bool,

    // Animation state
    pub animation_counter: u32,

    // Detailed logs viewer
    pub logs_scroll_offset: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            focused_panel: FocusedPanel::Manual,
            popup_state: PopupState::None,
            config: None,
            db_pool: None,
            start_date_input: String::new(),
            end_date_input: String::new(),
            date_input_focus: true, // Start with start date focused
            is_processing: false,
            progress_messages: Vec::new(),
            processing_step: None,
            total_processed: 0,
            total_uploaded: 0,
            total_failed: 0,
            billing_month: None,
            drive_folder: None,
            gmail_auth_status: AuthStatus::NotAuthenticated,
            drive_auth_status: AuthStatus::NotAuthenticated,
            fetch_invoices_day: None,
            schedule_input: String::new(),
            error_message: None,
            auth_url: None,
            auth_popup_success: false,
            scheduled_job_logged: false,
            animation_counter: 0,
            logs_scroll_offset: 0,
        }
    }

    pub fn add_progress_message(&mut self, message: String) {
        let formatted = format!("{}: {}", Utc::now().format("%H:%M:%S"), message);
        self.progress_messages.push(formatted.clone());

        // Save to database if pool exists
        if let Some(pool) = &self.db_pool {
            let pool_clone = pool.clone();
            tokio::spawn(async move {
                let _ = crate::db::save_log(&pool_clone, &formatted).await;
            });
        }

        // Keep only last 100 messages to prevent memory issues
        if self.progress_messages.len() > 100 {
            self.progress_messages.remove(0);
        }
    }

    pub async fn load_persisted_logs(&mut self) -> anyhow::Result<()> {
        if let Some(pool) = &self.db_pool {
            let messages = crate::db::load_logs(pool).await?;
            self.progress_messages = messages;
        }
        Ok(())
    }

    pub fn set_processing(&mut self, processing: bool) {
        self.is_processing = processing;
        if processing {
            self.progress_messages.clear();
            self.processing_step = Some("Initializing...".to_string());
        } else {
            self.processing_step = None;
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.is_processing = false;
        self.processing_step = None;
    }

    pub fn clear_results(&mut self) {
        self.total_processed = 0;
        self.total_uploaded = 0;
        self.total_failed = 0;
        self.billing_month = None;
        self.drive_folder = None;
    }

    pub fn reset_manual_inputs(&mut self) {
        self.start_date_input.clear();
        self.end_date_input.clear();
        self.clear_results();
    }

    pub fn is_date_input_valid(&self) -> bool {
        !self.start_date_input.is_empty() && !self.end_date_input.is_empty() &&
        self.start_date_input.len() == 10 && self.end_date_input.len() == 10 // YYYY-MM-DD format
    }

    pub fn open_popup(&mut self, popup: PopupState) {
        self.popup_state = popup;
        self.error_message = None; // Clear any previous errors
    }

    pub fn close_popup(&mut self) {
        self.popup_state = PopupState::None;
        self.error_message = None;
    }

    pub fn is_popup_open(&self) -> bool {
        !matches!(self.popup_state, PopupState::None)
    }

    pub fn load_config(&mut self) -> Result<(), String> {
        match Config::from_env() {
            Ok(config) => {
                self.config = Some(config.clone());
                self.fetch_invoices_day = config.fetch_invoices_day.map(|d| d as u32);
                Ok(())
            }
            Err(e) => Err(format!("Failed to load config: {}", e)),
        }
    }

    /// Validate existing authentication tokens and update auth status
    pub fn validate_existing_tokens(&mut self) {
        if let Some(_config) = &self.config {
            // Check Gmail token
            match crate::auth::oauth::get_config_dir() {
                Ok(config_dir) => {
                    let gmail_token_path = config_dir.join("gmail_token.json");
                    if gmail_token_path.exists() {
                        match crate::auth::oauth::load_token(&gmail_token_path) {
                            Ok(token_cache) => {
                                if !token_cache.is_expired() {
                                    self.gmail_auth_status = AuthStatus::Authenticated;
                                    self.add_progress_message("Gmail authentication restored from cached tokens".to_string());
                                } else {
                                    // Token exists but is expired - don't change status, let user re-auth
                                }
                            }
                            Err(e) => {
                                self.add_progress_message(format!("Warning: Could not load Gmail token: {}", e));
                            }
                        }
                    }
                }
                Err(e) => {
                    self.add_progress_message(format!("Warning: Could not access config directory: {}", e));
                }
            }

            // Check Drive token
            match crate::auth::oauth::get_config_dir() {
                Ok(config_dir) => {
                    let drive_token_path = config_dir.join("drive_token.json");
                    if drive_token_path.exists() {
                        match crate::auth::oauth::load_token(&drive_token_path) {
                            Ok(token_cache) => {
                                if !token_cache.is_expired() {
                                    self.drive_auth_status = AuthStatus::Authenticated;
                                    self.add_progress_message("Google Drive authentication restored from cached tokens".to_string());
                                } else {
                                    // Token exists but is expired - don't change status, let user re-auth
                                }
                            }
                            Err(e) => {
                                self.add_progress_message(format!("Warning: Could not load Drive token: {}", e));
                            }
                        }
                    }
                }
                Err(e) => {
                    self.add_progress_message(format!("Warning: Could not access config directory: {}", e));
                }
            }
        }
    }
}
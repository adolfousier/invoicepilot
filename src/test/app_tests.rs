#[cfg(test)]
mod tests {
    use crate::app::{App, AuthStatus, FocusedPanel, PopupState};
    use crate::process::jobs::{determine_billing_month, parse_folder_years};
    use chrono::NaiveDate;

    // ── App::new defaults ──────────────────────────────────────────────

    #[test]
    fn new_app_has_correct_defaults() {
        let app = App::new();

        assert_eq!(app.focused_panel, FocusedPanel::Manual);
        assert_eq!(app.popup_state, PopupState::None);
        assert!(app.config.is_none());
        assert!(app.db_pool.is_none());
        assert!(app.start_date_input.is_empty());
        assert!(app.end_date_input.is_empty());
        assert!(app.date_input_focus); // start date focused
        assert!(!app.is_processing);
        assert!(app.progress_messages.is_empty());
        assert!(app.processing_step.is_none());
        assert_eq!(app.total_processed, 0);
        assert_eq!(app.total_uploaded, 0);
        assert_eq!(app.total_failed, 0);
        assert!(app.billing_month.is_none());
        assert!(app.drive_folder.is_none());
        assert_eq!(app.catchup_total_missing, 0);
        assert_eq!(app.catchup_total_processed, 0);
        assert_eq!(app.gmail_auth_status, AuthStatus::NotAuthenticated);
        assert_eq!(app.drive_auth_status, AuthStatus::NotAuthenticated);
        assert!(app.fetch_invoices_day.is_none());
        assert!(app.schedule_input.is_empty());
        assert!(app.error_message.is_none());
        assert!(app.auth_url.is_none());
        assert!(!app.auth_popup_success);
        assert!(!app.scheduled_job_logged);
        assert_eq!(app.animation_counter, 0);
        assert_eq!(app.logs_scroll_offset, 0);
    }

    // ── add_progress_message ───────────────────────────────────────────

    #[test]
    fn add_progress_message_appends_formatted() {
        let mut app = App::new();
        app.add_progress_message("hello".to_string());

        assert_eq!(app.progress_messages.len(), 1);
        assert!(app.progress_messages[0].contains("hello"));
        // Should have HH:MM:SS prefix
        assert!(app.progress_messages[0].contains(':'));
    }

    #[test]
    fn add_progress_message_caps_at_100() {
        let mut app = App::new();
        for i in 0..110 {
            app.add_progress_message(format!("msg-{}", i));
        }
        assert_eq!(app.progress_messages.len(), 100);
        // Oldest messages should have been removed
        assert!(app.progress_messages[0].contains("msg-10"));
        assert!(app.progress_messages[99].contains("msg-109"));
    }

    // ── set_processing ─────────────────────────────────────────────────

    #[test]
    fn set_processing_true_clears_and_initializes() {
        let mut app = App::new();
        app.add_progress_message("old msg".to_string());
        assert!(!app.progress_messages.is_empty());

        app.set_processing(true);

        assert!(app.is_processing);
        assert!(app.progress_messages.is_empty());
        assert_eq!(app.processing_step, Some("Initializing...".to_string()));
    }

    #[test]
    fn set_processing_false_clears_step() {
        let mut app = App::new();
        app.set_processing(true);
        assert!(app.processing_step.is_some());

        app.set_processing(false);

        assert!(!app.is_processing);
        assert!(app.processing_step.is_none());
    }

    // ── set_error ──────────────────────────────────────────────────────

    #[test]
    fn set_error_stores_message_and_stops_processing() {
        let mut app = App::new();
        app.set_processing(true);

        app.set_error("boom".to_string());

        assert_eq!(app.error_message, Some("boom".to_string()));
        assert!(!app.is_processing);
        assert!(app.processing_step.is_none());
    }

    // ── clear_results ──────────────────────────────────────────────────

    #[test]
    fn clear_results_resets_counters() {
        let mut app = App::new();
        app.total_processed = 5;
        app.total_uploaded = 3;
        app.total_failed = 1;
        app.billing_month = Some("March".to_string());
        app.drive_folder = Some("some/folder".to_string());

        app.clear_results();

        assert_eq!(app.total_processed, 0);
        assert_eq!(app.total_uploaded, 0);
        assert_eq!(app.total_failed, 0);
        assert!(app.billing_month.is_none());
        assert!(app.drive_folder.is_none());
    }

    // ── reset_manual_inputs ────────────────────────────────────────────

    #[test]
    fn reset_manual_inputs_clears_dates_and_results() {
        let mut app = App::new();
        app.start_date_input = "2025-01-01".to_string();
        app.end_date_input = "2025-01-31".to_string();
        app.total_processed = 10;

        app.reset_manual_inputs();

        assert!(app.start_date_input.is_empty());
        assert!(app.end_date_input.is_empty());
        assert_eq!(app.total_processed, 0);
    }

    // ── is_date_input_valid ────────────────────────────────────────────

    #[test]
    fn is_date_input_valid_with_complete_dates() {
        let mut app = App::new();
        app.start_date_input = "2025-01-01".to_string();
        app.end_date_input = "2025-01-31".to_string();
        assert!(app.is_date_input_valid());
    }

    #[test]
    fn is_date_input_valid_rejects_empty_start() {
        let mut app = App::new();
        app.end_date_input = "2025-01-31".to_string();
        assert!(!app.is_date_input_valid());
    }

    #[test]
    fn is_date_input_valid_rejects_empty_end() {
        let mut app = App::new();
        app.start_date_input = "2025-01-01".to_string();
        assert!(!app.is_date_input_valid());
    }

    #[test]
    fn is_date_input_valid_rejects_partial_dates() {
        let mut app = App::new();
        app.start_date_input = "2025-01".to_string();
        app.end_date_input = "2025-01-31".to_string();
        assert!(!app.is_date_input_valid());
    }

    #[test]
    fn is_date_input_valid_rejects_both_empty() {
        let app = App::new();
        assert!(!app.is_date_input_valid());
    }

    // ── popup management ───────────────────────────────────────────────

    #[test]
    fn open_popup_sets_state_and_clears_error() {
        let mut app = App::new();
        app.error_message = Some("old error".to_string());

        app.open_popup(PopupState::Help);

        assert_eq!(app.popup_state, PopupState::Help);
        assert!(app.error_message.is_none());
    }

    #[test]
    fn close_popup_resets_to_none() {
        let mut app = App::new();
        app.open_popup(PopupState::DateInput);

        app.close_popup();

        assert_eq!(app.popup_state, PopupState::None);
        assert!(app.error_message.is_none());
    }

    #[test]
    fn is_popup_open_true_when_not_none() {
        let mut app = App::new();
        assert!(!app.is_popup_open());

        app.open_popup(PopupState::ProcessingConfirm);
        assert!(app.is_popup_open());
    }

    #[test]
    fn is_popup_open_false_after_close() {
        let mut app = App::new();
        app.open_popup(PopupState::Help);
        app.close_popup();
        assert!(!app.is_popup_open());
    }

    // ── all popup states ───────────────────────────────────────────────

    #[test]
    fn all_popup_states_register_as_open() {
        let states = vec![
            PopupState::DateInput,
            PopupState::ScheduleConfig,
            PopupState::GmailAuthUrl,
            PopupState::DriveAuthUrl,
            PopupState::ProcessingConfirm,
            PopupState::CatchupConfirm,
            PopupState::Help,
            PopupState::SetupGuide,
            PopupState::DetailedLogs,
        ];

        for state in states {
            let mut app = App::new();
            app.open_popup(state.clone());
            assert!(app.is_popup_open(), "PopupState::{:?} should register as open", state);
        }
    }

    // ── FocusedPanel enum ──────────────────────────────────────────────

    #[test]
    fn focused_panel_equality() {
        assert_eq!(FocusedPanel::Manual, FocusedPanel::Manual);
        assert_ne!(FocusedPanel::Manual, FocusedPanel::Auth);
        assert_ne!(FocusedPanel::Auth, FocusedPanel::Scheduled);
        assert_ne!(FocusedPanel::Scheduled, FocusedPanel::Logs);
    }

    // ── AuthStatus enum ───────────────────────────────────────────────

    #[test]
    fn auth_status_equality() {
        assert_eq!(AuthStatus::NotAuthenticated, AuthStatus::NotAuthenticated);
        assert_eq!(AuthStatus::Authenticating, AuthStatus::Authenticating);
        assert_eq!(AuthStatus::Authenticated, AuthStatus::Authenticated);
        assert_eq!(
            AuthStatus::Error("test".to_string()),
            AuthStatus::Error("test".to_string())
        );
        assert_ne!(
            AuthStatus::Error("a".to_string()),
            AuthStatus::Error("b".to_string())
        );
        assert_ne!(AuthStatus::NotAuthenticated, AuthStatus::Authenticated);
    }

    #[test]
    fn auth_status_clone() {
        let status = AuthStatus::Error("err".to_string());
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    // ── catchup state ──────────────────────────────────────────────────

    #[test]
    fn catchup_state_defaults_to_zero() {
        let app = App::new();
        assert_eq!(app.catchup_total_missing, 0);
        assert_eq!(app.catchup_total_processed, 0);
    }

    #[test]
    fn catchup_state_can_be_set() {
        let mut app = App::new();
        app.catchup_total_missing = 3;
        app.catchup_total_processed = 2;
        assert_eq!(app.catchup_total_missing, 3);
        assert_eq!(app.catchup_total_processed, 2);
    }

    // ── auth popup success state ───────────────────────────────────────

    #[test]
    fn auth_popup_success_default_false() {
        let app = App::new();
        assert!(!app.auth_popup_success);
    }

    // ── scheduled_job_logged ───────────────────────────────────────────

    #[test]
    fn scheduled_job_logged_default_false() {
        let app = App::new();
        assert!(!app.scheduled_job_logged);
    }

    // ── animation counter ──────────────────────────────────────────────

    #[test]
    fn animation_counter_default_zero() {
        let app = App::new();
        assert_eq!(app.animation_counter, 0);
    }

    // ── logs scroll offset ─────────────────────────────────────────────

    #[test]
    fn logs_scroll_offset_default_zero() {
        let app = App::new();
        assert_eq!(app.logs_scroll_offset, 0);
    }

    // ── auth_url ───────────────────────────────────────────────────────

    #[test]
    fn auth_url_default_none() {
        let app = App::new();
        assert!(app.auth_url.is_none());
    }

    #[test]
    fn auth_url_can_be_set() {
        let mut app = App::new();
        app.auth_url = Some("https://example.com/auth".to_string());
        assert_eq!(app.auth_url, Some("https://example.com/auth".to_string()));
    }

    // ── schedule_input ─────────────────────────────────────────────────

    #[test]
    fn schedule_input_default_empty() {
        let app = App::new();
        assert!(app.schedule_input.is_empty());
    }

    // ── multiple operations in sequence ────────────────────────────────

    #[test]
    fn full_lifecycle_manual_processing() {
        let mut app = App::new();

        // Start with defaults
        assert!(!app.is_processing);
        assert!(app.progress_messages.is_empty());

        // Set dates
        app.start_date_input = "2025-01-01".to_string();
        app.end_date_input = "2025-01-31".to_string();
        assert!(app.is_date_input_valid());

        // Start processing
        app.set_processing(true);
        assert!(app.is_processing);
        assert!(app.progress_messages.is_empty()); // cleared on start

        // Add some progress messages
        app.add_progress_message("Step 1".to_string());
        app.add_progress_message("Step 2".to_string());
        assert_eq!(app.progress_messages.len(), 2);

        // Finish processing
        app.set_processing(false);
        assert!(!app.is_processing);

        // Set results
        app.total_processed = 10;
        app.total_uploaded = 8;
        app.total_failed = 2;
        app.billing_month = Some("January".to_string());
        app.drive_folder = Some("Billing/2025/January".to_string());

        // Reset
        app.reset_manual_inputs();
        assert!(app.start_date_input.is_empty());
        assert_eq!(app.total_processed, 0);
    }

    #[test]
    fn error_during_processing_stops_and_records() {
        let mut app = App::new();
        app.set_processing(true);
        assert!(app.is_processing);

        app.set_error("Connection failed".to_string());

        assert!(!app.is_processing);
        assert!(app.processing_step.is_none());
        assert_eq!(app.error_message, Some("Connection failed".to_string()));
    }

    #[test]
    fn popup_cycle_through_all_states() {
        let mut app = App::new();
        assert!(!app.is_popup_open());

        let states = vec![
            PopupState::DateInput,
            PopupState::ScheduleConfig,
            PopupState::GmailAuthUrl,
            PopupState::DriveAuthUrl,
            PopupState::ProcessingConfirm,
            PopupState::CatchupConfirm,
            PopupState::Help,
            PopupState::SetupGuide,
            PopupState::DetailedLogs,
        ];

        for state in states {
            app.open_popup(state.clone());
            assert!(app.is_popup_open());
            assert_eq!(app.popup_state, state);
            app.close_popup();
            assert!(!app.is_popup_open());
        }
    }

    // ── determine_billing_month ────────────────────────────────────────

    #[test]
    fn billing_month_same_month() {
        let start = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 3, 31).unwrap();
        assert_eq!(determine_billing_month(start, end), "March");
    }

    #[test]
    fn billing_month_same_month_partial() {
        let start = NaiveDate::from_ymd_opt(2025, 6, 10).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 6, 20).unwrap();
        assert_eq!(determine_billing_month(start, end), "June");
    }

    #[test]
    fn billing_month_cross_month_early_end() {
        // Feb 1 - Mar 5 (5 days in March, < 15, total > 20)
        let start = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 3, 5).unwrap();
        assert_eq!(determine_billing_month(start, end), "February");
    }

    #[test]
    fn billing_month_cross_month_late_end() {
        // Jan 20 - Feb 25 (25 days in Feb, >= 15)
        let start = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 2, 25).unwrap();
        assert_eq!(determine_billing_month(start, end), "February");
    }

    #[test]
    fn billing_month_january() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
        assert_eq!(determine_billing_month(start, end), "January");
    }

    #[test]
    fn billing_month_december() {
        let start = NaiveDate::from_ymd_opt(2025, 12, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        assert_eq!(determine_billing_month(start, end), "December");
    }

    // ── parse_folder_years ─────────────────────────────────────────────

    #[test]
    fn parse_folder_years_with_year_in_path() {
        let (base, years) = parse_folder_years("Billing/All-Expenses/2025", 2025, 6);
        assert_eq!(base, "Billing/All-Expenses");
        assert_eq!(years, vec![2025]);
    }

    #[test]
    fn parse_folder_years_january_includes_previous_year() {
        let (base, years) = parse_folder_years("Billing/2026", 2026, 1);
        assert_eq!(base, "Billing");
        assert_eq!(years, vec![2025, 2026]);
    }

    #[test]
    fn parse_folder_years_older_year_includes_range() {
        let (base, years) = parse_folder_years("Billing/2024", 2026, 6);
        assert_eq!(base, "Billing");
        assert_eq!(years, vec![2024, 2025, 2026]);
    }

    #[test]
    fn parse_folder_years_no_year_in_path() {
        let (base, years) = parse_folder_years("Billing/All-Expenses", 2026, 6);
        assert_eq!(base, "Billing/All-Expenses");
        assert_eq!(years, vec![2026]);
    }

    #[test]
    fn parse_folder_years_no_year_january() {
        let (base, years) = parse_folder_years("Billing", 2026, 1);
        assert_eq!(base, "Billing");
        assert_eq!(years, vec![2025, 2026]);
    }

    #[test]
    fn parse_folder_years_single_segment_with_year() {
        // Single path segment that happens to be a number but no slash
        let (base, years) = parse_folder_years("2025", 2025, 3);
        // No slash means rsplitn won't split; treated as base path
        assert_eq!(base, "2025");
        assert_eq!(years, vec![2025]);
    }

    #[test]
    fn parse_folder_years_deep_path() {
        let (base, years) = parse_folder_years("Company/Finance/Billing/2025", 2025, 10);
        assert_eq!(base, "Company/Finance/Billing");
        assert_eq!(years, vec![2025]);
    }

    #[test]
    fn parse_folder_years_same_year_not_january() {
        let (base, years) = parse_folder_years("Billing/2026", 2026, 3);
        assert_eq!(base, "Billing");
        assert_eq!(years, vec![2026]);
    }
}

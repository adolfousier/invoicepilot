use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc};

/// Check if today is the scheduled day to fetch invoices
pub fn should_run_today(scheduled_day: u8) -> bool {
    let today = Utc::now().day() as u8;
    today == scheduled_day
}

/// Calculate the date range for the previous month
pub fn get_previous_month_range() -> (NaiveDate, NaiveDate) {
    let now = Utc::now();
    let current_year = now.year();
    let current_month = now.month();

    let (prev_year, prev_month) = if current_month == 1 {
        (current_year - 1, 12)
    } else {
        (current_year, current_month - 1)
    };

    let start_date = NaiveDate::from_ymd_opt(prev_year, prev_month, 1)
        .expect("Invalid start date");

    // Get last day of previous month
    let end_date = if prev_month == 12 {
        NaiveDate::from_ymd_opt(prev_year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(prev_year, prev_month + 1, 1)
    }
    .expect("Invalid end date calculation")
    .pred_opt()
    .expect("Invalid end date");

    (start_date, end_date)
}

/// Parse custom date range from string (format: YYYY-MM-DD:YYYY-MM-DD)
pub fn parse_date_range(range_str: &str) -> Result<(NaiveDate, NaiveDate)> {
    let parts: Vec<&str> = range_str.split(':').collect();

    if parts.len() != 2 {
        anyhow::bail!("Date range must be in format YYYY-MM-DD:YYYY-MM-DD");
    }

    let start_date = NaiveDate::parse_from_str(parts[0], "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid start date: {}", e))?;

    let end_date = NaiveDate::parse_from_str(parts[1], "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid end date: {}", e))?;

    if end_date < start_date {
        anyhow::bail!("End date must be after start date");
    }

    Ok((start_date, end_date))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_run_today() {
        let today = Utc::now().day() as u8;
        assert!(should_run_today(today));
        assert!(!should_run_today((today % 28) + 1));
    }

    #[test]
    fn test_previous_month_range() {
        let (start, end) = get_previous_month_range();

        // Start should be first day of previous month
        assert_eq!(start.day(), 1);

        // End should be last day of previous month
        assert!(end.day() >= 28);
        assert!(end.day() <= 31);

        // Start should be before end
        assert!(start < end);
    }

    #[test]
    fn test_parse_date_range() {
        let result = parse_date_range("2024-09-01:2024-10-12").unwrap();
        assert_eq!(result.0, NaiveDate::from_ymd_opt(2024, 9, 1).unwrap());
        assert_eq!(result.1, NaiveDate::from_ymd_opt(2024, 10, 12).unwrap());

        // Test invalid format
        assert!(parse_date_range("2024-09-01").is_err());
        assert!(parse_date_range("invalid:date").is_err());

        // Test end before start
        assert!(parse_date_range("2024-10-12:2024-09-01").is_err());
    }
}

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate};
use super::client::{GmailClient, GMAIL_API_BASE, MessageListResponse};

/// Search Gmail for invoice emails within a date range
pub async fn search_invoices(
    client: &GmailClient,
    start_date: NaiveDate,
    end_date: NaiveDate,
    keywords: &[String],
) -> Result<Vec<String>> {
    // Silently search - detailed progress sent via UI
    let keywords_to_search = if keywords.is_empty() {
        vec!["invoice".to_string(), "invoices".to_string(), "fatura".to_string(), "faturas".to_string(), "statement".to_string(), "bank".to_string()]
    } else {
        keywords.to_vec()
    };

    let mut all_message_ids = std::collections::HashSet::new();

    // Search for each keyword separately to maximize results
    for keyword in &keywords_to_search {
        let query = build_search_query_single(start_date, end_date, keyword);

        match search_with_query(client, &query).await {
            Ok(message_ids) => {
                for id in message_ids {
                    all_message_ids.insert(id);
                }
            }
            Err(_e) => {
                // Silently skip failed searches
            }
        }
    }

    let final_results: Vec<String> = all_message_ids.into_iter().collect();
    Ok(final_results)
}

/// Perform a single search query
async fn search_with_query(client: &GmailClient, query: &str) -> Result<Vec<String>> {
    let url = format!("{}/users/me/messages", GMAIL_API_BASE);

    let response = client.client()
        .get(&url)
        .bearer_auth(client.access_token())
        .query(&[("q", query)])
        .send()
        .await
        .context("Failed to search Gmail")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Gmail API error ({}): {}", status, error_text);
    }

    let result: MessageListResponse = response.json().await
        .context("Failed to parse Gmail search response")?;

    let message_ids: Vec<String> = result.messages
        .unwrap_or_default()
        .into_iter()
        .map(|m| m.id)
        .collect();

    Ok(message_ids)
}

/// Build Gmail search query for a single keyword
fn build_search_query_single(start_date: NaiveDate, end_date: NaiveDate, keyword: &str) -> String {
    format!(
        "{} has:attachment after:{}/{}/{} before:{}/{}/{}",
        keyword,
        start_date.year(),
        start_date.month(),
        start_date.day(),
        end_date.year(),
        end_date.month(),
        end_date.day()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_build_search_query_single() {
        let start = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 10, 12).unwrap();

        let query = build_search_query_single(start, end, "invoice");

        assert!(query.contains("invoice"));
        assert!(query.contains("has:attachment"));
        assert!(query.contains("after:2024/9/1"));
        assert!(query.contains("before:2024/10/12"));
        assert!(!query.contains("OR")); // Should be single keyword only
    }
}

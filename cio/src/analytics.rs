#![allow(clippy::from_over_into)]
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use macros::db;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::airtable::{AIRTABLE_BASE_ID_CUSTOMER_LEADS, AIRTABLE_PAGE_VIEWS_TABLE};
use crate::auth_logins::AuthUsers;
use crate::core::UpdateAirtableRecord;
use crate::schema::page_views;

#[db {
    new_struct_name = "PageView",
    airtable_base_id = "AIRTABLE_BASE_ID_CUSTOMER_LEADS",
    airtable_table = "AIRTABLE_PAGE_VIEWS_TABLE",
    match_on = {
        "time" = "DateTime<Utc>",
        "user_email" = "String",
    },
}]
#[derive(Debug, Insertable, AsChangeset, PartialEq, Clone, JsonSchema, Deserialize, Serialize)]
#[table_name = "page_views"]
pub struct NewPageView {
    pub time: DateTime<Utc>,
    pub domain: String,
    pub path: String,
    pub user_email: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub page_link: String,
    /// link to another table in Airtable
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub link_to_auth_user: Vec<String>,
}

/// Implement updating the Airtable record for a PageView.
#[async_trait]
impl UpdateAirtableRecord<PageView> for PageView {
    async fn update_airtable_record(&mut self, _record: PageView) {
        // Get the current auth users in Airtable so we can link to it.
        // TODO: make this more dry so we do not call it every single damn time.
        let auth_users = AuthUsers::get_from_airtable().await;

        // Iterate over the auth_users and see if we find a match.
        for (_id, auth_user_record) in auth_users {
            if auth_user_record.fields.email == self.user_email {
                // Set the link_to_auth_user to the right user.
                self.link_to_auth_user = vec![auth_user_record.id];
                // Break the loop and return early.
                break;
            }
        }
    }
}

impl NewPageView {
    pub fn set_page_link(&mut self) {
        // Set the link.
        self.page_link = format!("https://{}/{}", self.domain, self.path.trim_start_matches('/'));
    }
}

#[cfg(test)]
mod tests {
    use crate::analytics::PageViews;
    use crate::db::Database;

    #[ignore]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_cron_page_views_airtable() {
        // Initialize our database.
        let db = Database::new();

        PageViews::get_from_db(&db).update_airtable().await;
    }
}

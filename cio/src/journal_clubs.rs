use std::str::from_utf8;

use chrono::naive::NaiveDate;
use csv::ReaderBuilder;
use hubcaps::Github;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::slack::{
    FormattedMessage, MessageBlock, MessageBlockText, MessageBlockType,
    MessageType,
};
use crate::utils::github_org;

/// The data type for a journal club meeting.
#[derive(Debug, Clone, JsonSchema, Deserialize, Serialize)]
pub struct Meeting {
    pub title: String,
    pub issue: String,
    pub papers: Vec<Paper>,
    pub date: NaiveDate,
    pub coordinator: String,
    pub state: String,
    pub recording: String,
}

impl Meeting {
    /// Convert the journal club meeting into JSON as Slack message.
    pub fn as_slack_msg(&self) -> Value {
        let mut objects: Vec<Value> = Default::default();

        if !self.recording.is_empty() {
            objects.push(json!(MessageBlock {
                block_type: MessageBlockType::Context,
                elements: Some(vec![MessageBlockText {
                    text_type: MessageType::Markdown,
                    text: format!("<{}|Meeting recording>", self.recording),
                }]),
                text: None,
                accessory: None,
                block_id: None,
                fields: None,
            }));
        }

        for p in self.papers.clone() {
            let mut title = p.title.to_string();
            if p.title == self.title {
                title = "Paper".to_string();
            }
            objects.push(json!(MessageBlock {
                block_type: MessageBlockType::Context,
                elements: Some(vec![MessageBlockText {
                    text_type: MessageType::Markdown,
                    text: format!("<{}|{}>", p.link, title),
                }]),
                text: None,
                accessory: None,
                block_id: None,
                fields: None,
            }));
        }

        json!(FormattedMessage {
            channel: None,
            attachments: None,
            blocks: Some(vec![
                MessageBlock {
                    block_type: MessageBlockType::Section,
                    text: Some(MessageBlockText {
                        text_type: MessageType::Markdown,
                        text: format!("<{}|*{}*>", self.issue, self.title),
                    }),
                    elements: None,
                    accessory: None,
                    block_id: None,
                    fields: None,
                },
                MessageBlock {
                    block_type: MessageBlockType::Context,
                    elements: Some(vec![MessageBlockText {
                        text_type: MessageType::Markdown,
                        text: format!(
                            "<https://github.com/{}|@{}> | {} | status: *{}*",
                            self.coordinator,
                            self.coordinator,
                            self.date.format("%m/%d/%Y"),
                            self.state
                        ),
                    }]),
                    text: None,
                    accessory: None,
                    block_id: None,
                    fields: None,
                }
            ]),
        })
    }
}

/// The data type for a paper.
#[derive(Debug, Default, Clone, JsonSchema, Deserialize, Serialize)]
pub struct Paper {
    pub title: String,
    pub link: String,
}

/// Get the journal club meetings from the papers GitHub repo.
pub async fn get_meetings_from_repo(github: &Github) -> Vec<Meeting> {
    // Get the contents of the .helpers/meetings.csv file.
    let meetings_csv_content = github
        .repo(github_org(), "papers")
        .content()
        .file("/.helpers/meetings.csv")
        .await
        .expect("failed to get meetings csv content")
        .content;
    let meetings_csv_string = from_utf8(&meetings_csv_content).unwrap();

    // Create the csv reader.
    let mut csv_reader = ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(true)
        .from_reader(meetings_csv_string.as_bytes());

    // Create the BTreeMap of Meetings.
    let mut meetings: Vec<Meeting> = Default::default();
    for r in csv_reader.records() {
        let record = r.unwrap();

        // Parse the date.
        let date = NaiveDate::parse_from_str(&record[5], "%m/%d/%Y").unwrap();

        // Parse the papers.
        let mut papers: Vec<Paper> = Default::default();
        let papers_parts = record[2].trim().split(") [");
        for p in papers_parts {
            // Parse the markdown for the papers.
            let start_title = p.find('[').unwrap_or(0);
            let end_title = p.find(']').unwrap_or_else(|| p.len());
            let title = p[start_title..end_title]
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_string();

            let start_link = p.find('(').unwrap_or(0);
            let end_link = p.find(')').unwrap_or_else(|| p.len());
            let link = p[start_link..end_link]
                .trim_start_matches('(')
                .trim_end_matches(')')
                .to_string();

            papers.push(Paper { title, link });
        }

        let meeting = Meeting {
            title: record[0].to_string(),
            issue: record[1].to_string(),
            papers,
            coordinator: record[3].to_string(),
            state: record[4].to_string(),
            date,
            recording: record[6].to_string(),
        };

        // Add this to our Vec.
        meetings.push(meeting);
    }

    meetings
}
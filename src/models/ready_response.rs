use nanoserde::{DeJson, SerJson};

use super::{guild::Guild, user::User};

#[derive(DeJson, SerJson)]
pub struct ReadyResponse {
    #[nserde(rename = "d")]
    pub data: ReadyData,
}

#[derive(DeJson, SerJson)]
pub struct ReadyData {
    pub user: User,
    pub session_type: String,
    pub session_id: String,
    pub resume_gateway_url: String,
    pub guilds: Vec<Guild>,
    pub geo_ordered_rtc_regions: Vec<String>,
    pub application: ApplicationData,
}

#[derive(DeJson, SerJson)]
pub struct ApplicationData {
    pub id: String,
    pub flags: usize,
}

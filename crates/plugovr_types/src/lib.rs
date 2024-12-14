use egui::Pos2;
use image::{ImageBuffer, Rgba};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct UserInfo {
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub name: Option<String>,
    pub email: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub subscription_status: Option<String>,
    pub subscription_name: Option<String>,
    pub subscription_end_date: Option<String>,
}

pub type Screenshots = Vec<(ImageBuffer<Rgba<u8>, Vec<u8>>, Pos2)>;

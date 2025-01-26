#[derive(serde::Deserialize)]
#[allow(dead_code)]
pub struct Config {
    club_id: u32,
    app_key: String,
    username: String,
    password: String,
}

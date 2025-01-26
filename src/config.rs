#[derive(serde::Deserialize)]
#[allow(dead_code)]
pub struct Config {
    pub club_id: u32,
    pub app_key: String,
    pub username: String,
    pub password: String,
}

impl Config {
    #[cfg(test)]
    pub fn dummy() -> Self {
        Self {
            club_id: 1,
            app_key: "app_key".to_string(),
            username: "username".to_string(),
            password: "password".to_string(),
        }
    }
}

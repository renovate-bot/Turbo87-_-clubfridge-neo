use crate::database;
use crate::state::Message;
use iced::futures::FutureExt;
use iced::keyboard::key::Named;
use iced::keyboard::Key;
use iced::{Subscription, Task};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sqlx::SqlitePool;
use std::mem;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use ulid::Ulid;

pub struct RunningClubFridge {
    pub pool: SqlitePool,

    pub user: Option<database::Member>,
    pub input: String,
    pub items: Vec<Item>,
    pub show_sale_confirmation: bool,
}

impl RunningClubFridge {
    pub fn subscription(&self) -> Subscription<Message> {
        iced::keyboard::on_key_press(|key, _modifiers| Some(Message::KeyPress(key)))
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub amount: u16,
    pub article: database::Article,
}

impl Item {
    pub fn total(&self) -> Decimal {
        Decimal::from(self.amount) * self.article.current_price().unwrap_or_default()
    }
}

impl RunningClubFridge {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::KeyPress(Key::Character(c)) => {
                debug!("Key pressed: {c:?}");
                self.input.push_str(c.as_str());
                self.show_sale_confirmation = false;
            }
            Message::KeyPress(Key::Named(Named::Enter)) => {
                debug!("Key pressed: Enter");
                let task = if self.user.is_some() {
                    let barcode = self.input.clone();
                    Task::future(database::Article::find_by_barcode(
                        self.pool.clone(),
                        barcode.clone(),
                    ))
                    .then(move |result| match result {
                        Ok(Some(article)) => Task::done(Message::AddToSale(article)),
                        Ok(None) => {
                            warn!("No article found for barcode: {barcode}");
                            Task::none()
                        }
                        Err(err) => {
                            error!("Failed to find article: {err}");
                            Task::none()
                        }
                    })
                } else {
                    let keycode = self.input.clone();
                    Task::future(database::Member::find_by_keycode(
                        self.pool.clone(),
                        keycode.clone(),
                    ))
                    .then(move |result| match result {
                        Ok(Some(member)) => Task::done(Message::SetUser(member)),
                        Ok(None) => {
                            warn!("No user found for keycode: {keycode}");
                            Task::none()
                        }
                        Err(err) => {
                            error!("Failed to find user: {err}");
                            Task::none()
                        }
                    })
                };

                self.input.clear();
                self.show_sale_confirmation = false;

                return task;
            }
            #[cfg(debug_assertions)]
            Message::KeyPress(Key::Named(Named::Control)) => {
                let task = if self.user.is_some() {
                    let ulid = Ulid::new().to_string();
                    Task::done(Message::AddToSale(database::Article {
                        id: ulid.clone(),
                        designation: ulid,
                        prices: vec![{
                            database::Price {
                                valid_from: jiff::civil::Date::constant(2000, 1, 1),
                                valid_to: jiff::civil::Date::constant(2999, 12, 31),
                                unit_price: dec!(0.9),
                            }
                        }],
                    }))
                } else {
                    Task::done(Message::SetUser(database::Member {
                        id: "11011".to_string(),
                        firstname: "Tobias".to_string(),
                        lastname: "Bieniek".to_string(),
                        nickname: "Turbo".to_string(),
                    }))
                };

                self.show_sale_confirmation = false;

                return task;
            }
            Message::AddToSale(article) => {
                info!("Adding article to sale: {article:?}");
                if self.user.is_some() && article.current_price().is_some() {
                    self.items
                        .iter_mut()
                        .find(|item| item.article.id == article.id)
                        .map(|item| {
                            item.amount += 1;
                        })
                        .unwrap_or_else(|| {
                            self.items.push(Item { amount: 1, article });
                        });
                }
            }
            Message::SetUser(member) => {
                info!("Setting user: {member:?}");
                self.user = Some(member);
            }
            Message::Pay => {
                info!("Processing sale");
                let pool = self.pool.clone();
                let date = jiff::Zoned::now().date();

                let sales = mem::take(&mut self.items)
                    .into_iter()
                    .map(|item| database::NewSale {
                        id: Ulid::new(),
                        date,
                        member_id: self
                            .user
                            .as_ref()
                            .map(|user| &user.id)
                            .cloned()
                            .unwrap_or_default(),
                        article_id: item.article.id,
                        amount: item.amount as u32,
                    })
                    .collect();

                return Task::future(database::add_sales(pool, sales).map(|result| match result {
                    Ok(()) => Message::SalesSaved,
                    Err(err) => {
                        error!("Failed to save sales: {err}");
                        Message::SavingSalesFailed
                    }
                }));
            }
            Message::SalesSaved => {
                info!("Sales saved");
                self.user = None;
                self.items.clear();
                self.show_sale_confirmation = true;
                return Task::perform(tokio::time::sleep(Duration::from_secs(3)), |_| {
                    Message::HideSaleConfirmation
                });
            }
            Message::SavingSalesFailed => {
                error!("Failed to save sales");
            }
            Message::Cancel => {
                info!("Cancelling sale");
                self.user = None;
                self.items.clear();
            }
            Message::HideSaleConfirmation => {
                debug!("Hiding sale confirmation popup");
                self.show_sale_confirmation = false;
            }
            _ => {}
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ClubFridge;

    fn input(cf: &mut RunningClubFridge, input: &str) {
        for c in input.chars() {
            let char = c.to_string().into();
            let _ = cf.update(Message::KeyPress(Key::Character(char)));
        }

        let _ = cf.update(Message::KeyPress(Key::Named(Named::Enter)));
    }

    #[tokio::test]
    async fn test_happy_path() {
        let (mut cf, _) = ClubFridge::new(Default::default());
        let _ = cf.update(Message::DatabaseConnected(
            database::connect().await.unwrap(),
        ));
        let _ = cf.update(Message::DatabaseMigrated);

        let ClubFridge::Running(mut cf) = cf else {
            panic!("Expected ClubFridge::Running");
        };

        input(&mut cf, "0005635570");
        assert_eq!(
            cf.user.as_ref().map(|u| &u.id).cloned().unwrap_or_default(),
            "0005635570"
        );
        assert_eq!(cf.items.len(), 0);
        assert!(!cf.show_sale_confirmation);

        input(&mut cf, "3800235265659");
        assert_eq!(
            cf.user.as_ref().map(|u| &u.id).cloned().unwrap_or_default(),
            "0005635570"
        );
        assert_eq!(cf.items.len(), 1);
        assert_eq!(cf.items[0].article.designation, "Gloriette Cola Mix");
        assert_eq!(cf.items[0].amount, 1);
        assert!(!cf.show_sale_confirmation);

        input(&mut cf, "3800235266700");
        assert_eq!(
            cf.user.as_ref().map(|u| &u.id).cloned().unwrap_or_default(),
            "0005635570"
        );
        assert_eq!(cf.items.len(), 2);
        assert_eq!(cf.items[0].article.designation, "Gloriette Cola Mix");
        assert_eq!(cf.items[0].amount, 1);
        assert_eq!(cf.items[1].article.designation, "Erdinger Weissbier 0.5L");
        assert_eq!(cf.items[1].amount, 1);
        assert!(!cf.show_sale_confirmation);

        input(&mut cf, "3800235265659");
        assert_eq!(
            cf.user.as_ref().map(|u| &u.id).cloned().unwrap_or_default(),
            "0005635570"
        );
        assert_eq!(cf.items.len(), 2);
        assert_eq!(cf.items[0].article.designation, "Gloriette Cola Mix");
        assert_eq!(cf.items[0].amount, 2);
        assert_eq!(cf.items[1].article.designation, "Erdinger Weissbier 0.5L");
        assert_eq!(cf.items[1].amount, 1);
        assert!(!cf.show_sale_confirmation);

        let _ = cf.update(Message::Pay);
        assert_eq!(cf.user, None);
        assert_eq!(cf.items.len(), 0);
        assert!(cf.show_sale_confirmation);
    }
}

use iced::keyboard::key::Named;
use iced::keyboard::Key;
use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{application, color, Center, Element, Fill, Right, Subscription, Theme};
use std::sync::Arc;

pub fn main() -> iced::Result {
    application("ClubFridge neo", update, view)
        .theme(theme)
        .subscription(subscription)
        .resizable(true)
        .window_size((800., 480.))
        .run()
}

fn theme(_state: &State) -> Theme {
    Theme::Custom(Arc::new(iced::theme::Custom::new(
        "clubfridge".to_string(),
        iced::theme::Palette {
            background: color!(0x000000),
            text: color!(0xffffff),
            primary: color!(0xffffff),
            success: color!(0x4BD130),
            danger: color!(0xD5A30F),
        },
    )))
}

#[derive(Default)]
struct State {
    user: Option<String>,
    input: String,
    items: Vec<Item>,
}

#[derive(Debug, Clone)]
struct Item {
    amount: u16,
    description: String,
    price: f32,
}

impl Item {
    fn total(&self) -> f32 {
        self.amount as f32 * self.price
    }
}

#[derive(Debug, Clone)]
enum Message {
    KeyPress(Key),
    AddItem(Item),
    Cancel,
}

fn subscription(_state: &State) -> Subscription<Message> {
    iced::keyboard::on_key_release(|key, _modifiers| Some(Message::KeyPress(key)))
}

fn update(state: &mut State, message: Message) {
    match message {
        Message::KeyPress(Key::Character(c)) => state.input.push_str(c.as_str()),
        Message::KeyPress(Key::Named(Named::Enter)) => {
            if state.user.is_some() {
                state
                    .items
                    .iter_mut()
                    .find(|item| item.description == state.input)
                    .map(|item| {
                        item.amount += 1;
                    })
                    .unwrap_or_else(|| {
                        state.items.push(Item {
                            amount: 1,
                            description: state.input.clone(),
                            price: 0.5,
                        });
                    });
            } else {
                state.user = Some(state.input.clone());
            }

            state.input.clear();
        }
        Message::AddItem(item) => state.items.push(item),
        Message::Cancel => {
            state.user = None;
            state.items.clear()
        }
        _ => {}
    }
}

fn view(state: &State) -> Element<Message> {
    let items = Column::with_children(state.items.iter().map(|item| {
        text(format!(
            "{}x {}    {:.2}€   Gesamt {:.2}",
            item.amount,
            item.description,
            item.price,
            item.total()
        ))
        .size(24)
        .into()
    }));

    let sum = state.items.iter().map(|item| item.total()).sum::<f32>();

    container(
        column![
            text(state.user.as_deref().unwrap_or("Bitte RFID Chip")).size(36),
            scrollable(items).height(Fill).width(Fill).anchor_bottom(),
            text(format!("Summe: € {sum:.2}"))
                .size(24)
                .align_x(Right)
                .width(Fill),
            row![
                button(
                    text("Abbruch")
                        .color(color!(0xffffff))
                        .size(36)
                        .align_x(Center)
                )
                .width(Fill)
                .style(button::danger)
                .padding([10, 20])
                .on_press_maybe(state.user.as_ref().map(|_| Message::Cancel)),
                button(
                    text("Bezahlen")
                        .color(color!(0xffffff))
                        .size(36)
                        .align_x(Center)
                )
                .width(Fill)
                .style(button::success)
                .padding([10, 20])
                .on_press_maybe(state.user.as_ref().map(|_| Message::AddItem(Item {
                    amount: 1,
                    description: "Kaffee Pott/Tasse/Es".to_string(),
                    price: 0.5,
                }))),
            ]
            .spacing(10),
        ]
        .spacing(10),
    )
    .style(|_theme: &Theme| container::background(color!(0x000000)))
    .padding([20, 30])
    .into()
}

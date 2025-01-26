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
    items: Vec<String>,
}

#[derive(Debug, Clone)]
enum Message {
    KeyPress(Key),
    AddItem,
    ClearItems,
}

fn subscription(_state: &State) -> Subscription<Message> {
    iced::keyboard::on_key_release(|key, _modifiers| Some(Message::KeyPress(key)))
}

fn update(state: &mut State, message: Message) {
    match message {
        Message::KeyPress(Key::Character(c)) => state.input.push_str(c.as_str()),
        Message::KeyPress(Key::Named(Named::Enter)) => {
            state.user = Some(state.input.clone());
            state.input.clear();
        }
        Message::AddItem => state
            .items
            .push("1x  Kaffee Pott/Tasse/Es    €0.50   Gesamt: €0.50".to_string()),
        Message::ClearItems => state.items.clear(),
        _ => {}
    }
}

fn view(state: &State) -> Element<Message> {
    let items = Column::with_children(state.items.iter().map(|item| text(item).size(24).into()));

    container(
        column![
            text(state.user.as_deref().unwrap_or("Bitte RFID Chip")).size(36),
            scrollable(items).height(Fill).width(Fill).anchor_bottom(),
            text("Summe: € 0.00").size(24).align_x(Right).width(Fill),
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
                .on_press_maybe(state.user.as_ref().map(|_| Message::ClearItems)),
                button(
                    text("Bezahlen")
                        .color(color!(0xffffff))
                        .size(36)
                        .align_x(Center)
                )
                .width(Fill)
                .style(button::success)
                .padding([10, 20])
                .on_press_maybe(state.user.as_ref().map(|_| Message::AddItem)),
            ]
            .spacing(10),
        ]
        .spacing(10),
    )
    .style(|_theme: &Theme| container::background(color!(0x000000)))
    .padding([20, 30])
    .into()
}

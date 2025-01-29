use crate::state::{Item, Message, State};
use iced::widget::{button, column, container, row, scrollable, stack, text};
use iced::{color, Center, Element, Fill, Right, Theme};
use rust_decimal::Decimal;
use std::sync::Arc;

impl State {
    pub fn theme(&self) -> Theme {
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

    pub fn view(&self) -> Element<Message> {
        let title = self
            .user
            .as_ref()
            .and_then(|user_id| {
                self.users
                    .get(user_id)
                    .map(|user_name| text(format!("{user_name} ({user_id})")))
            })
            .unwrap_or(text("Bitte RFID Chip"));

        let sum = self.items.iter().map(|item| item.total()).sum::<Decimal>();

        let content = column![
            title.size(36),
            scrollable(items(&self.items))
                .height(Fill)
                .width(Fill)
                .anchor_bottom(),
            text(format!("Summe: {sum:.2}€"))
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
                .on_press_maybe(self.user.as_ref().map(|_| Message::Cancel)),
                button(
                    text("Bezahlen")
                        .color(color!(0xffffff))
                        .size(36)
                        .align_x(Center)
                )
                .width(Fill)
                .style(button::success)
                .padding([10, 20])
                .on_press_maybe(self.user.as_ref().map(|_| Message::Pay)),
            ]
            .spacing(10),
        ]
        .spacing(10);

        let mut stack = stack![content];

        if self.show_sale_confirmation {
            stack = stack.push(
                container(
                    container(
                        text("Danke für deinen Kauf")
                            .size(36)
                            .color(color!(0x000000)),
                    )
                    .style(|_theme: &Theme| container::background(color!(0xffffff)))
                    .padding([15, 30]),
                )
                .width(Fill)
                .height(Fill)
                .align_x(Center)
                .align_y(Center),
            );
        }

        container(stack)
            .style(|_theme: &Theme| container::background(color!(0x000000)))
            .padding([20, 30])
            .into()
    }
}

fn items(items: &[Item]) -> Element<Message> {
    row![
        column(
            items
                .iter()
                .map(|item| { text(format!("{}x", item.amount)).size(24).into() })
        )
        .align_x(Right)
        .spacing(10),
        column(
            items
                .iter()
                .map(|item| { text(&item.description).size(24).into() })
        )
        .width(Fill)
        .spacing(10),
        column(items.iter().map(|item| {
            text(format!("{:.2}€", item.price))
                .size(24)
                .color(color!(0x888888))
                .into()
        }))
        .align_x(Right)
        .spacing(10),
        column(
            items
                .iter()
                .map(|item| { text(format!("{:.2}€", item.total())).size(24).into() })
        )
        .align_x(Right)
        .spacing(10),
    ]
    .spacing(20)
    .into()
}

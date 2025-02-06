use crate::state::Message;
use iced::border::rounded;
use iced::futures::FutureExt;
use iced::widget::{container, text};
use iced::{color, Element, Task, Theme};
use std::time::Duration;

/// The time after which the popup is automatically hidden.
const POPUP_TIMEOUT: Duration = Duration::from_secs(3);

pub struct Popup {
    pub message: String,
    _timeout_handle: iced::task::Handle,
}

impl Popup {
    pub fn new(message: String) -> (Self, Task<Message>) {
        let timeout_future = tokio::time::sleep(POPUP_TIMEOUT);
        let timeout_task = Task::future(timeout_future.map(|_| Message::PopupTimeoutReached));
        let (task, handle) = timeout_task.abortable();

        let popup = Self {
            message,
            _timeout_handle: handle.abort_on_drop(),
        };

        (popup, task)
    }

    pub fn view(&self) -> Element<Message> {
        container(text(&self.message).size(36).color(color!(0x000000)))
            .style(|_theme: &Theme| container::background(color!(0xffffff)).border(rounded(10.)))
            .padding([15, 30])
            .into()
    }
}

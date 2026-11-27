use iced::{
    Element, Length,
    widget::{column, text},
};

use crate::{
    consts::{FADER_MAX_DB, FADER_MIN_DB},
    ticks,
};

pub fn vertical_ticks<Message: 'static>() -> Element<'static, Message> {
    column![
        ticks::ticks(FADER_MIN_DB..=FADER_MAX_DB, 1.0),
        text("").size(10)
    ]
    .spacing(4)
    .height(Length::Fill)
    .into()
}

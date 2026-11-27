use iced::{
    Alignment, Element, Length,
    widget::{column, container, text},
};

use crate::{
    consts::{FADER_MAX_DB, FADER_MIN_DB},
    meters,
};

const SIDE_FADER_WIDTH: f32 = 50.0;
const SIDE_TICKS_WIDTH: f32 = 25.0;
const SIDE_COLUMN_WIDTH: f32 = SIDE_FADER_WIDTH + SIDE_TICKS_WIDTH;

pub fn vu_meter<Message: 'static>(
    channels: usize,
    levels_db: [f32; 2],
) -> Element<'static, Message> {
    let channels = channels.clamp(1, 2);
    let peak_db = levels_db
        .iter()
        .take(channels)
        .copied()
        .fold(FADER_MIN_DB, f32::max)
        .clamp(FADER_MIN_DB, FADER_MAX_DB);
    let readout = if peak_db <= FADER_MIN_DB {
        "-inf dB".to_string()
    } else {
        format!("{peak_db:.1} dB")
    };

    container(
        column![
            container(meters::meters(channels, &levels_db, 1.0))
                .width(Length::Fixed(meters::total_width(channels)))
                .height(Length::Fill),
            text(readout).size(10),
        ]
        .spacing(4)
        .height(Length::Fill)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(SIDE_COLUMN_WIDTH))
    .height(Length::Fill)
    .into()
}

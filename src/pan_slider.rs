use iced::{
    Alignment, Element, Length,
    alignment::Horizontal,
    widget::{column, container, row, text},
};

use crate::horizontal_slider::horizontal_slider;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReadoutPosition {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
}

pub fn pan_slider<Message: Clone + 'static>(
    value: f32,
    on_change: impl Fn(f32) -> Message + 'static,
    on_release: Message,
    readout_position: Option<ReadoutPosition>,
) -> Element<'static, Message> {
    let value_text = if value.abs() < 0.005 {
        "C".to_string()
    } else if value < 0.0 {
        format!("L{:.0}", -value * 100.0)
    } else {
        format!("R{:.0}", value * 100.0)
    };
    let readout: Element<'_, Message> = container(text(value_text).size(9))
        .width(Length::Fixed(24.0))
        .align_x(Horizontal::Center)
        .into();
    let slider: Element<'_, Message> = horizontal_slider(-1.0..=1.0, value, on_change)
        .width(Length::Fixed(52.0))
        .height(Length::Fixed(12.0))
        .step(0.01)
        .double_click_reset(0.0)
        .on_release(on_release)
        .into();

    match readout_position {
        Some(ReadoutPosition::Left) => row![readout, slider]
            .spacing(4)
            .align_y(Alignment::Center)
            .into(),
        Some(ReadoutPosition::Right) => row![slider, readout]
            .spacing(4)
            .align_y(Alignment::Center)
            .into(),
        Some(ReadoutPosition::Top) => column![readout, slider]
            .spacing(4)
            .align_x(Alignment::Center)
            .into(),
        Some(ReadoutPosition::Bottom) => column![slider, readout]
            .spacing(4)
            .align_x(Alignment::Center)
            .into(),
        None => slider,
    }
}

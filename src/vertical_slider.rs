use iced::{
    Alignment, Element, Length,
    widget::{column, container, text},
};

use crate::slider::Slider;

pub struct VerticalSlider {
    pub value: f32,
    pub range: std::ops::RangeInclusive<f32>,
    pub default: f32,
    pub step: f32,
    pub value_text: String,
}

pub fn vertical_slider<Message: Clone + 'static>(
    fader: VerticalSlider,
    on_change: impl Fn(f32) -> Message + 'static,
    on_release: Message,
) -> Element<'static, Message> {
    let min = *fader.range.start();
    let max = *fader.range.end();
    let slider = Slider::new(fader.range, fader.value.clamp(min, max), on_change)
        .step(fader.step)
        .double_click_reset(fader.default)
        .on_release(on_release)
        .width(Length::Fixed(14.0))
        .height(Length::Fill);

    container(
        column![slider, text(fader.value_text).size(10)]
            .spacing(4)
            .height(Length::Fill)
            .align_x(Alignment::Center),
    )
    .width(Length::Shrink)
    .height(Length::Fill)
    .into()
}

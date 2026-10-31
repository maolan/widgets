use iced::{
    Alignment, Background, Border, Color, Element, Length, Theme,
    widget::{column, container, text},
};

use crate::arch_range_slider::arch_range_slider;
use crate::arch_slider::arch_slider;

/// Data used to render a labeled knob and its value readout.
pub struct Knob {
    pub label: String,
    pub value: f32,
    pub range: std::ops::RangeInclusive<f32>,
    pub default: f32,
    pub step: f32,
    pub value_text: String,
    pub assigned: bool,
}

/// A compact labeled knob with an optional amber modulation-assignment state.
pub fn knob<Message: Clone + 'static>(
    knob: Knob,
    on_change: impl Fn(f32) -> Message + 'static,
    on_release: Message,
) -> Element<'static, Message> {
    let min = *knob.range.start();
    let max = *knob.range.end();
    let mut slider = arch_slider(knob.range, knob.value.clamp(min, max), on_change)
        .step(knob.step)
        .double_click_reset(knob.default)
        .on_release(on_release)
        .fill_from_start()
        .width(Length::Fixed(41.0))
        .height(Length::Fixed(41.0));

    if knob.assigned {
        slider = slider
            .filled_color(Color::from_rgb(0.82, 0.58, 0.08))
            .handle_color(Color::from_rgb(1.0, 0.83, 0.10));
    }

    let assigned = knob.assigned;
    container(
        column![
            text(knob.label)
                .size(11)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
            slider,
            text(knob.value_text)
                .size(10)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
        ]
        .spacing(2)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(50.0))
    .padding(2)
    .style(move |_theme: &Theme| {
        let border_color = if assigned {
            Color::from_rgb(1.0, 0.83, 0.10)
        } else {
            Color::TRANSPARENT
        };
        container::Style {
            background: assigned
                .then(|| Background::Color(Color::from_rgba(1.0, 0.83, 0.10, 0.10))),
            border: Border {
                color: border_color,
                width: if assigned { 1.0 } else { 0.0 },
                radius: 3.0.into(),
            },
            ..container::Style::default()
        }
    })
    .into()
}

/// Data used to render a knob with two independently adjustable range handles.
pub struct RangeKnob {
    pub label: String,
    pub low: f32,
    pub high: f32,
    pub range: std::ops::RangeInclusive<f32>,
    pub step: f32,
    pub value_text: String,
    pub assigned: bool,
}

/// A compact labeled range knob, styled like [`knob`].
pub fn range_knob<Message: Clone + 'static>(
    knob: RangeKnob,
    on_change: impl Fn(f32, f32) -> Message + 'static,
    on_release: Message,
) -> Element<'static, Message> {
    let min = *knob.range.start();
    let max = *knob.range.end();
    let low = knob.low.clamp(min, max);
    let high = knob.high.clamp(min, max);
    let mut slider = arch_range_slider(knob.range, low.min(high), low.max(high), on_change)
        .step(knob.step)
        .on_release(on_release)
        .width(Length::Fixed(41.0))
        .height(Length::Fixed(41.0));

    if knob.assigned {
        slider = slider
            .filled_color(Color::from_rgb(0.82, 0.58, 0.08))
            .handle_color(Color::from_rgb(1.0, 0.83, 0.10));
    }

    let assigned = knob.assigned;
    container(
        column![
            text(knob.label)
                .size(11)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
            slider,
            text(knob.value_text)
                .size(10)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
        ]
        .spacing(2)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(50.0))
    .padding(2)
    .style(move |_theme: &Theme| {
        let border_color = if assigned {
            Color::from_rgb(1.0, 0.83, 0.10)
        } else {
            Color::TRANSPARENT
        };
        container::Style {
            background: assigned
                .then(|| Background::Color(Color::from_rgba(1.0, 0.83, 0.10, 0.10))),
            border: Border {
                color: border_color,
                width: if assigned { 1.0 } else { 0.0 },
                radius: 3.0.into(),
            },
            ..container::Style::default()
        }
    })
    .into()
}

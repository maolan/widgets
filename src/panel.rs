use iced::{
    Alignment, Background, Border, Color, Element, Theme,
    widget::{column, container},
};

use crate::section_title::section_title;

pub fn panel<'a, Message: Clone + 'static>(
    title: &'static str,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    container(
        column![section_title(title), content]
            .spacing(6)
            .align_x(Alignment::Start),
    )
    .padding(8)
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(Color::from_rgb(0.08, 0.08, 0.10))),
        border: Border {
            color: Color::from_rgb(0.20, 0.20, 0.24),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    })
    .into()
}

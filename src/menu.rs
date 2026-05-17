use iced::{
    Border, Color, Element, Length, alignment,
    widget::{button, row, text},
};
use iced_fonts::lucide::{chevron_right, square, square_check_big};

pub fn base_button<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    msg: Message,
) -> button::Button<'a, Message>
where
    Message: Clone + 'a,
{
    button(content)
        .padding([4, 8])
        .style(|theme, status| {
            use button::{Status, Style};

            let palette = theme.extended_palette();
            let base = Style {
                text_color: palette.background.base.text,
                border: Border::default().rounded(6.0),
                ..Style::default()
            };
            match status {
                Status::Active => base.with_background(Color::TRANSPARENT),
                Status::Hovered => base.with_background(Color::from_rgb(
                    palette.primary.weak.color.r * 1.2,
                    palette.primary.weak.color.g * 1.2,
                    palette.primary.weak.color.b * 1.2,
                )),
                Status::Disabled => base.with_background(Color::from_rgb(0.5, 0.5, 0.5)),
                Status::Pressed => base.with_background(palette.primary.weak.color),
            }
        })
        .on_press(msg)
}

pub fn menu_button<Message>(
    label: impl Into<String>,
    width: Option<Length>,
    height: Option<Length>,
    msg: Message,
) -> Element<'static, Message, iced::Theme, iced::Renderer>
where
    Message: Clone + 'static,
{
    let label = label.into();
    base_button(
        text(label)
            .height(height.unwrap_or(Length::Shrink))
            .align_y(alignment::Vertical::Center),
        msg,
    )
    .width(width.unwrap_or(Length::Shrink))
    .height(height.unwrap_or(Length::Shrink))
    .into()
}

pub fn menu_dropdown<Message>(
    label: impl Into<String>,
    message: Message,
) -> Element<'static, Message, iced::Theme, iced::Renderer>
where
    Message: Clone + 'static,
{
    menu_button(label, Some(Length::Shrink), Some(Length::Shrink), message)
}

pub fn menu_item<Message>(
    label: impl Into<String>,
    message: Message,
) -> Element<'static, Message, iced::Theme, iced::Renderer>
where
    Message: Clone + 'static,
{
    menu_button(label, Some(Length::Fill), Some(Length::Shrink), message)
}

pub fn menu_checkbox_item<Message>(
    label: impl Into<String>,
    checked: bool,
    message: Message,
) -> Element<'static, Message, iced::Theme, iced::Renderer>
where
    Message: Clone + 'static,
{
    let label = label.into();
    let icon = if checked {
        square_check_big()
    } else {
        square()
    };
    base_button(
        row![
            icon.width(20).align_y(alignment::Vertical::Center),
            text(label)
                .height(Length::Shrink)
                .align_y(alignment::Vertical::Center),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
        message,
    )
    .width(Length::Fill)
    .height(Length::Shrink)
    .into()
}

pub fn menu_item_maybe<Message>(
    label: impl Into<String>,
    message: Option<Message>,
) -> Element<'static, Message, iced::Theme, iced::Renderer>
where
    Message: Clone + 'static,
{
    let label = label.into();
    let btn = button(
        text(label)
            .height(Length::Shrink)
            .align_y(alignment::Vertical::Center),
    )
    .padding([4, 8])
    .style(|theme: &iced::Theme, status| {
        use button::{Status, Style};

        let palette = theme.extended_palette();
        let base = Style {
            text_color: palette.background.base.text,
            border: Border::default().rounded(6.0),
            ..Style::default()
        };
        match status {
            Status::Active => base.with_background(Color::TRANSPARENT),
            Status::Hovered => base.with_background(Color::from_rgb(
                palette.primary.weak.color.r * 1.2,
                palette.primary.weak.color.g * 1.2,
                palette.primary.weak.color.b * 1.2,
            )),
            Status::Disabled => base.with_background(Color::from_rgb(0.5, 0.5, 0.5)),
            Status::Pressed => base.with_background(palette.primary.weak.color),
        }
    })
    .width(Length::Fill)
    .height(Length::Shrink);
    if let Some(message) = message {
        btn.on_press(message).into()
    } else {
        btn.into()
    }
}

pub fn submenu<Message>(
    label: impl Into<String>,
    msg: Message,
) -> Element<'static, Message, iced::Theme, iced::Renderer>
where
    Message: Clone + 'static,
{
    let label = label.into();
    base_button(
        row![
            text(label)
                .width(Length::Fill)
                .align_y(alignment::Vertical::Center),
            chevron_right(),
        ]
        .align_y(iced::Alignment::Center),
        msg,
    )
    .width(Length::Fill)
    .height(Length::Shrink)
    .into()
}

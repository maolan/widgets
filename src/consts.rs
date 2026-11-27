use iced::Color;
use std::time::Duration;

pub const DOUBLE_CLICK: Duration = Duration::from_millis(350);
pub const FADER_MIN_DB: f32 = -90.0;
pub const FADER_MAX_DB: f32 = 20.0;
pub const CONTROL_BACKGROUND_COLOR: Color = Color::from_rgb(
    0x42 as f32 / 255.0,
    0x46 as f32 / 255.0,
    0x4D as f32 / 255.0,
);
pub const CONTROL_BORDER_COLOR: Color = Color::from_rgb(
    0x30 as f32 / 255.0,
    0x33 as f32 / 255.0,
    0x3C as f32 / 255.0,
);
pub const CONTROL_FILLED_COLOR: Color = Color::from_rgb(
    0x29 as f32 / 255.0,
    0x66 as f32 / 255.0,
    0xA3 as f32 / 255.0,
);
pub const CONTROL_HANDLE_COLOR: Color = Color::from_rgb(
    0x75 as f32 / 255.0,
    0xC2 as f32 / 255.0,
    0xFF as f32 / 255.0,
);
pub const CONTROL_TEXT_COLOR: Color = Color::from_rgb(
    0xD7 as f32 / 255.0,
    0xDA as f32 / 255.0,
    0xE0 as f32 / 255.0,
);
pub const CONTROL_SELECTED_TEXT_COLOR: Color = Color::WHITE;

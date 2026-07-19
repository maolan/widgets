use iced::widget::{
    canvas,
    canvas::{Frame, Geometry, Path},
};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme, mouse};
use std::{
    cell::Cell,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

const FADER_MIN_DB: f32 = -90.0;
const FADER_MAX_DB: f32 = 20.0;
const METER_BAR_HEIGHT: f32 = 3.0;
const METER_BAR_GAP: f32 = 2.0;
const METER_PAD_Y: f32 = 3.0;
const OUTER_PAD_X: f32 = 7.0;

#[derive(Default)]
struct State {
    cache: canvas::Cache,
    last_hash: Cell<u64>,
}

#[derive(Clone)]
struct SmallMeterLevels {
    len: usize,
    data: [u8; 32],
}

impl SmallMeterLevels {
    fn from_db(levels_db: &[f32], channels: usize) -> Self {
        let len = channels.clamp(1, 32);
        let mut data = [0; 32];
        for (idx, slot) in data.iter_mut().take(len).enumerate() {
            *slot = level_to_qdb(levels_db.get(idx).copied().unwrap_or(FADER_MIN_DB));
        }
        Self { len, data }
    }

    fn get(&self, idx: usize) -> u8 {
        if idx < self.len { self.data[idx] } else { 0 }
    }
}

#[derive(Clone)]
struct HorizontalMeterCanvas {
    channels: usize,
    levels_qdb: SmallMeterLevels,
}

impl HorizontalMeterCanvas {
    fn static_hash(&self, bounds: Rectangle) -> u64 {
        let mut hasher = DefaultHasher::new();
        bounds.width.to_bits().hash(&mut hasher);
        bounds.height.to_bits().hash(&mut hasher);
        self.channels.hash(&mut hasher);
        hasher.finish()
    }
}

impl<Message> canvas::Program<Message> for HorizontalMeterCanvas {
    type State = State;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return vec![];
        }

        let static_hash = self.static_hash(bounds);
        if state.last_hash.get() != static_hash {
            state.cache.clear();
            state.last_hash.set(static_hash);
        }

        let static_geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            frame.fill(
                &Path::rectangle(Point::new(0.0, 0.0), bounds.size()),
                Color::from_rgba(0.09, 0.10, 0.12, 1.0),
            );
        });

        let meter_inner_w = (bounds.width - (OUTER_PAD_X * 2.0)).max(1.0);
        let mut dynamic_frame = Frame::new(renderer, bounds.size());
        for channel_idx in 0..self.channels.max(1) {
            let db = qdb_to_level(self.levels_qdb.get(channel_idx));
            let fill = level_to_meter_fill(db);
            let filled_w = (meter_inner_w * fill).max(1.0);
            let y = METER_PAD_Y + channel_idx as f32 * (METER_BAR_HEIGHT + METER_BAR_GAP);
            dynamic_frame.fill(
                &Path::rectangle(
                    Point::new(OUTER_PAD_X, y),
                    iced::Size::new(filled_w, METER_BAR_HEIGHT),
                ),
                meter_fill_color(db),
            );
        }

        vec![static_geometry, dynamic_frame.into_geometry()]
    }
}

fn level_to_meter_fill(level_db: f32) -> f32 {
    ((level_db - FADER_MIN_DB) / (FADER_MAX_DB - FADER_MIN_DB)).clamp(0.0, 1.0)
}

fn meter_fill_color(level_db: f32) -> Color {
    if level_db >= 0.0 {
        Color::from_rgb(0.96, 0.47, 0.34)
    } else if level_db >= -12.0 {
        Color::from_rgb(0.69, 0.86, 0.41)
    } else {
        Color::from_rgb(0.20, 0.78, 0.51)
    }
}

fn level_to_qdb(level_db: f32) -> u8 {
    (level_db
        .clamp(FADER_MIN_DB, FADER_MAX_DB)
        .round()
        .max(FADER_MIN_DB) as i16)
        .saturating_add(90)
        .clamp(0, 110) as u8
}

fn qdb_to_level(q: u8) -> f32 {
    q as f32 - 90.0
}

pub fn total_height(channels: usize) -> f32 {
    let channels = channels.max(1);
    channels as f32 * METER_BAR_HEIGHT
        + (channels.saturating_sub(1) as f32 * METER_BAR_GAP)
        + (METER_PAD_Y * 2.0)
}

pub fn horizontal_meter<'a, Message>(channels: usize, levels_db: &[f32]) -> Element<'a, Message>
where
    Message: 'a,
{
    canvas(HorizontalMeterCanvas {
        channels: channels.max(1),
        levels_qdb: SmallMeterLevels::from_db(levels_db, channels),
    })
    .width(Length::Fill)
    .height(Length::Fixed(total_height(channels)))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_height_uses_minimum_single_channel() {
        assert_eq!(total_height(0), total_height(1));
        assert_eq!(total_height(1), METER_BAR_HEIGHT + (METER_PAD_Y * 2.0));
    }

    #[test]
    fn total_height_scales_with_channel_count() {
        assert_eq!(total_height(2), 14.0);
        assert_eq!(total_height(4), 24.0);
    }

    #[test]
    fn level_quantization_roundtrips_whole_db_values() {
        for db in [-90.0, -48.0, -12.0, 0.0, 20.0] {
            assert_eq!(qdb_to_level(level_to_qdb(db)), db);
        }
    }

    #[test]
    fn meter_fill_color_switches_at_thresholds() {
        assert_eq!(meter_fill_color(1.0), Color::from_rgb(0.96, 0.47, 0.34));
        assert_eq!(meter_fill_color(-6.0), Color::from_rgb(0.69, 0.86, 0.41));
        assert_eq!(meter_fill_color(-18.0), Color::from_rgb(0.20, 0.78, 0.51));
    }

    #[test]
    fn level_to_meter_fill_clamps_to_range() {
        assert_eq!(level_to_meter_fill(FADER_MIN_DB - 10.0), 0.0);
        assert_eq!(level_to_meter_fill(FADER_MAX_DB + 10.0), 1.0);
        assert!((level_to_meter_fill(-35.0) - 0.5).abs() < 0.001);
    }
}

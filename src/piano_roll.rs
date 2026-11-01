use crate::{
    midi::{PITCH_MAX, PianoNote},
    piano::{note_color, note_two_edge_gradient, row_height},
};
use iced::{
    Element, Length, Point,
    widget::{container, pin},
};

/// Descending MIDI pitches displayed by the piano roll. All geometry and editing
/// must use this mapping when scale rows are collapsed.
#[derive(Clone, Debug)]
pub struct PitchRows {
    pitches: Vec<u8>,
}

impl PitchRows {
    pub fn new(scale: Option<(u8, &'static [u8])>) -> Self {
        let pitches = (0..=PITCH_MAX)
            .rev()
            .filter(|&pitch| {
                scale.is_none_or(|(root, pattern)| {
                    pattern.contains(&((i16::from(pitch) - i16::from(root)).rem_euclid(12) as u8))
                })
            })
            .collect();
        Self { pitches }
    }

    pub fn pitches(&self) -> &[u8] {
        &self.pitches
    }

    pub fn row(&self, pitch: u8) -> Option<usize> {
        self.pitches.iter().position(|&p| p == pitch)
    }

    pub fn pitch_at(&self, row: f32) -> u8 {
        self.pitches[row.floor().clamp(0.0, (self.pitches.len() - 1) as f32) as usize]
    }

    pub fn shift(&self, pitch: u8, rows: f32) -> u8 {
        self.row(pitch)
            .map_or(pitch, |row| self.pitch_at(row as f32 + rows.round()))
    }
}

pub struct PianoRoll<'a, Message> {
    notes: Vec<PianoNote>,
    clip_length_samples: usize,
    zoom_y: f32,
    pixels_per_sample: f32,
    zoom_x: f32,
    interaction: Element<'a, Message>,
    scale_filter: Option<(u8, &'static [u8])>,
}

impl<'a, Message: 'a> PianoRoll<'a, Message> {
    pub fn new(
        notes: Vec<PianoNote>,
        clip_length_samples: usize,
        zoom_y: f32,
        pixels_per_sample: f32,
        zoom_x: f32,
        interaction: Element<'a, Message>,
    ) -> Self {
        Self {
            notes,
            clip_length_samples,
            zoom_y,
            pixels_per_sample,
            zoom_x,
            interaction,
            scale_filter: None,
        }
    }

    pub fn with_scale_filter(mut self, root_semitone: u8, pattern: &'static [u8]) -> Self {
        self.scale_filter = Some((root_semitone, pattern));
        self
    }

    fn visible_notes(&self) -> impl Iterator<Item = &PianoNote> {
        self.notes.iter().filter(|note| {
            note.pitch <= PITCH_MAX
                && self.scale_filter.is_none_or(|(root, pattern)| {
                    let relative = (note.pitch as i16 - root as i16).rem_euclid(12) as u8;
                    pattern.contains(&relative)
                })
        })
    }

    pub fn into_element(self) -> Element<'a, Message> {
        let row_h = row_height(self.zoom_y);
        let pps = (self.pixels_per_sample * self.zoom_x).max(0.0001);
        let notes_w = (self.clip_length_samples as f32 * pps).max(1.0);
        let rows = PitchRows::new(self.scale_filter);
        let notes_h = rows.pitches().len() as f32 * row_h;

        let mut layers: Vec<Element<'a, Message>> = vec![];
        for note in self.visible_notes() {
            let y_idx = rows.row(note.pitch).expect("visible pitch");
            let y = y_idx as f32 * row_h + 1.0;
            let x = note.start_sample as f32 * pps;
            let w = (note.length_samples as f32 * pps).max(2.0);
            let color = note_color(note.velocity, note.channel);
            layers.push(
                pin(container("")
                    .width(Length::Fixed(w))
                    .height(Length::Fixed((row_h - 2.0).max(2.0)))
                    .style(move |_theme| container::Style {
                        background: Some(note_two_edge_gradient(color)),
                        ..container::Style::default()
                    }))
                .position(Point::new(x, y))
                .into(),
            );
        }

        layers.push(pin(self.interaction).position(Point::new(0.0, 0.0)).into());

        iced::widget::Stack::from_vec(layers)
            .width(Length::Fixed(notes_w))
            .height(Length::Fixed(notes_h))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(pitch: u8) -> PianoNote {
        PianoNote {
            start_sample: 0,
            length_samples: 16,
            pitch,
            velocity: 100,
            channel: 0,
            mpe: Default::default(),
        }
    }

    fn piano_roll() -> PianoRoll<'static, ()> {
        PianoRoll::new(
            vec![note(60), note(61), note(63)],
            256,
            1.0,
            1.0,
            1.0,
            container("").into(),
        )
    }

    #[test]
    fn collapsed_rows_round_trip_for_scale_patterns() {
        for pattern in [
            &[0, 2, 4, 5, 7, 9, 11][..],
            &[0, 2, 3, 5, 7, 8, 10][..],
            &[0, 3, 5, 6, 7, 10][..],
        ] {
            for root in 0..12 {
                let rows = PitchRows::new(Some((root, pattern)));
                for (row, &pitch) in rows.pitches().iter().enumerate() {
                    assert_eq!(rows.row(pitch), Some(row));
                    assert_eq!(rows.pitch_at(row as f32 + 0.5), pitch);
                }
                assert_eq!(rows.pitch_at(-100.0), rows.pitches()[0]);
                assert_eq!(rows.pitch_at(200.0), *rows.pitches().last().unwrap());
            }
        }
    }

    #[test]
    fn dragging_moves_by_visible_scale_steps() {
        let rows = PitchRows::new(Some((0, &[0, 2, 4, 5, 7, 9, 11])));
        assert_eq!(rows.row(61), None);
        assert_eq!(rows.shift(60, -1.0), 62);
        assert_eq!(rows.shift(60, 1.0), 59);
        assert_eq!(rows.shift(127, -10.0), 127);
        assert_eq!(rows.shift(0, 10.0), 0);
        let chromatic = PitchRows::new(None);
        assert_eq!(chromatic.pitches().len(), 128);
        assert_eq!(chromatic.shift(60, -1.0), 61);
    }

    #[test]
    fn scale_filter_controls_the_notes_used_for_rendering() {
        assert_eq!(piano_roll().visible_notes().count(), 3);

        let major: Vec<u8> = piano_roll()
            .with_scale_filter(0, &[0, 2, 4, 5, 7, 9, 11])
            .visible_notes()
            .map(|note| note.pitch)
            .collect();
        assert_eq!(major, vec![60]);

        let minor: Vec<u8> = piano_roll()
            .with_scale_filter(0, &[0, 2, 3, 5, 7, 8, 10])
            .visible_notes()
            .map(|note| note.pitch)
            .collect();
        assert_eq!(minor, vec![60, 63]);
    }
}

use crate::midi::PianoNote;
use iced::{
    Color, Event, Point, Rectangle, Renderer, Size, Theme, mouse,
    widget::canvas::{Action as CanvasAction, Frame, Geometry, Path, Program},
};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum DrumMessage {
    NoteSelected(usize),
    ClearSelection,
    NoteCreate {
        start_sample: usize,
        pitch: u8,
    },
    NoteDelete(usize),
    NoteMove {
        note_index: usize,
        delta_samples: i64,
    },
    AdjustVelocity {
        note_index: usize,
        delta: i8,
    },
    SelectRectStart {
        position: Point,
    },
    SelectRectDrag {
        position: Point,
    },
    SelectRectEnd,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum DraggingMode {
    #[default]
    None,
    SelectingRect,
    DraggingNote,
}

#[derive(Debug)]
pub struct DrumRollInteraction {
    pub notes: Vec<PianoNote>,
    pub pixels_per_sample: f32,
    pub zoom_x: f32,
    pub drum_rows: Vec<u8>,
    pub row_height: f32,
    pub selecting_rect: Option<(Point, Point)>,
    pub selected_notes: HashSet<usize>,
}

#[derive(Default, Debug)]
pub struct DrumRollInteractionState {
    pub dragging_mode: DraggingMode,
    pub drag_start: Option<Point>,
    pub drag_note_index: Option<usize>,
    pub hover_note_index: Option<usize>,
}

impl DrumRollInteraction {
    pub fn new(
        notes: Vec<PianoNote>,
        pixels_per_sample: f32,
        zoom_x: f32,
        drum_rows: Vec<u8>,
        row_height: f32,
        selecting_rect: Option<(Point, Point)>,
        selected_notes: HashSet<usize>,
    ) -> Self {
        Self {
            notes,
            pixels_per_sample,
            zoom_x,
            drum_rows,
            row_height,
            selecting_rect,
            selected_notes,
        }
    }

    fn note_at_position(&self, position: Point, pps: f32, notes: &[PianoNote]) -> Option<usize> {
        for (idx, note) in notes.iter().enumerate() {
            let Some(row_idx) = self.drum_rows.iter().position(|&p| p == note.pitch) else {
                continue;
            };
            let y = row_idx as f32 * self.row_height + 1.0;
            let x = note.start_sample as f32 * pps;
            let w = (note.length_samples as f32 * pps).max(2.0);
            let h = (self.row_height - 2.0).max(2.0);
            if position.x >= x && position.x <= x + w && position.y >= y && position.y <= y + h {
                return Some(idx);
            }
        }
        None
    }

    fn pitch_at_y(&self, y: f32) -> u8 {
        let row_idx = (y / self.row_height)
            .floor()
            .clamp(0.0, (self.drum_rows.len().saturating_sub(1)) as f32)
            as usize;
        self.drum_rows.get(row_idx).copied().unwrap_or(60)
    }

    fn sample_at_x(&self, x: f32, pps: f32) -> usize {
        (x / pps).max(0.0) as usize
    }
}

impl Program<DrumMessage> for DrumRollInteraction {
    type State = DrumRollInteractionState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<CanvasAction<DrumMessage>> {
        let pps = (self.pixels_per_sample * self.zoom_x).max(0.0001);
        let notes = &self.notes;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(position) = cursor.position_in(bounds) {
                    if let Some(note_idx) = self.note_at_position(position, pps, notes) {
                        state.drag_start = Some(position);
                        state.drag_note_index = Some(note_idx);
                        state.dragging_mode = DraggingMode::DraggingNote;
                        return Some(
                            CanvasAction::publish(DrumMessage::NoteSelected(note_idx))
                                .and_capture(),
                        );
                    } else {
                        state.drag_start = Some(position);
                        state.drag_note_index = None;
                        state.dragging_mode = DraggingMode::SelectingRect;
                        return Some(
                            CanvasAction::publish(DrumMessage::SelectRectStart { position })
                                .and_capture(),
                        );
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if let Some(position) = cursor.position_in(bounds) {
                    let pitch = self.pitch_at_y(position.y);
                    let start_sample = self.sample_at_x(position.x, pps);
                    return Some(
                        CanvasAction::publish(DrumMessage::NoteCreate {
                            start_sample,
                            pitch,
                        })
                        .and_capture(),
                    );
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                if let Some(position) = cursor.position_in(bounds)
                    && let Some(note_idx) = self.note_at_position(position, pps, notes)
                {
                    return Some(
                        CanvasAction::publish(DrumMessage::NoteDelete(note_idx)).and_capture(),
                    );
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(position) = cursor.position_in(bounds) {
                    match state.dragging_mode {
                        DraggingMode::SelectingRect => {
                            return Some(CanvasAction::publish(DrumMessage::SelectRectDrag {
                                position,
                            }));
                        }
                        DraggingMode::DraggingNote => {
                            return Some(CanvasAction::request_redraw());
                        }
                        DraggingMode::None => {}
                    }
                    state.hover_note_index = self.note_at_position(position, pps, notes);
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let mode = state.dragging_mode;

                match mode {
                    DraggingMode::SelectingRect => {
                        state.drag_start = None;
                        state.drag_note_index = None;
                        state.dragging_mode = DraggingMode::None;
                        return Some(CanvasAction::publish(DrumMessage::SelectRectEnd));
                    }
                    DraggingMode::DraggingNote => {
                        if let (Some(drag_start), Some(note_idx)) =
                            (state.drag_start.take(), state.drag_note_index.take())
                        {
                            state.dragging_mode = DraggingMode::None;
                            if let Some(position) = cursor.position_in(bounds) {
                                let delta_x = position.x - drag_start.x;
                                let delta_samples = (delta_x / pps) as i64;
                                if delta_samples != 0 {
                                    return Some(
                                        CanvasAction::publish(DrumMessage::NoteMove {
                                            note_index: note_idx,
                                            delta_samples,
                                        })
                                        .and_capture(),
                                    );
                                }
                            }
                        }
                    }
                    DraggingMode::None => {}
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if let Some(position) = cursor.position_in(bounds) {
                    let raw = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => *y,
                        mouse::ScrollDelta::Pixels { y, .. } => *y / 16.0,
                    };
                    let steps = raw.round() as i32;
                    if steps != 0
                        && let Some(note_idx) = self.note_at_position(position, pps, notes)
                    {
                        let delta = steps.clamp(-24, 24) as i8;
                        return Some(
                            CanvasAction::publish(DrumMessage::AdjustVelocity {
                                note_index: note_idx,
                                delta,
                            })
                            .and_capture(),
                        );
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        if state.dragging_mode == DraggingMode::DraggingNote
            && let (Some(drag_start), Some(cursor_pos)) =
                (state.drag_start, cursor.position_in(bounds))
        {
            let pps = (self.pixels_per_sample * self.zoom_x).max(0.0001);
            let delta_x = cursor_pos.x - drag_start.x;
            for &note_idx in &self.selected_notes {
                if let Some(note) = self.notes.get(note_idx)
                    && let Some(row_idx) = self.drum_rows.iter().position(|&p| p == note.pitch)
                {
                    let x = note.start_sample as f32 * pps + delta_x;
                    let y = row_idx as f32 * self.row_height + 1.0;
                    let w = (note.length_samples as f32 * pps).max(2.0);
                    let h = (self.row_height - 2.0).max(2.0);
                    frame.fill(
                        &Path::rectangle(Point::new(x, y), Size::new(w, h)),
                        Color::from_rgba(0.9, 0.9, 0.95, 0.35),
                    );
                }
            }
        }

        if let Some(note_idx) = state.hover_note_index
            && let Some(note) = self.notes.get(note_idx)
            && let Some(row_idx) = self.drum_rows.iter().position(|&p| p == note.pitch)
        {
            let pps = (self.pixels_per_sample * self.zoom_x).max(0.0001);
            let x = note.start_sample as f32 * pps;
            let y = row_idx as f32 * self.row_height + 1.0;
            let w = (note.length_samples as f32 * pps).max(2.0);
            let h = (self.row_height - 2.0).max(2.0);
            frame.stroke(
                &Path::rectangle(Point::new(x, y), Size::new(w, h)),
                iced::widget::canvas::Stroke::default()
                    .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.6))
                    .with_width(1.5),
            );
        }

        if let Some((start, end)) = self.selecting_rect {
            let min_x = start.x.min(end.x);
            let min_y = start.y.min(end.y);
            let max_x = start.x.max(end.x);
            let max_y = start.y.max(end.y);

            let rect = Rectangle {
                x: min_x,
                y: min_y,
                width: max_x - min_x,
                height: max_y - min_y,
            };

            frame.fill(
                &Path::rectangle(
                    Point::new(rect.x, rect.y),
                    Size::new(rect.width, rect.height),
                ),
                Color {
                    r: 0.3,
                    g: 0.5,
                    b: 0.8,
                    a: 0.2,
                },
            );
            frame.stroke(
                &Path::rectangle(
                    Point::new(rect.x, rect.y),
                    Size::new(rect.width, rect.height),
                ),
                iced::widget::canvas::Stroke::default()
                    .with_color(Color::from_rgb(0.4, 0.6, 0.9))
                    .with_width(1.5),
            );
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::PianoNote;
    use iced::widget::canvas::Program;
    use iced::{Event, Point, Rectangle, Size, event, mouse};
    use std::collections::HashSet;

    fn action_message(action: CanvasAction<DrumMessage>) -> (Option<DrumMessage>, event::Status) {
        let (message, _redraw, status) = action.into_inner();
        (message, status)
    }

    fn drum_note(start_sample: usize, pitch: u8) -> PianoNote {
        PianoNote {
            start_sample,
            length_samples: 20,
            pitch,
            velocity: 100,
            channel: 0,
            mpe: Default::default(),
        }
    }

    #[test]
    fn drum_roll_click_on_note_selects_and_starts_drag() {
        let interaction = DrumRollInteraction::new(
            vec![drum_note(10, 38)],
            1.0,
            1.0,
            vec![36, 38],
            20.0,
            None,
            HashSet::new(),
        );
        let mut state = DrumRollInteractionState::default();
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(200.0, 100.0));
        let cursor = mouse::Cursor::Available(Point::new(15.0, 22.0));

        let action = interaction
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds,
                cursor,
            )
            .expect("action");

        let (message, status) = action_message(action);
        assert_eq!(message, Some(DrumMessage::NoteSelected(0)));
        assert_eq!(status, event::Status::Captured);
        assert_eq!(state.dragging_mode, DraggingMode::DraggingNote);
        assert_eq!(state.drag_note_index, Some(0));
    }

    #[test]
    fn drum_roll_drag_release_publishes_move_with_delta() {
        let interaction = DrumRollInteraction::new(
            vec![drum_note(10, 38)],
            1.0,
            1.0,
            vec![36, 38],
            20.0,
            None,
            HashSet::new(),
        );
        let mut state = DrumRollInteractionState::default();
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(200.0, 100.0));
        let press_cursor = mouse::Cursor::Available(Point::new(15.0, 22.0));
        let release_cursor = mouse::Cursor::Available(Point::new(35.0, 22.0));

        let _ = interaction.update(
            &mut state,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            bounds,
            press_cursor,
        );

        let action = interaction
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                bounds,
                release_cursor,
            )
            .expect("release action");

        let (message, status) = action_message(action);
        assert_eq!(
            message,
            Some(DrumMessage::NoteMove {
                note_index: 0,
                delta_samples: 20,
            })
        );
        assert_eq!(status, event::Status::Captured);
    }

    #[test]
    fn drum_roll_cursor_moved_while_dragging_requests_redraw() {
        let interaction = DrumRollInteraction::new(
            vec![drum_note(10, 38)],
            1.0,
            1.0,
            vec![36, 38],
            20.0,
            None,
            HashSet::new(),
        );
        let mut state = DrumRollInteractionState::default();
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(200.0, 100.0));
        let press_cursor = mouse::Cursor::Available(Point::new(15.0, 22.0));
        let drag_cursor = mouse::Cursor::Available(Point::new(35.0, 22.0));

        let _ = interaction.update(
            &mut state,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            bounds,
            press_cursor,
        );

        let action = interaction
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved {
                    position: Point::new(35.0, 22.0),
                }),
                bounds,
                drag_cursor,
            )
            .expect("drag action");

        let (message, _status) = action_message(action);
        assert!(message.is_none());
    }
}

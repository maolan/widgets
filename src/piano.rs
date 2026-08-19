use crate::midi::{MIDI_NOTE_COUNT, NOTES_PER_OCTAVE, WHITE_KEY_HEIGHT, WHITE_KEYS_PER_OCTAVE};
use iced::{
    Background, Color, Event, Point, Rectangle, Renderer, Size, Theme, gradient, mouse,
    widget::canvas::{self, Action as CanvasAction, Frame, Geometry, Path, Program},
};
use std::collections::{HashMap, HashSet};

pub fn is_black_key(pitch: u8) -> bool {
    matches!(pitch % 12, 1 | 3 | 6 | 8 | 10)
}

pub fn note_color(velocity: u8, channel: u8) -> Color {
    let t = (velocity as f32 / 127.0).clamp(0.0, 1.0);
    let c = (channel as f32 / 15.0).clamp(0.0, 1.0);
    Color {
        r: 0.25 + 0.45 * t,
        g: 0.35 + 0.4 * (1.0 - c),
        b: 0.65 + 0.3 * c,
        a: 0.9,
    }
}

pub fn brighten(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r + amount).min(1.0),
        g: (color.g + amount).min(1.0),
        b: (color.b + amount).min(1.0),
        a: color.a,
    }
}

pub fn darken(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r - amount).max(0.0),
        g: (color.g - amount).max(0.0),
        b: (color.b - amount).max(0.0),
        a: color.a,
    }
}

pub fn note_two_edge_gradient(base: Color) -> Background {
    let edge = brighten(base, 0.08);
    let middle = darken(base, 0.08);
    Background::Gradient(
        gradient::Linear::new(0.0)
            .add_stop(0.0, edge)
            .add_stop(0.5, middle)
            .add_stop(1.0, edge)
            .into(),
    )
}

pub fn octave_note_count(octave: u8) -> u8 {
    let start = usize::from(octave) * NOTES_PER_OCTAVE;
    if start >= MIDI_NOTE_COUNT {
        0
    } else {
        (MIDI_NOTE_COUNT - start).min(NOTES_PER_OCTAVE) as u8
    }
}

/// Orientation of a piano-style keyboard within its bounding rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Horizontal, white keys run left-to-right, black keys protrude to the top.
    #[default]
    Degree0,
    /// Vertical, white keys run top-to-bottom, black keys protrude to the right.
    Degree90,
    /// Horizontal, white keys run left-to-right, black keys protrude to the bottom.
    Degree180,
    /// Vertical, white keys run bottom-to-top, black keys protrude to the left.
    Degree270,
}

/// Ratio of the keyboard's cross-axis depth that black keys occupy.
///
/// This matches the original Maolan vertical keyboard, where black keys
/// protrude 60% of the keyboard depth.
const BLACK_KEY_DEPTH_RATIO: f32 = 0.6;

/// Ratio of a white key's length that a black key occupies along the note axis.
const BLACK_KEY_LENGTH_RATIO: f32 = 0.6;

fn velocity_from_v(v: f32, depth: f32) -> u8 {
    let normalized = (v / depth).clamp(0.0, 1.0);
    (normalized * 126.0).round() as u8 + 1
}

/// Hit-test a point inside a keyboard range drawn by `draw_keyboard_range_into`.
///
/// `cursor` and `bounds` must be in the same canvas coordinate space. The returned
/// tuple is `(note_class, velocity)` where `note_class` is the MIDI note class
/// within the octave (0..11) and `velocity` is in the range 1..=127.
pub fn note_at_in_range(
    cursor: Point,
    bounds: Rectangle,
    orientation: Orientation,
    note_count: u8,
) -> Option<(u8, u8)> {
    if note_count == 0 {
        return None;
    }
    if orientation == Orientation::Degree270 {
        return note_at_in_range_degree270(cursor, bounds, note_count);
    }
    let natural_cursor = to_natural_cursor(orientation, bounds, cursor);
    let natural = natural_bounds(orientation, bounds);
    let height = natural.height;
    let width = natural.width;
    let white_notes: &[u8] = if note_count >= 12 {
        &[0, 2, 4, 5, 7, 9, 11]
    } else {
        &[0, 2, 4, 5, 7]
    };
    let white_key_count = white_notes.iter().take_while(|&n| *n < note_count).count() as f32;
    if white_key_count == 0.0 {
        return None;
    }
    let white_key_size = height / white_key_count;
    let black_key_width = width * BLACK_KEY_DEPTH_RATIO;
    let black_key_length = white_key_size * BLACK_KEY_LENGTH_RATIO;
    let u = natural_cursor.x.clamp(0.0, height);
    let v = natural_cursor.y;

    let (black_offsets, black_notes): (&[u8], &[u8]) = if note_count >= 12 {
        (&[1, 2, 4, 5, 6], &[1, 3, 6, 8, 10])
    } else {
        (&[1, 2, 4], &[1, 3, 6])
    };

    if v <= black_key_width {
        for (idx, &note_id) in black_notes
            .iter()
            .enumerate()
            .take_while(|&(_, n)| *n < note_count)
        {
            let center = black_offsets[idx] as f32 * white_key_size;
            if (u - center).abs() <= black_key_length * 0.5 {
                let velocity = velocity_from_v(v, black_key_width);
                return Some((note_id, velocity));
            }
        }
    }

    let index = (u / white_key_size)
        .clamp(0.0, white_key_count - 1.0)
        .floor() as usize;
    let note_id = white_notes.get(index).copied()?;
    let velocity = velocity_from_v(v, width);
    Some((note_id, velocity))
}

fn note_at_in_range_degree270(
    cursor: Point,
    bounds: Rectangle,
    note_count: u8,
) -> Option<(u8, u8)> {
    if note_count < NOTES_PER_OCTAVE as u8 {
        let white_note_ids = [0_u8, 2, 4, 5, 7];
        let black_key_offsets = [1_u8, 2, 4];
        let black_note_ids = [1_u8, 3, 6];
        let white_key_height = bounds.height / white_note_ids.len() as f32;
        let black_key_width = bounds.width * BLACK_KEY_DEPTH_RATIO;
        let black_key_height = white_key_height * BLACK_KEY_LENGTH_RATIO;

        if cursor.x <= black_key_width {
            for (idx, offset) in black_key_offsets.iter().enumerate() {
                let note_id = black_note_ids[idx];
                if note_id >= note_count {
                    continue;
                }
                let y_pos_black = bounds.height
                    - (f32::from(*offset) * white_key_height)
                    - (black_key_height * 0.5);
                if cursor.y >= y_pos_black && cursor.y <= y_pos_black + black_key_height {
                    let velocity = velocity_from_v(cursor.x, black_key_width);
                    return Some((note_id, velocity));
                }
            }
        }

        for (i, note_id) in white_note_ids.iter().enumerate() {
            let y_pos = bounds.height - ((i + 1) as f32 * white_key_height);
            if cursor.y >= y_pos && cursor.y <= y_pos + white_key_height {
                let velocity = velocity_from_v(cursor.x, bounds.width);
                return Some((*note_id, velocity));
            }
        }
        return None;
    }
    let white_key_height = bounds.height / 7.0;
    let black_key_offsets = [1, 2, 4, 5, 6];
    let black_note_ids = [1, 3, 6, 8, 10];
    let black_key_width = bounds.width * BLACK_KEY_DEPTH_RATIO;
    let black_key_height = white_key_height * BLACK_KEY_LENGTH_RATIO;

    if cursor.x <= black_key_width {
        for (idx, offset) in black_key_offsets.iter().enumerate() {
            let y_pos_black =
                bounds.height - (*offset as f32 * white_key_height) - (black_key_height * 0.5);
            if cursor.y >= y_pos_black && cursor.y <= y_pos_black + black_key_height {
                let velocity = velocity_from_v(cursor.x, black_key_width);
                return Some((black_note_ids[idx], velocity));
            }
        }
    }

    for i in 0..7 {
        let note_id = match i {
            0 => 0,
            1 => 2,
            2 => 4,
            3 => 5,
            4 => 7,
            5 => 9,
            6 => 11,
            _ => 0,
        };
        let y_pos = bounds.height - ((i + 1) as f32 * white_key_height);
        if cursor.y >= y_pos && cursor.y <= y_pos + white_key_height {
            let velocity = velocity_from_v(cursor.x, bounds.width);
            return Some((note_id, velocity));
        }
    }
    None
}

/// Map a point from the keyboard's natural coordinate system to canvas space.
///
/// In the natural coordinate system `u` runs along the note axis from the
/// lowest note (0) to the highest note (length), and `v` runs across the
/// front of the keys from the black-key side (0) to the white-key front (width).
pub fn map_point(orientation: Orientation, bounds: Rectangle, u: f32, v: f32) -> Point {
    let x = bounds.x;
    let y = bounds.y;
    let w = bounds.width;
    let h = bounds.height;
    match orientation {
        Orientation::Degree0 => Point::new(x + u, y + v),
        Orientation::Degree90 => Point::new(x + w - v, y + u),
        Orientation::Degree180 => Point::new(x + u, y + h - v),
        Orientation::Degree270 => Point::new(x + v, y + h - u),
    }
}

/// Map a natural-coordinate rectangle `[u_low, u_high] x [v_low, v_high]` to a
/// canvas-space origin and size suitable for `Path::rectangle`.
fn map_rect(
    orientation: Orientation,
    bounds: Rectangle,
    u_low: f32,
    u_high: f32,
    v_low: f32,
    v_high: f32,
) -> (Point, Size) {
    let x = bounds.x;
    let y = bounds.y;
    match orientation {
        Orientation::Degree0 => (
            Point::new(x + u_low, y + v_low),
            Size::new(u_high - u_low, v_high - v_low),
        ),
        Orientation::Degree90 => (
            Point::new(x + bounds.width - v_high, y + u_low),
            Size::new(v_high - v_low, u_high - u_low),
        ),
        Orientation::Degree180 => (
            Point::new(x + u_low, y + bounds.height - v_high),
            Size::new(u_high - u_low, v_high - v_low),
        ),
        Orientation::Degree270 => (
            Point::new(x + v_low, y + bounds.height - u_high),
            Size::new(v_high - v_low, u_high - u_low),
        ),
    }
}

/// Bounds in the natural coordinate system used by the drawing code.
fn natural_bounds(orientation: Orientation, bounds: Rectangle) -> Rectangle {
    match orientation {
        Orientation::Degree90 | Orientation::Degree270 => bounds,
        Orientation::Degree0 | Orientation::Degree180 => Rectangle {
            x: bounds.x,
            y: bounds.y,
            width: bounds.height,
            height: bounds.width,
        },
    }
}

/// Convert a cursor position into the keyboard's natural `(u, v)` coordinates.
fn to_natural_cursor(orientation: Orientation, bounds: Rectangle, cursor: Point) -> Point {
    let w = bounds.width;
    let h = bounds.height;
    match orientation {
        Orientation::Degree0 => Point::new(cursor.x, cursor.y),
        Orientation::Degree90 => Point::new(cursor.y, w - cursor.x),
        Orientation::Degree180 => Point::new(cursor.x, h - cursor.y),
        Orientation::Degree270 => Point::new(h - cursor.y, cursor.x),
    }
}

/// Choose text alignment for a key label based on orientation.
fn text_align_x(orientation: Orientation) -> iced::advanced::text::Alignment {
    use iced::advanced::text::Alignment;
    match orientation {
        Orientation::Degree0 => Alignment::Center,
        Orientation::Degree90 => Alignment::Left,
        Orientation::Degree180 => Alignment::Center,
        Orientation::Degree270 => Alignment::Right,
    }
}

fn text_align_y(orientation: Orientation) -> iced::alignment::Vertical {
    use iced::alignment::Vertical;
    match orientation {
        Orientation::Degree0 => Vertical::Top,
        Orientation::Degree90 => Vertical::Center,
        Orientation::Degree180 => Vertical::Bottom,
        Orientation::Degree270 => Vertical::Center,
    }
}

fn draw_keyboard_range_into(
    frame: &mut Frame<Renderer>,
    bounds: Rectangle,
    pressed_notes: &HashSet<u8>,
    octave: u8,
    midnam_note_names: &HashMap<u8, String>,
    orientation: Orientation,
    note_count: u8,
) {
    if note_count == 0 {
        return;
    }
    let natural = natural_bounds(orientation, bounds);
    let height = natural.height;
    let width = natural.width;
    let white_notes: &[u8] = if note_count >= 12 {
        &[0, 2, 4, 5, 7, 9, 11]
    } else {
        &[0, 2, 4, 5, 7]
    };
    let white_key_count = white_notes.iter().take_while(|&&n| n < note_count).count() as f32;
    if white_key_count == 0.0 {
        return;
    }
    let white_key_size = height / white_key_count;
    let black_key_width = width * BLACK_KEY_DEPTH_RATIO;
    let black_key_length = white_key_size * BLACK_KEY_LENGTH_RATIO;
    let stroke = canvas::Stroke::default().with_width(1.0);
    let h_align = text_align_x(orientation);
    let v_align = text_align_y(orientation);

    // White keys, drawn full depth; black keys are overlaid on the back half.
    for (i, &note_id) in white_notes
        .iter()
        .enumerate()
        .take_while(|&(_, n)| *n < note_count)
    {
        let midi_note = octave * 12 + note_id;
        let is_pressed = pressed_notes.contains(&note_id);
        let color = if is_pressed {
            Color::from_rgb(0.0, 0.5, 1.0)
        } else {
            Color::WHITE
        };
        let u_low = i as f32 * white_key_size;
        let u_high = u_low + white_key_size - 1.0;

        let (origin, size) = map_rect(orientation, bounds, u_low, u_high, 0.0, width);
        let rect = Path::rectangle(origin, size);
        frame.fill(&rect, color);
        frame.stroke(&rect, stroke);

        if let Some(note_name) = midnam_note_names.get(&midi_note) {
            use iced::widget::canvas::Text;
            let text_pos = map_point(orientation, bounds, (u_low + u_high) * 0.5, width - 4.0);
            frame.fill_text(Text {
                content: note_name.clone(),
                position: text_pos,
                color: Color::BLACK,
                size: 10.0.into(),
                align_x: h_align,
                align_y: v_align,
                ..Text::default()
            });
        }
    }

    let (black_offsets, black_notes): (&[u8], &[u8]) = if note_count >= 12 {
        (&[1, 2, 4, 5, 6], &[1, 3, 6, 8, 10])
    } else {
        (&[1, 2, 4], &[1, 3, 6])
    };

    for (idx, &note_id) in black_notes
        .iter()
        .enumerate()
        .take_while(|&(_, n)| *n < note_count)
    {
        let is_pressed = pressed_notes.contains(&note_id);
        let color = if is_pressed {
            Color::from_rgb(0.0, 0.4, 0.8)
        } else {
            Color::BLACK
        };
        let center = black_offsets[idx] as f32 * white_key_size;
        let (origin, size) = map_rect(
            orientation,
            bounds,
            center - black_key_length * 0.5,
            center + black_key_length * 0.5,
            0.0,
            black_key_width,
        );
        let rect = Path::rectangle(origin, size);
        frame.fill(&rect, color);
    }
}

pub fn draw_octave_into(
    frame: &mut Frame<Renderer>,
    bounds: Rectangle,
    pressed_notes: &HashSet<u8>,
    octave: u8,
    midnam_note_names: &HashMap<u8, String>,
    orientation: Orientation,
) {
    draw_keyboard_range_into(
        frame,
        bounds,
        pressed_notes,
        octave,
        midnam_note_names,
        orientation,
        12,
    );
}

pub fn draw_partial_octave_into(
    frame: &mut Frame<Renderer>,
    bounds: Rectangle,
    pressed_notes: &HashSet<u8>,
    octave: u8,
    midnam_note_names: &HashMap<u8, String>,
    orientation: Orientation,
) {
    draw_keyboard_range_into(
        frame,
        bounds,
        pressed_notes,
        octave,
        midnam_note_names,
        orientation,
        octave_note_count(octave),
    );
}

fn draw_octave_degree270(
    renderer: &Renderer,
    bounds: Rectangle,
    pressed_notes: &HashSet<u8>,
    octave: u8,
    midnam_note_names: &HashMap<u8, String>,
) -> Vec<canvas::Geometry> {
    let mut frame = Frame::new(renderer, bounds.size());
    let white_key_height = bounds.height / 7.0;

    for i in 0..7 {
        let note_id = match i {
            0 => 0,
            1 => 2,
            2 => 4,
            3 => 5,
            4 => 7,
            5 => 9,
            6 => 11,
            _ => 0,
        };
        let midi_note = octave * 12 + note_id;
        let is_pressed = pressed_notes.contains(&note_id);
        let y_pos = bounds.height - ((i + 1) as f32 * white_key_height);
        let rect = Path::rectangle(
            Point::new(0.0, y_pos),
            Size::new(bounds.width, white_key_height - 1.0),
        );

        frame.fill(
            &rect,
            if is_pressed {
                Color::from_rgb(0.0, 0.5, 1.0)
            } else {
                Color::WHITE
            },
        );
        frame.stroke(&rect, canvas::Stroke::default().with_width(1.0));

        if let Some(note_name) = midnam_note_names.get(&midi_note) {
            use iced::widget::canvas::Text;
            frame.fill_text(Text {
                content: note_name.clone(),
                position: Point::new(bounds.width - 25.0, y_pos + white_key_height * 0.5 - 6.0),
                color: Color::BLACK,
                size: 10.0.into(),
                ..Text::default()
            });
        }
    }

    let black_key_offsets = [1, 2, 4, 5, 6];
    let black_note_ids = [1, 3, 6, 8, 10];
    let black_key_width = bounds.width * 0.6;
    let black_key_height = white_key_height * 0.6;

    for (idx, offset) in black_key_offsets.iter().enumerate() {
        let note_id = black_note_ids[idx];
        let is_pressed = pressed_notes.contains(&note_id);
        let y_pos_black =
            bounds.height - (*offset as f32 * white_key_height) - (black_key_height * 0.5);
        let rect = Path::rectangle(
            Point::new(0.0, y_pos_black),
            Size::new(black_key_width, black_key_height),
        );

        frame.fill(
            &rect,
            if is_pressed {
                Color::from_rgb(0.0, 0.4, 0.8)
            } else {
                Color::BLACK
            },
        );
    }

    vec![frame.into_geometry()]
}

fn draw_partial_octave_degree270(
    renderer: &Renderer,
    bounds: Rectangle,
    pressed_notes: &HashSet<u8>,
    octave: u8,
    midnam_note_names: &HashMap<u8, String>,
    note_count: u8,
) -> Vec<canvas::Geometry> {
    let mut frame = Frame::new(renderer, bounds.size());
    let white_note_ids = [0_u8, 2, 4, 5, 7];
    let black_key_offsets = [1_u8, 2, 4];
    let black_note_ids = [1_u8, 3, 6];
    let white_key_height = bounds.height / white_note_ids.len() as f32;
    let black_key_height = white_key_height * 0.6;
    let black_key_width = bounds.width * 0.6;

    for (i, note_id) in white_note_ids.iter().enumerate() {
        let midi_note = octave * 12 + *note_id;
        let is_pressed = pressed_notes.contains(note_id);
        let y_pos = bounds.height - ((i + 1) as f32 * white_key_height);
        let rect = Path::rectangle(
            Point::new(0.0, y_pos),
            Size::new(bounds.width, white_key_height - 1.0),
        );
        frame.fill(
            &rect,
            if is_pressed {
                Color::from_rgb(0.0, 0.5, 1.0)
            } else {
                Color::WHITE
            },
        );
        frame.stroke(&rect, canvas::Stroke::default().with_width(1.0));
        if let Some(note_name) = midnam_note_names.get(&midi_note) {
            use iced::widget::canvas::Text;
            frame.fill_text(Text {
                content: note_name.clone(),
                position: Point::new(bounds.width - 25.0, y_pos + white_key_height * 0.5 - 6.0),
                color: Color::BLACK,
                size: 10.0.into(),
                ..Text::default()
            });
        }
    }

    for (idx, offset) in black_key_offsets.iter().enumerate() {
        let note_id = black_note_ids[idx];
        if note_id >= note_count {
            continue;
        }
        let is_pressed = pressed_notes.contains(&note_id);
        let y_pos_black =
            bounds.height - (f32::from(*offset) * white_key_height) - (black_key_height * 0.5);
        let rect = Path::rectangle(
            Point::new(0.0, y_pos_black),
            Size::new(black_key_width, black_key_height),
        );
        frame.fill(
            &rect,
            if is_pressed {
                Color::from_rgb(0.0, 0.4, 0.8)
            } else {
                Color::BLACK
            },
        );
    }

    vec![frame.into_geometry()]
}

pub fn draw_octave(
    renderer: &Renderer,
    bounds: Rectangle,
    pressed_notes: &HashSet<u8>,
    octave: u8,
    midnam_note_names: &HashMap<u8, String>,
    orientation: Orientation,
) -> Vec<canvas::Geometry> {
    if orientation == Orientation::Degree270 {
        return draw_octave_degree270(renderer, bounds, pressed_notes, octave, midnam_note_names);
    }
    let mut frame = Frame::new(renderer, bounds.size());
    let local_bounds = Rectangle {
        x: 0.0,
        y: 0.0,
        width: bounds.width,
        height: bounds.height,
    };
    draw_octave_into(
        &mut frame,
        local_bounds,
        pressed_notes,
        octave,
        midnam_note_names,
        orientation,
    );
    vec![frame.into_geometry()]
}

pub fn draw_partial_octave(
    renderer: &Renderer,
    bounds: Rectangle,
    pressed_notes: &HashSet<u8>,
    octave: u8,
    midnam_note_names: &HashMap<u8, String>,
    orientation: Orientation,
) -> Vec<canvas::Geometry> {
    if orientation == Orientation::Degree270 {
        return draw_partial_octave_degree270(
            renderer,
            bounds,
            pressed_notes,
            octave,
            midnam_note_names,
            octave_note_count(octave),
        );
    }
    let mut frame = Frame::new(renderer, bounds.size());
    let local_bounds = Rectangle {
        x: 0.0,
        y: 0.0,
        width: bounds.width,
        height: bounds.height,
    };
    draw_partial_octave_into(
        &mut frame,
        local_bounds,
        pressed_notes,
        octave,
        midnam_note_names,
        orientation,
    );
    vec![frame.into_geometry()]
}

#[derive(Debug, Clone)]
pub struct OctaveKeyboard<Message, Press, Release>
where
    Press: Fn(u8, u8) -> Message + Clone,
    Release: Fn(u8) -> Message + Clone,
{
    pub octave: u8,
    pub note_count: u8,
    pub midnam_note_names: HashMap<u8, String>,
    pub orientation: Orientation,
    /// Cached names count for widget identity
    on_press: Press,
    on_release: Release,
}

impl<Message, Press, Release> OctaveKeyboard<Message, Press, Release>
where
    Press: Fn(u8, u8) -> Message + Clone,
    Release: Fn(u8) -> Message + Clone,
{
    pub fn new(
        octave: u8,
        midnam_note_names: HashMap<u8, String>,
        on_press: Press,
        on_release: Release,
    ) -> Self {
        Self {
            octave,
            note_count: octave_note_count(octave),
            midnam_note_names,
            orientation: Orientation::Degree0,
            on_press,
            on_release,
        }
    }

    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    fn note_at(&self, cursor: Point, bounds: Rectangle) -> Option<(u8, u8)> {
        note_at_in_range(cursor, bounds, self.orientation, self.note_count)
    }

    fn midi_note(&self, note_class: u8) -> u8 {
        (usize::from(self.octave) * 12 + usize::from(note_class)) as u8
    }
}

#[derive(Default, Debug)]
pub struct OctaveKeyboardState {
    pub pressed_notes: HashSet<u8>,
    pub active_note_class: Option<u8>,
}

impl<Message, Press, Release> Program<Message> for OctaveKeyboard<Message, Press, Release>
where
    Message: 'static,
    Press: Fn(u8, u8) -> Message + Clone,
    Release: Fn(u8) -> Message + Clone,
{
    type State = OctaveKeyboardState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<CanvasAction<Message>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(position) = cursor.position_in(bounds)
                    && let Some((note_class, velocity)) = self.note_at(position, bounds)
                {
                    state.active_note_class = Some(note_class);
                    state.pressed_notes.clear();
                    state.pressed_notes.insert(note_class);
                    return Some(
                        CanvasAction::publish((self.on_press.clone())(
                            self.midi_note(note_class),
                            velocity,
                        ))
                        .and_capture(),
                    );
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(note_class) = state.active_note_class.take() {
                    state.pressed_notes.clear();
                    return Some(CanvasAction::publish((self.on_release.clone())(
                        self.midi_note(note_class),
                    )));
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
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        if self.note_count == NOTES_PER_OCTAVE as u8 {
            draw_octave(
                renderer,
                bounds,
                &state.pressed_notes,
                self.octave,
                &self.midnam_note_names,
                self.orientation,
            )
        } else {
            draw_partial_octave(
                renderer,
                bounds,
                &state.pressed_notes,
                self.octave,
                &self.midnam_note_names,
                self.orientation,
            )
        }
    }
}

pub fn row_height(zoom_y: f32) -> f32 {
    ((WHITE_KEY_HEIGHT * WHITE_KEYS_PER_OCTAVE as f32 / NOTES_PER_OCTAVE as f32) * zoom_y).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::canvas::Program;
    use iced::{Point, Rectangle, Size, event, mouse};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum TestMessage {
        Pressed(u8, u8),
        Released(u8),
    }

    fn action_message(action: CanvasAction<TestMessage>) -> (Option<TestMessage>, event::Status) {
        let (message, _redraw, status) = action.into_inner();
        (message, status)
    }

    #[test]
    fn octave_keyboard_update_publishes_pressed_and_released_notes() {
        let keyboard = OctaveKeyboard::new(
            4,
            HashMap::new(),
            TestMessage::Pressed,
            TestMessage::Released,
        )
        .with_orientation(Orientation::Degree270);
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(20.0, 70.0));
        let press_cursor = mouse::Cursor::Available(Point::new(15.0, 65.0));
        let release_cursor = mouse::Cursor::Available(Point::new(15.0, 65.0));
        let mut state = OctaveKeyboardState::default();

        let press = keyboard
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds,
                press_cursor,
            )
            .expect("press action");
        let (message, status) = action_message(press);
        assert_eq!(message, Some(TestMessage::Pressed(48, 96)));
        assert_eq!(status, event::Status::Captured);
        assert_eq!(state.active_note_class, Some(0));
        assert!(state.pressed_notes.contains(&0));

        let release = keyboard
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                bounds,
                release_cursor,
            )
            .expect("release action");
        let (message, status) = action_message(release);
        assert_eq!(message, Some(TestMessage::Released(48)));
        assert_eq!(status, event::Status::Ignored);
        assert!(state.pressed_notes.is_empty());
    }

    #[test]
    fn partial_octave_keyboard_maps_top_note_to_midi_127() {
        let keyboard = OctaveKeyboard::new(
            10,
            HashMap::new(),
            TestMessage::Pressed,
            TestMessage::Released,
        )
        .with_orientation(Orientation::Degree270);
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(20.0, 80.0));
        let cursor = mouse::Cursor::Available(Point::new(15.0, 5.0));
        let mut state = OctaveKeyboardState::default();

        let press = keyboard
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds,
                cursor,
            )
            .expect("press action");
        let (message, status) = action_message(press);

        assert_eq!(message, Some(TestMessage::Pressed(127, 96)));
        assert_eq!(status, event::Status::Captured);
        assert_eq!(state.active_note_class, Some(7));
    }
}

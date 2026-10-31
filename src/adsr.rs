//! Interactive attack, decay, sustain, and release envelope editor.
use iced::{
    Color, Element, Event, Length, Point, Rectangle, Renderer, Theme, mouse,
    widget::{
        canvas,
        canvas::{Action, Frame, Geometry, Path, Program, Stroke, Text},
        column, container, mouse_area, row, text, text_input,
    },
};

/// Edits emitted by [`adsr_envelope`]. Stage indices follow attack, decay,
/// sustain, release order.
#[derive(Debug, Clone, PartialEq)]
pub enum AdsrMessage {
    Change(usize, f32),
    Release(usize),
    Reset(usize),
    Assign(usize),
    Edit(usize),
    EditChanged(usize, String),
    EditSubmit(usize),
}

/// Draw an interactive ADSR envelope.
///
/// `ranges` and `steps` use attack, decay, sustain, release order.
/// Time stages are spaced logarithmically. Right-clicking a handle resets its
/// stage; stage labels emit `Assign` for hosts that support modulation routing.
pub fn adsr_envelope<'a>(
    values: [f32; 4],
    ranges: [(f32, f32); 4],
    steps: [f32; 4],
    assigned: [bool; 4],
) -> Element<'a, AdsrMessage> {
    adsr_envelope_with_edit(
        values,
        ranges,
        steps,
        assigned,
        None,
        "",
        iced::widget::Id::unique(),
    )
}

/// Draw an envelope with an optional editable stage value.
pub fn adsr_envelope_with_edit<'a>(
    values: [f32; 4],
    ranges: [(f32, f32); 4],
    steps: [f32; 4],
    assigned: [bool; 4],
    editing: Option<usize>,
    edit_value: &'a str,
    input_id: iced::widget::Id,
) -> Element<'a, AdsrMessage> {
    let graph = Envelope {
        values,
        ranges,
        steps,
        assigned,
    };
    let labels = row((0..4).map(|i| {
        let value = if i == 2 {
            format!("{:.0}%", values[i] * 100.0)
        } else {
            format!("{:.2}s", values[i])
        };
        let label: Element<'a, AdsrMessage> = if editing == Some(i) {
            text_input::<AdsrMessage, iced::Theme, iced::Renderer>("", edit_value)
                .id(input_id.clone())
                .size(10)
                .on_input(move |value| AdsrMessage::EditChanged(i, value))
                .on_submit(AdsrMessage::EditSubmit(i))
                .width(Length::Fill)
                .into()
        } else {
            let content = container(column![
                text(["Attack", "Decay", "Sustain", "Release"][i]).size(10),
                text(value).size(10)
            ])
            .padding(2)
            .width(Length::Fill);
            mouse_area(content)
                .on_press(AdsrMessage::Assign(i))
                .on_double_click(AdsrMessage::Edit(i))
                .into()
        };
        label
    }))
    .width(210);
    column![canvas(graph).width(210).height(96), labels]
        .spacing(3)
        .into()
}

struct Envelope {
    values: [f32; 4],
    ranges: [(f32, f32); 4],
    steps: [f32; 4],
    assigned: [bool; 4],
}

#[derive(Default)]
struct DragState {
    drag: Option<(usize, Point, f32)>,
}

impl Envelope {
    fn time_position(value: f32, range: (f32, f32)) -> f32 {
        let norm = ((value - range.0) / (range.1 - range.0).max(f32::EPSILON)).clamp(0.0, 1.0);
        (1.0 + norm * 100.0).ln() / 101.0_f32.ln()
    }
    fn time_value(position: f32, range: (f32, f32)) -> f32 {
        let norm = (101.0_f32.powf(position.clamp(0.0, 1.0)) - 1.0) / 100.0;
        range.0 + norm * (range.1 - range.0)
    }
    fn points(&self) -> [Point; 5] {
        let attack = 12.0 + 28.0 * Self::time_position(self.values[0], self.ranges[0]);
        let decay = attack + 12.0 + 28.0 * Self::time_position(self.values[1], self.ranges[1]);
        let sustain_range = self.ranges[2];
        let sustain_norm = ((self.values[2] - sustain_range.0)
            / (sustain_range.1 - sustain_range.0).max(f32::EPSILON))
        .clamp(0.0, 1.0);
        let sustain = 94.0 - sustain_norm * 72.0;
        [
            Point::new(12.0, 94.0),
            Point::new(attack, 22.0),
            Point::new(decay, sustain),
            Point::new(decay + 36.0, sustain),
            Point::new(
                decay + 48.0 + 28.0 * Self::time_position(self.values[3], self.ranges[3]),
                94.0,
            ),
        ]
    }
    fn hit(&self, point: Point) -> Option<usize> {
        self.points()[1..]
            .iter()
            .position(|p| p.distance(point) <= 9.0)
    }
}

impl Program<AdsrMessage> for Envelope {
    type State = DragState;
    fn update(
        &self,
        state: &mut DragState,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<AdsrMessage>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let point = cursor.position_in(bounds)?;
                let i = self.hit(point)?;
                state.drag = Some((i, point, self.values[i]));
                Some(Action::capture())
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let i = self.hit(cursor.position_in(bounds)?)?;
                Some(Action::publish(AdsrMessage::Reset(i)).and_capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let (i, start, initial) = state.drag?;
                let point = Point::new(position.x - bounds.x, position.y - bounds.y);
                let range = self.ranges[i];
                let value = if i == 2 {
                    let norm = (initial - range.0) / (range.1 - range.0).max(f32::EPSILON)
                        - (point.y - start.y) / 72.0;
                    range.0 + norm * (range.1 - range.0)
                } else {
                    Self::time_value(
                        Self::time_position(initial, range) + (point.x - start.x) / 28.0,
                        range,
                    )
                };
                let step = self.steps[i].max(f32::EPSILON);
                let value = ((value / step).round() * step).clamp(range.0, range.1);
                Some(Action::publish(AdsrMessage::Change(i, value)).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Window(iced::window::Event::Unfocused) => {
                let (i, _, _) = state.drag.take()?;
                Some(Action::publish(AdsrMessage::Release(i)).and_capture())
            }
            _ => None,
        }
    }
    fn draw(
        &self,
        _state: &DragState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = theme.extended_palette();
        let ink = palette.background.base.text;
        let accent = palette.primary.base.color;
        let points = self.points();
        let shape = Path::new(|p| {
            p.move_to(points[0]);
            for point in &points[1..] {
                p.line_to(*point);
            }
            p.close();
        });
        frame.fill(&shape, Color { a: 0.18, ..accent });
        let curve = Path::new(|p| {
            p.move_to(points[0]);
            for point in &points[1..] {
                p.line_to(*point);
            }
        });
        frame.stroke(&curve, Stroke::default().with_color(accent).with_width(2.0));
        frame.stroke(
            &Path::new(|p| {
                p.move_to(Point::new(12.0, 16.0));
                p.line_to(points[0]);
                p.line_to(Point::new(205.0, 94.0));
            }),
            Stroke::default().with_color(ink),
        );
        for (i, point) in points[1..].iter().enumerate() {
            let mut y = point.y + 7.0;
            while y < 101.0 {
                frame.stroke(
                    &Path::line(
                        Point::new(point.x, y),
                        Point::new(point.x, (y + 2.0).min(101.0)),
                    ),
                    Stroke::default().with_color(Color { a: 0.4, ..ink }),
                );
                y += 5.0;
            }
            frame.fill(
                &Path::circle(*point, 4.0),
                if self.assigned[i] {
                    Color::from_rgb(1.0, 0.83, 0.10)
                } else {
                    accent
                },
            );
        }
        for (label, position) in [
            ("level", Point::new(0.0, 0.0)),
            ("time", Point::new(183.0, 78.0)),
        ] {
            frame.fill_text(Text {
                content: label.into(),
                position,
                color: ink,
                size: 10.0.into(),
                ..Text::default()
            });
        }
        vec![frame.into_geometry()]
    }
    fn mouse_interaction(
        &self,
        state: &DragState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.drag.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor
            .position_in(bounds)
            .and_then(|p| self.hit(p))
            .is_some()
        {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

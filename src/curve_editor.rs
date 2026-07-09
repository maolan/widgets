use iced::{
    Color, Event, Point, Rectangle, Renderer, Theme, mouse,
    widget::canvas::{Action as CanvasAction, Frame, Geometry, Path, Program, Stroke},
};

/// A single sample/value point on a normalized curve.
///
/// `value` is expected to be in the `[0, 1]` range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurvePoint {
    pub sample: usize,
    pub value: f32,
}

/// Messages produced by a [`CurveEditor`].
///
/// Implement this trait for your application message type, supplying a context
/// value that identifies the curve being edited (e.g. track name + target).
pub trait CurveEditorMessage: Clone + 'static {
    type Context: Clone;

    /// Called when the user finishes a right-drag.
    ///
    /// `points` contains the interpolated points along the drawn line.
    fn draw_line(context: Self::Context, points: Vec<CurvePoint>) -> Self;

    /// Called when the user right-clicks an existing point.
    fn delete_point(context: Self::Context, sample: usize) -> Self;
}

/// Interactive canvas for drawing and editing a normalized curve.
///
/// Interaction matches the MIDI controller lane: right-drag draws a line of
/// points, right-clicking an existing point deletes it, and a preview line is
/// shown while dragging.
pub struct CurveEditor<M: CurveEditorMessage> {
    pub context: M::Context,
    pub points: Vec<CurvePoint>,
    pub pixels_per_sample: f32,
    pub color: Color,
    pub dot_radius: f32,
    pub line_width: f32,
}

pub struct CurveEditorState {
    mode: CurveEditorDragMode,
}

#[derive(Default, Debug, Clone, Copy)]
enum CurveEditorDragMode {
    #[default]
    None,
    Drawing {
        start: Point,
        current: Point,
    },
}

const DELETE_RADIUS: f32 = 5.0;
const PREVIEW_COLOR: Color = Color::from_rgba(0.98, 0.94, 0.2, 0.95);

impl Default for CurveEditorState {
    fn default() -> Self {
        Self {
            mode: CurveEditorDragMode::None,
        }
    }
}

impl<M: CurveEditorMessage> CurveEditor<M> {
    fn point_index_at_position(&self, position: Point, bounds: Rectangle) -> Option<usize> {
        if self.pixels_per_sample <= f32::EPSILON {
            return None;
        }
        let mut best: Option<(usize, f32)> = None;
        for (idx, point) in self.points.iter().enumerate() {
            let x = point.sample as f32 * self.pixels_per_sample;
            let y = if bounds.height <= f32::EPSILON {
                0.0
            } else {
                bounds.height * (1.0 - point.value.clamp(0.0, 1.0))
            };
            let dx = position.x - x;
            let dy = position.y - y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq > DELETE_RADIUS * DELETE_RADIUS {
                continue;
            }
            match best {
                Some((_, best_dist_sq)) if dist_sq >= best_dist_sq => {}
                _ => best = Some((idx, dist_sq)),
            }
        }
        best.map(|(idx, _)| idx)
    }

    fn build_drawn_points(&self, start: Point, end: Point, bounds: Rectangle) -> Vec<CurvePoint> {
        if self.pixels_per_sample <= f32::EPSILON || bounds.height <= f32::EPSILON {
            return vec![];
        }
        let sample_at =
            |x: f32| -> usize { ((x / self.pixels_per_sample).round().max(0.0)) as usize };
        let value_at = |y: f32| -> f32 { (1.0 - (y / bounds.height)).clamp(0.0, 1.0) };

        let start_sample = sample_at(start.x);
        let end_sample = sample_at(end.x);
        let start_value = value_at(start.y);
        let end_value = value_at(end.y);

        let sample_min = start_sample.min(end_sample);
        let sample_max = start_sample.max(end_sample);
        if sample_min == sample_max {
            return vec![CurvePoint {
                sample: sample_min,
                value: end_value,
            }];
        }

        let step_samples = (1.0 / self.pixels_per_sample).max(1.0).round() as usize;
        let mut out = Vec::new();
        let mut sample = sample_min;
        loop {
            let t = (sample - sample_min) as f32 / (sample_max - sample_min) as f32;
            let value = if start_sample <= end_sample {
                start_value + (end_value - start_value) * t
            } else {
                end_value + (start_value - end_value) * t
            };
            out.push(CurvePoint { sample, value });
            if sample == sample_max {
                break;
            }
            sample = (sample + step_samples).min(sample_max);
        }
        out
    }

    fn point_position(&self, point: CurvePoint, bounds: Rectangle) -> Point {
        let x = point.sample as f32 * self.pixels_per_sample;
        let y = if bounds.height <= f32::EPSILON {
            0.0
        } else {
            bounds.height * (1.0 - point.value.clamp(0.0, 1.0))
        };
        Point::new(x, y)
    }
}

impl<M: CurveEditorMessage> Program<M> for CurveEditor<M> {
    type State = CurveEditorState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<CanvasAction<M>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let position = cursor.position_in(bounds)?;
                if let Some(idx) = self.point_index_at_position(position, bounds) {
                    let sample = self.points[idx].sample;
                    return Some(
                        CanvasAction::publish(M::delete_point(self.context.clone(), sample))
                            .and_capture(),
                    );
                }
                state.mode = CurveEditorDragMode::Drawing {
                    start: position,
                    current: position,
                };
                Some(CanvasAction::request_redraw().and_capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let CurveEditorDragMode::Drawing { start, .. } = state.mode
                    && let Some(position) = cursor.position_in(bounds)
                {
                    state.mode = CurveEditorDragMode::Drawing {
                        start,
                        current: position,
                    };
                    return Some(CanvasAction::request_redraw().and_capture());
                }
                None
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)) => {
                if let CurveEditorDragMode::Drawing { start, current } = state.mode {
                    state.mode = CurveEditorDragMode::None;
                    let points = self.build_drawn_points(start, current, bounds);
                    if points.is_empty() {
                        return Some(CanvasAction::request_redraw().and_capture());
                    }
                    return Some(
                        CanvasAction::publish(M::draw_line(self.context.clone(), points))
                            .and_capture(),
                    );
                }
                None
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        if bounds.width <= f32::EPSILON || bounds.height <= f32::EPSILON {
            return vec![];
        }

        let mut frame = Frame::new(renderer, bounds.size());

        if self.points.len() >= 2 {
            let mut sorted = self.points.clone();
            sorted.sort_unstable_by_key(|point| point.sample);

            let line = Path::new(|path| {
                let first = self.point_position(sorted[0], bounds);
                path.move_to(first);
                for point in sorted.iter().skip(1) {
                    path.line_to(self.point_position(*point, bounds));
                }
            });
            frame.stroke(
                &line,
                Stroke::default()
                    .with_color(self.color)
                    .with_width(self.line_width),
            );
        }

        for point in &self.points {
            let center = self.point_position(*point, bounds);
            let circle = Path::circle(center, self.dot_radius.max(0.0));
            frame.fill(&circle, self.color);
        }

        if let CurveEditorDragMode::Drawing { start, current } = state.mode {
            let line = Path::line(start, current);
            frame.stroke(
                &line,
                Stroke::default()
                    .with_color(PREVIEW_COLOR)
                    .with_width(self.line_width),
            );
        }

        vec![frame.into_geometry()]
    }
}

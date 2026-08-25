use iced::{
    Color, Element, Event, Length, Point, Rectangle, Renderer, Theme, mouse,
    widget::{
        canvas,
        canvas::{Action as CanvasAction, Frame, Geometry, Path, Program, Stroke},
    },
};
use std::time::Instant;

use crate::consts::DOUBLE_CLICK;

pub struct SampleWaveform<'a, Message> {
    channels: Vec<&'a [f32]>,
    peak: f32,
    empty_label: &'static str,
    playhead_ratio: Option<f32>,
    selection_ratio: Option<(f32, f32)>,
    markers: Vec<(usize, String)>,
    on_click: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    on_double_click: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    on_selection_start: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    on_selection_drag: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    on_selection_finish: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    on_right_click: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    on_middle_click: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    on_middle_click_away: Option<Box<dyn Fn(f32) -> Message + 'a>>,
}

impl<'a, Message> SampleWaveform<'a, Message> {
    pub fn new(channels: impl IntoIterator<Item = &'a [f32]>, peak: f32) -> Self {
        Self {
            channels: channels.into_iter().collect(),
            peak,
            empty_label: "No sample loaded",
            playhead_ratio: None,
            selection_ratio: None,
            markers: Vec::new(),
            on_click: None,
            on_double_click: None,
            on_selection_start: None,
            on_selection_drag: None,
            on_selection_finish: None,
            on_right_click: None,
            on_middle_click: None,
            on_middle_click_away: None,
        }
    }

    pub fn playhead_ratio(mut self, playhead_ratio: Option<f32>) -> Self {
        self.playhead_ratio = playhead_ratio.map(|ratio| ratio.clamp(0.0, 1.0));
        self
    }

    pub fn selection_ratio(mut self, selection_ratio: Option<(f32, f32)>) -> Self {
        self.selection_ratio = selection_ratio.map(|(start, end)| {
            let start = start.clamp(0.0, 1.0);
            let end = end.clamp(0.0, 1.0);
            (start.min(end), start.max(end))
        });
        self
    }

    pub fn on_selection_start(mut self, on_selection_start: impl Fn(f32) -> Message + 'a) -> Self {
        self.on_selection_start = Some(Box::new(on_selection_start));
        self
    }

    pub fn on_selection_drag(mut self, on_selection_drag: impl Fn(f32) -> Message + 'a) -> Self {
        self.on_selection_drag = Some(Box::new(on_selection_drag));
        self
    }

    pub fn on_selection_finish(
        mut self,
        on_selection_finish: impl Fn(f32) -> Message + 'a,
    ) -> Self {
        self.on_selection_finish = Some(Box::new(on_selection_finish));
        self
    }

    pub fn markers(mut self, markers: impl IntoIterator<Item = (usize, String)>) -> Self {
        self.markers = markers.into_iter().collect();
        self.markers.sort_unstable_by_key(|(sample, _)| *sample);
        self
    }

    pub fn on_click(mut self, on_click: impl Fn(f32) -> Message + 'a) -> Self {
        self.on_click = Some(Box::new(on_click));
        self
    }

    pub fn on_double_click(mut self, on_double_click: impl Fn(f32) -> Message + 'a) -> Self {
        self.on_double_click = Some(Box::new(on_double_click));
        self
    }

    pub fn on_right_click(mut self, on_right_click: impl Fn(f32) -> Message + 'a) -> Self {
        self.on_right_click = Some(Box::new(on_right_click));
        self
    }

    pub fn on_middle_click(mut self, on_middle_click: impl Fn(f32) -> Message + 'a) -> Self {
        self.on_middle_click = Some(Box::new(on_middle_click));
        self
    }

    pub fn on_middle_click_away(
        mut self,
        on_middle_click_away: impl Fn(f32) -> Message + 'a,
    ) -> Self {
        self.on_middle_click_away = Some(Box::new(on_middle_click_away));
        self
    }

    pub fn empty_label(mut self, label: &'static str) -> Self {
        self.empty_label = label;
        self
    }

    pub fn view(self) -> Element<'a, Message>
    where
        Message: 'a,
    {
        canvas(self).width(Length::Fill).height(Length::Fill).into()
    }

    fn frame_count(&self) -> usize {
        self.channels
            .iter()
            .map(|channel| channel.len())
            .min()
            .unwrap_or(0)
    }

    fn near_marker(&self, ratio: f32, width: f32) -> bool {
        if self.markers.is_empty() {
            return false;
        }
        let frames = self.frame_count();
        if frames == 0 {
            return false;
        }
        let x = width * ratio;
        self.markers
            .iter()
            .map(|(sample, _)| width * (*sample as f32 / frames as f32))
            .map(|marker_x| (marker_x - x).abs())
            .any(|distance| distance <= 8.0)
    }
}

#[derive(Debug, Default)]
pub struct SampleWaveformState {
    selecting: bool,
    drag_start: Option<Point>,
    last_click_at: Option<Instant>,
}

impl<Message> Program<Message> for SampleWaveform<'_, Message> {
    type State = SampleWaveformState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<CanvasAction<Message>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let ratio = cursor_ratio(cursor, bounds)?;
                let position = cursor.position_in(bounds)?;
                let now = Instant::now();
                let is_double_click = state
                    .last_click_at
                    .is_some_and(|last| now.duration_since(last) <= DOUBLE_CLICK);
                state.last_click_at = Some(now);

                if is_double_click {
                    state.drag_start = None;
                    state.selecting = false;
                    return self
                        .on_double_click
                        .as_ref()
                        .map(|message| CanvasAction::publish(message(ratio)).and_capture());
                }

                state.drag_start = Some(position);
                state.selecting = false;
                None
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let position = cursor.position_in(bounds)?;
                if let Some(drag_start) = state.drag_start
                    && !state.selecting
                    && (position.x - drag_start.x).abs() > 2.0
                {
                    state.selecting = true;
                    let ratio = cursor_ratio(cursor, bounds)?;
                    return self
                        .on_selection_start
                        .as_ref()
                        .map(|message| CanvasAction::publish(message(ratio)).and_capture());
                }
                if state.selecting {
                    let ratio = cursor_ratio(cursor, bounds)?;
                    return self
                        .on_selection_drag
                        .as_ref()
                        .map(|message| CanvasAction::publish(message(ratio)).and_capture());
                }
                None
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let ratio = cursor_ratio(cursor, bounds)?;
                if state.selecting {
                    state.selecting = false;
                    state.drag_start = None;
                    return self
                        .on_selection_finish
                        .as_ref()
                        .map(|message| CanvasAction::publish(message(ratio)).and_capture());
                }
                if state.drag_start.take().is_some() {
                    return self
                        .on_click
                        .as_ref()
                        .map(|message| CanvasAction::publish(message(ratio)).and_capture());
                }
                None
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let ratio = cursor_ratio(cursor, bounds)?;
                self.on_right_click
                    .as_ref()
                    .map(|message| CanvasAction::publish(message(ratio)).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                let ratio = cursor_ratio(cursor, bounds)?;
                if self.near_marker(ratio, bounds.width) {
                    self.on_middle_click
                        .as_ref()
                        .map(|message| CanvasAction::publish(message(ratio)).and_capture())
                } else {
                    self.on_middle_click_away
                        .as_ref()
                        .map(|message| CanvasAction::publish(message(ratio)).and_capture())
                }
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let background = Path::rectangle(Point::ORIGIN, bounds.size());
        frame.fill(&background, Color::from_rgb(0.075, 0.078, 0.095));

        let frames = self.frame_count();
        if frames == 0 {
            draw_empty_label(&mut frame, self.empty_label);
            return vec![frame.into_geometry()];
        }

        draw_waveform(&mut frame, bounds, &self.channels, frames, self.peak);
        if let Some(selection) = self.selection_ratio {
            draw_selection(&mut frame, bounds, selection);
        }
        if let Some(ratio) = self.playhead_ratio {
            draw_playhead(&mut frame, bounds, ratio);
        }
        draw_markers(&mut frame, bounds, frames, &self.markers);

        vec![frame.into_geometry()]
    }
}

fn cursor_ratio(cursor: mouse::Cursor, bounds: Rectangle) -> Option<f32> {
    let position = cursor.position_in(bounds)?;
    Some((position.x / bounds.width.max(1.0)).clamp(0.0, 1.0))
}

fn draw_empty_label(frame: &mut Frame, label: &'static str) {
    frame.fill_text(canvas::Text {
        content: String::from(label),
        position: Point::new(8.0, 18.0),
        color: Color::from_rgb(0.50, 0.52, 0.58),
        size: iced::Pixels(12.0),
        ..canvas::Text::default()
    });
}

fn draw_waveform(
    frame: &mut Frame,
    bounds: Rectangle,
    channels: &[&[f32]],
    frames: usize,
    peak: f32,
) {
    let width = bounds.width.max(1.0) as usize;
    let height = bounds.height.max(1.0);
    let channel_count = channels.len().max(1);
    let lane_height = height / channel_count as f32;
    let peak = peak.max(1e-10);
    let samples_per_pixel = frames as f32 / width as f32;

    for (channel_index, channel) in channels.iter().enumerate() {
        let top = channel_index as f32 * lane_height;
        let center = top + lane_height / 2.0;
        let scale = (lane_height / 2.0 - 2.0).max(1.0);
        let mut top_points = Vec::with_capacity(width);
        let mut bottom_points = Vec::with_capacity(width);
        for x in 0..width {
            let start = ((x as f32 * samples_per_pixel) as usize).min(frames);
            let end = (((x + 1) as f32 * samples_per_pixel) as usize).min(frames);

            let mut min_sample = 0.0f32;
            let mut max_sample = 0.0f32;
            if start < end {
                for &sample in &channel[start..end] {
                    min_sample = min_sample.min(sample);
                    max_sample = max_sample.max(sample);
                }
            }

            let y_top = (center - (max_sample / peak) * scale).clamp(top, top + lane_height);
            let y_bottom = (center - (min_sample / peak) * scale).clamp(top, top + lane_height);
            top_points.push(Point::new(x as f32, y_top));
            bottom_points.push(Point::new(x as f32, y_bottom));
        }

        let waveform_path = Path::new(|builder| {
            if let Some(first) = top_points.first() {
                builder.move_to(*first);
                for point in &top_points[1..] {
                    builder.line_to(*point);
                }
                for point in bottom_points.iter().rev() {
                    builder.line_to(*point);
                }
                builder.close();
            }
        });

        frame.fill(&waveform_path, Color::from_rgba(0.35, 0.55, 0.85, 0.35));
        frame.stroke(
            &waveform_path,
            Stroke::default()
                .with_color(Color::from_rgb(0.45, 0.72, 1.0))
                .with_width(1.0),
        );

        if channel_index > 0 {
            let separator = Path::line(Point::new(0.0, top), Point::new(width as f32, top));
            frame.stroke(
                &separator,
                Stroke::default()
                    .with_color(Color::from_rgb(0.12, 0.13, 0.16))
                    .with_width(1.0),
            );
        }
    }
}

fn draw_playhead(frame: &mut Frame, bounds: Rectangle, ratio: f32) {
    let x = (bounds.width * ratio.clamp(0.0, 1.0)).clamp(0.0, bounds.width);
    let path = Path::line(Point::new(x, 0.0), Point::new(x, bounds.height));
    frame.stroke(
        &path,
        Stroke::default()
            .with_color(Color::from_rgb(1.0, 0.82, 0.25))
            .with_width(2.0),
    );
}

fn draw_selection(frame: &mut Frame, bounds: Rectangle, selection: (f32, f32)) {
    let x0 = bounds.width * selection.0.clamp(0.0, 1.0);
    let x1 = bounds.width * selection.1.clamp(0.0, 1.0);
    let width = (x1 - x0).abs().max(1.0);
    let path = Path::rectangle(
        Point::new(x0.min(x1), 0.0),
        iced::Size::new(width, bounds.height),
    );
    frame.fill(&path, Color::from_rgba(1.0, 0.82, 0.25, 0.22));
    frame.stroke(
        &path,
        Stroke::default()
            .with_color(Color::from_rgba(1.0, 0.82, 0.25, 0.70))
            .with_width(1.0),
    );
}

fn draw_markers(frame: &mut Frame, bounds: Rectangle, frames: usize, markers: &[(usize, String)]) {
    if frames == 0 || markers.is_empty() {
        return;
    }
    let marker_color = Color::from_rgba(0.96, 0.72, 0.18, 0.95);
    let marker_border = Color::from_rgba(0.2, 0.16, 0.04, 0.95);
    let label_bg = Color::from_rgba(0.28, 0.20, 0.06, 0.92);
    let label_border = Color::from_rgba(0.78, 0.62, 0.18, 0.85);
    let text_color = Color::from_rgba(0.98, 0.92, 0.72, 0.96);

    for (sample, name) in markers {
        let ratio = (*sample as f32 / frames as f32).clamp(0.0, 1.0);
        let x = bounds.width * ratio;

        frame.stroke(
            &Path::line(Point::new(x, 3.0), Point::new(x, bounds.height - 3.0)),
            Stroke::default().with_width(2.0).with_color(marker_color),
        );

        let handle_size = 6.0;
        frame.fill(
            &Path::rectangle(
                Point::new(x - handle_size / 2.0, 0.0),
                iced::Size::new(handle_size, handle_size),
            ),
            marker_color,
        );
        frame.stroke(
            &Path::rectangle(
                Point::new(x - handle_size / 2.0, 0.0),
                iced::Size::new(handle_size, handle_size),
            ),
            Stroke::default().with_width(1.0).with_color(marker_border),
        );

        let trimmed = name.trim();
        if !trimmed.is_empty() {
            let text_x = x + 5.0;
            let text_y = 1.0;
            let approx_width = trimmed.len() as f32 * 5.5 + 8.0;
            frame.fill(
                &Path::rectangle(
                    Point::new(text_x, text_y),
                    iced::Size::new(approx_width, 12.0),
                ),
                label_bg,
            );
            frame.stroke(
                &Path::rectangle(
                    Point::new(text_x, text_y),
                    iced::Size::new(approx_width, 12.0),
                ),
                Stroke::default().with_width(1.0).with_color(label_border),
            );
            frame.fill_text(canvas::Text {
                content: trimmed.to_string(),
                position: Point::new(text_x + 3.0, text_y + 1.0),
                color: text_color,
                size: iced::Pixels(9.0),
                ..Default::default()
            });
        }
    }
}

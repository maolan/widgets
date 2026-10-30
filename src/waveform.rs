use iced::{
    Color, Element, Event, Length, Point, Rectangle, Renderer, Theme, keyboard, mouse,
    widget::{
        canvas,
        canvas::{Action as CanvasAction, Frame, Geometry, Path, Program, Stroke},
    },
};
use std::time::Instant;

use crate::consts::DOUBLE_CLICK;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionEdge {
    Start,
    End,
}

type RatioMessageFn<'a, Message> = Box<dyn Fn(f32) -> Message + 'a>;
type RatioPairMessageFn<'a, Message> = Box<dyn Fn((f32, f32)) -> Message + 'a>;
type ScrollMessageFn<'a, Message> = Box<dyn Fn(f32, f32, f32) -> Message + 'a>;

pub struct SampleWaveform<'a, Message> {
    channels: Vec<&'a [f32]>,
    peak: f32,
    empty_label: &'static str,
    visible_range: (f32, f32),
    playhead_ratio: Option<f32>,
    selection_ratio: Option<(f32, f32)>,
    xfade_ratio: Option<(f32, f32)>,
    markers: Vec<(usize, String)>,
    on_click: Option<RatioMessageFn<'a, Message>>,
    on_double_click: Option<RatioMessageFn<'a, Message>>,
    on_scroll: Option<ScrollMessageFn<'a, Message>>,
    on_selection_start: Option<RatioMessageFn<'a, Message>>,
    on_selection_drag: Option<RatioMessageFn<'a, Message>>,
    on_selection_finish: Option<RatioMessageFn<'a, Message>>,
    on_xfade: Option<RatioPairMessageFn<'a, Message>>,
    on_right_click: Option<RatioMessageFn<'a, Message>>,
    on_middle_click: Option<RatioMessageFn<'a, Message>>,
    on_middle_click_away: Option<RatioMessageFn<'a, Message>>,
}

impl<'a, Message> SampleWaveform<'a, Message> {
    pub fn new(channels: impl IntoIterator<Item = &'a [f32]>, peak: f32) -> Self {
        Self {
            channels: channels.into_iter().collect(),
            peak,
            empty_label: "No sample loaded",
            visible_range: (0.0, 1.0),
            playhead_ratio: None,
            selection_ratio: None,
            xfade_ratio: None,
            markers: Vec::new(),
            on_click: None,
            on_double_click: None,
            on_scroll: None,
            on_selection_start: None,
            on_selection_drag: None,
            on_selection_finish: None,
            on_xfade: None,
            on_right_click: None,
            on_middle_click: None,
            on_middle_click_away: None,
        }
    }

    pub fn visible_range(mut self, visible_range: (f32, f32)) -> Self {
        let start = visible_range.0.clamp(0.0, 1.0);
        let end = visible_range.1.clamp(0.0, 1.0);
        self.visible_range = if start < end {
            (start, end)
        } else {
            (0.0, 1.0)
        };
        self
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

    pub fn xfade_ratio(mut self, xfade_ratio: Option<(f32, f32)>) -> Self {
        self.xfade_ratio = xfade_ratio.map(|(start, end)| {
            let start = start.clamp(0.0, 1.0);
            let end = end.clamp(0.0, 1.0);
            (start.min(end), start.max(end))
        });
        self
    }

    pub fn on_xfade(mut self, on_xfade: impl Fn((f32, f32)) -> Message + 'a) -> Self {
        self.on_xfade = Some(Box::new(on_xfade));
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

    pub fn on_scroll(mut self, on_scroll: impl Fn(f32, f32, f32) -> Message + 'a) -> Self {
        self.on_scroll = Some(Box::new(on_scroll));
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

    fn visible_ratio(&self, ratio: f32) -> Option<f32> {
        let (start, end) = self.visible_range;
        if (start..=end).contains(&ratio) {
            Some((ratio - start) / (end - start))
        } else {
            None
        }
    }

    fn visible_region(&self, region: (f32, f32)) -> Option<(f32, f32)> {
        let (view_start, view_end) = self.visible_range;
        let start = region.0.max(view_start);
        let end = region.1.min(view_end);
        if start <= end {
            Some((
                (start - view_start) / (view_end - view_start),
                (end - view_start) / (view_end - view_start),
            ))
        } else {
            None
        }
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
            .filter_map(|(sample, _)| self.visible_ratio(*sample as f32 / frames as f32))
            .map(|marker_ratio| (width * marker_ratio - x).abs())
            .any(|distance| distance <= 8.0)
    }

    fn selection_edge_at(&self, ratio: f32, width: f32) -> Option<SelectionEdge> {
        let (start, end) = self.visible_region(self.selection_ratio?)?;
        let x = width * ratio;
        let start_distance = (width * start - x).abs();
        let end_distance = (width * end - x).abs();
        const EDGE_TOLERANCE: f32 = 6.0;
        match (
            start_distance <= EDGE_TOLERANCE,
            end_distance <= EDGE_TOLERANCE,
        ) {
            (true, true) => {
                if start_distance <= end_distance {
                    Some(SelectionEdge::Start)
                } else {
                    Some(SelectionEdge::End)
                }
            }
            (true, false) => Some(SelectionEdge::Start),
            (false, true) => Some(SelectionEdge::End),
            (false, false) => None,
        }
    }

    fn publish_xfade(&self, edge: SelectionEdge, ratio: f32) -> Option<CanvasAction<Message>> {
        let selection = self.visible_region(self.selection_ratio?)?;
        let region = xfade_inner_region(selection, edge, ratio);
        self.on_xfade
            .as_ref()
            .map(|message| CanvasAction::publish(message(region)).and_capture())
    }
}

/// Computes the symmetric inner ("xfade") region for a ctrl+drag starting at
/// one edge of the selection: the distance from the selection start to the
/// inner start always equals the distance from the inner end to the
/// selection end.
fn xfade_inner_region(selection: (f32, f32), edge: SelectionEdge, ratio: f32) -> (f32, f32) {
    let (sel_start, sel_end) = selection;
    let (inner_start, inner_end) = match edge {
        SelectionEdge::Start => (ratio, sel_end - (ratio - sel_start)),
        SelectionEdge::End => (sel_start + (sel_end - ratio), ratio),
    };
    let inner_start = inner_start.clamp(sel_start, sel_end);
    let inner_end = inner_end.clamp(sel_start, sel_end);
    if inner_start > inner_end {
        let mid = (sel_start + sel_end) / 2.0;
        (mid, mid)
    } else {
        (inner_start, inner_end)
    }
}

#[derive(Debug, Default)]
pub struct SampleWaveformState {
    selecting: bool,
    drag_start: Option<Point>,
    last_click_at: Option<Instant>,
    modifiers: keyboard::Modifiers,
    xfade_edge: Option<SelectionEdge>,
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
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
                None
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let ratio = cursor_ratio(cursor, bounds)?;
                let delta_y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 16.0,
                };
                self.on_scroll.as_ref().map(|message| {
                    CanvasAction::publish(message(delta_y, ratio, bounds.width)).and_capture()
                })
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cfg!(target_os = "macos") && state.modifiers.alt() =>
            {
                state.drag_start = None;
                state.selecting = false;
                state.xfade_edge = None;
                state.last_click_at = None;
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
                    state.xfade_edge = None;
                    return self
                        .on_double_click
                        .as_ref()
                        .map(|message| CanvasAction::publish(message(ratio)).and_capture());
                }

                if state.modifiers.command()
                    && let Some(edge) = self.selection_edge_at(ratio, bounds.width)
                {
                    state.drag_start = None;
                    state.selecting = false;
                    state.xfade_edge = Some(edge);
                    return self.publish_xfade(edge, ratio);
                }

                state.drag_start = Some(position);
                state.selecting = false;
                state.xfade_edge = None;
                None
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let position = cursor.position_in(bounds)?;
                if let Some(edge) = state.xfade_edge {
                    let ratio = cursor_ratio(cursor, bounds)?;
                    return self.publish_xfade(edge, ratio);
                }
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
                if let Some(edge) = state.xfade_edge.take() {
                    return self.publish_xfade(edge, ratio);
                }
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

        draw_waveform(
            &mut frame,
            bounds,
            &self.channels,
            frames,
            self.peak,
            self.visible_range,
        );
        if let Some(selection) = self
            .selection_ratio
            .and_then(|selection| self.visible_region(selection))
        {
            draw_selection(&mut frame, bounds, selection);
        }
        if let Some(xfade) = self
            .xfade_ratio
            .and_then(|xfade| self.visible_region(xfade))
        {
            draw_xfade(&mut frame, bounds, xfade);
        }
        if let Some(ratio) = self
            .playhead_ratio
            .and_then(|ratio| self.visible_ratio(ratio))
        {
            draw_playhead(&mut frame, bounds, ratio);
        }
        draw_markers(
            &mut frame,
            bounds,
            frames,
            &self.markers,
            self.visible_range,
        );

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
    visible_range: (f32, f32),
) {
    let width = bounds.width.max(1.0) as usize;
    let height = bounds.height.max(1.0);
    let channel_count = channels.len().max(1);
    let lane_height = height / channel_count as f32;
    let peak = peak.max(1e-10);
    let visible_start = ((frames as f32 * visible_range.0) as usize).min(frames - 1);
    let visible_end = ((frames as f32 * visible_range.1).ceil() as usize)
        .min(frames)
        .max(visible_start + 1);
    let visible_frames = visible_end - visible_start;
    let samples_per_pixel = visible_frames as f32 / width as f32;

    for (channel_index, channel) in channels.iter().enumerate() {
        let top = channel_index as f32 * lane_height;
        let center = top + lane_height / 2.0;
        let scale = (lane_height / 2.0 - 2.0).max(1.0);
        let mut top_points = Vec::with_capacity(width);
        let mut bottom_points = Vec::with_capacity(width);
        for x in 0..width {
            let start =
                (visible_start + (x as f32 * samples_per_pixel) as usize).min(visible_end - 1);
            let end = (visible_start + ((x + 1) as f32 * samples_per_pixel) as usize)
                .min(visible_end)
                .max(start + 1);

            let mut min_sample = 0.0f32;
            let mut max_sample = 0.0f32;
            for &sample in &channel[start..end] {
                min_sample = min_sample.min(sample);
                max_sample = max_sample.max(sample);
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

fn draw_xfade(frame: &mut Frame, bounds: Rectangle, region: (f32, f32)) {
    let x0 = bounds.width * region.0.clamp(0.0, 1.0);
    let x1 = bounds.width * region.1.clamp(0.0, 1.0);
    if x1 - x0 < 1.0 {
        return;
    }
    let path = Path::rectangle(Point::new(x0, 0.0), iced::Size::new(x1 - x0, bounds.height));
    frame.fill(&path, Color::from_rgba(0.35, 0.85, 0.55, 0.16));
    frame.stroke(
        &path,
        Stroke::default()
            .with_color(Color::from_rgba(0.35, 0.85, 0.55, 0.65))
            .with_width(1.0),
    );
}

fn draw_markers(
    frame: &mut Frame,
    bounds: Rectangle,
    frames: usize,
    markers: &[(usize, String)],
    visible_range: (f32, f32),
) {
    if frames == 0 || markers.is_empty() {
        return;
    }
    let marker_color = Color::from_rgba(0.96, 0.72, 0.18, 0.95);
    let marker_border = Color::from_rgba(0.2, 0.16, 0.04, 0.95);
    let label_bg = Color::from_rgba(0.28, 0.20, 0.06, 0.92);
    let label_border = Color::from_rgba(0.78, 0.62, 0.18, 0.85);
    let text_color = Color::from_rgba(0.98, 0.92, 0.72, 0.96);

    for (sample, name) in markers {
        let global_ratio = (*sample as f32 / frames as f32).clamp(0.0, 1.0);
        if !(visible_range.0..=visible_range.1).contains(&global_ratio) {
            continue;
        }
        let ratio = (global_ratio - visible_range.0) / (visible_range.1 - visible_range.0);
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

#[cfg(test)]
mod tests {
    use super::{SampleWaveform, SelectionEdge, xfade_inner_region};

    #[test]
    fn visible_range_converts_global_ratios() {
        let waveform = SampleWaveform::<()>::new(std::iter::empty::<&[f32]>(), 0.0)
            .visible_range((0.25, 0.75));
        assert_eq!(waveform.visible_ratio(0.5), Some(0.5));
        assert_eq!(waveform.visible_ratio(0.75), Some(1.0));
        assert_eq!(waveform.visible_ratio(0.1), None);
        assert_eq!(waveform.visible_region((0.0, 1.0)), Some((0.0, 1.0)));
        assert_eq!(waveform.visible_region((0.0, 0.5)), Some((0.0, 0.5)));
        assert_eq!(waveform.visible_region((0.8, 1.0)), None);
    }

    #[test]
    fn xfade_inner_region_from_start_edge_is_symmetric() {
        let (start, end) = xfade_inner_region((0.2, 0.8), SelectionEdge::Start, 0.3);
        assert!((start - 0.3).abs() < 1e-6);
        assert!((end - 0.7).abs() < 1e-6);
    }

    #[test]
    fn xfade_inner_region_from_end_edge_is_symmetric() {
        let (start, end) = xfade_inner_region((0.2, 0.8), SelectionEdge::End, 0.7);
        assert!((start - 0.3).abs() < 1e-6);
        assert!((end - 0.7).abs() < 1e-6);
    }

    #[test]
    fn xfade_inner_region_clamps_inside_selection() {
        let (start, end) = xfade_inner_region((0.2, 0.8), SelectionEdge::Start, 0.05);
        assert!((start - 0.2).abs() < 1e-6);
        assert!((end - 0.8).abs() < 1e-6);
        let (start, end) = xfade_inner_region((0.2, 0.8), SelectionEdge::End, 0.95);
        assert!((start - 0.2).abs() < 1e-6);
        assert!((end - 0.8).abs() < 1e-6);
    }

    #[test]
    fn xfade_inner_region_collapses_to_middle_when_edges_cross() {
        let (start, end) = xfade_inner_region((0.2, 0.8), SelectionEdge::Start, 0.6);
        assert!((start - 0.5).abs() < 1e-6);
        assert!((end - 0.5).abs() < 1e-6);
    }
}

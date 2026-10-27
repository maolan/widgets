use crate::consts::DOUBLE_CLICK;
use iced::advanced::Shell;
use iced::advanced::graphics::geometry;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Tree, Widget};
use iced::widget::canvas::path::Arc as PathArc;
use iced::widget::canvas::{Frame, Path, Stroke};
use iced::{Color, Element, Event, Length, Point, Radians, Rectangle, Size, Vector};
use iced::{keyboard, mouse};
use std::cell::Cell;
use std::f32::consts::PI;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

const START_ANGLE: f32 = 3.0 * PI / 4.0;
const SWEEP_ANGLE: f32 = 3.0 * PI / 2.0;
const END_ANGLE: f32 = START_ANGLE + SWEEP_ANGLE;
static NEXT_OWNER_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static ACTIVE_DRAG_OWNER: Cell<Option<u64>> = const { Cell::new(None) };
}

/// A knob-style arch slider with two handles editing a `low..=high` value
/// pair. Dragging moves the high handle, Shift+dragging moves the low handle,
/// and the handles cannot cross. Double-click resets the pair to the full
/// range.
pub struct ArchRangeSlider<'a, Message> {
    range: std::ops::RangeInclusive<f32>,
    low: f32,
    high: f32,
    on_change: Box<dyn Fn(f32, f32) -> Message + 'a>,
    width: Length,
    height: Length,
    handle_radius: f32,
    track_width: f32,
    step: Option<f32>,
    filled_color: Color,
    handle_color: Color,
    on_release: Option<Message>,
    on_right_click: Option<Message>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DraggedHandle {
    Low,
    High,
}

impl<'a, Message> ArchRangeSlider<'a, Message> {
    pub fn new<F>(range: std::ops::RangeInclusive<f32>, low: f32, high: f32, on_change: F) -> Self
    where
        F: Fn(f32, f32) -> Message + 'a,
    {
        Self {
            range,
            low,
            high,
            on_change: Box::new(on_change),
            width: Length::Fixed(64.0),
            height: Length::Fixed(64.0),
            handle_radius: 4.0,
            track_width: 6.0,
            step: None,
            filled_color: Color::from_rgb(
                0x29 as f32 / 255.0,
                0x66 as f32 / 255.0,
                0xA3 as f32 / 255.0,
            ),
            handle_color: Color::from_rgb(
                0x75 as f32 / 255.0,
                0xC2 as f32 / 255.0,
                0xFF as f32 / 255.0,
            ),
            on_release: None,
            on_right_click: None,
        }
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn track_width(mut self, track_width: f32) -> Self {
        self.track_width = track_width.max(1.0);
        self
    }

    pub fn handle_radius(mut self, handle_radius: f32) -> Self {
        self.handle_radius = handle_radius.max(0.5);
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = Some(step.abs()).filter(|step| *step > 0.0);
        self
    }

    pub fn filled_color(mut self, color: Color) -> Self {
        self.filled_color = color;
        self
    }

    pub fn handle_color(mut self, color: Color) -> Self {
        self.handle_color = color;
        self
    }

    pub fn on_release(mut self, message: Message) -> Self {
        self.on_release = Some(message);
        self
    }

    pub fn on_right_click(mut self, message: Message) -> Self {
        self.on_right_click = Some(message);
        self
    }
}

pub fn arch_range_slider<'a, Message, F>(
    range: std::ops::RangeInclusive<f32>,
    low: f32,
    high: f32,
    on_change: F,
) -> ArchRangeSlider<'a, Message>
where
    F: Fn(f32, f32) -> Message + 'a,
{
    ArchRangeSlider::new(range, low, high, on_change)
}

fn value_angle(value: f32, start: f32, end: f32) -> f32 {
    let normalized = if end > start {
        (value - start) / (end - start)
    } else {
        0.0
    };
    normalized.clamp(0.0, 1.0) * SWEEP_ANGLE
}

fn drag_value(start_value: f32, delta_y: f32, range_size: f32) -> f32 {
    start_value + (delta_y / 200.0) * range_size
}

fn clamp_to_step(value: f32, start: f32, end: f32, step: Option<f32>) -> f32 {
    let clamped = value.clamp(start, end);
    let Some(step) = step else {
        return clamped;
    };
    let steps = ((clamped - start) / step).round();
    (start + steps * step).clamp(start, end)
}

fn clamp_pair(
    dragged: f32,
    other: f32,
    handle: DraggedHandle,
    start: f32,
    end: f32,
    step: Option<f32>,
) -> (f32, f32) {
    match handle {
        DraggedHandle::Low => {
            let low = clamp_to_step(dragged.min(other), start, end, step);
            (low, clamp_to_step(other, start, end, step).max(low))
        }
        DraggedHandle::High => {
            let high = clamp_to_step(dragged.max(other), start, end, step);
            (clamp_to_step(other, start, end, step).min(high), high)
        }
    }
}

struct State {
    owner_id: u64,
    dragging: Option<DraggedHandle>,
    last_click_at: Option<Instant>,
    modifiers: keyboard::Modifiers,
    drag_start_y: f32,
    drag_start_low: f32,
    drag_start_high: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            owner_id: NEXT_OWNER_ID.fetch_add(1, Ordering::Relaxed),
            dragging: None,
            last_click_at: None,
            modifiers: keyboard::Modifiers::default(),
            drag_start_y: 0.0,
            drag_start_low: 0.0,
            drag_start_high: 0.0,
        }
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for ArchRangeSlider<'a, Message>
where
    Renderer: geometry::Renderer,
    Message: Clone,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.width(self.width).height(self.height).resolve(
            self.width,
            self.height,
            Size::ZERO,
        );
        layout::Node::new(size)
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = (center.x.min(center.y) - self.handle_radius - 2.0).max(1.0);

        let low_angle = value_angle(self.low, *self.range.start(), *self.range.end());
        let high_angle = value_angle(self.high, *self.range.start(), *self.range.end());

        let back_color = Color::from_rgb(
            0x42 as f32 / 255.0,
            0x46 as f32 / 255.0,
            0x4D as f32 / 255.0,
        );
        let border_color = Color::from_rgb(
            0x30 as f32 / 255.0,
            0x33 as f32 / 255.0,
            0x3C as f32 / 255.0,
        );

        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            let mut frame = Frame::new(renderer, bounds.size());

            let border_path = Path::new(|builder| {
                builder.arc(PathArc {
                    center: Point::new(center.x, center.y),
                    radius,
                    start_angle: Radians(START_ANGLE),
                    end_angle: Radians(END_ANGLE),
                });
            });
            frame.stroke(
                &border_path,
                Stroke::default()
                    .with_width(self.track_width + 2.0)
                    .with_color(border_color),
            );

            let track_path = Path::new(|builder| {
                builder.arc(PathArc {
                    center: Point::new(center.x, center.y),
                    radius,
                    start_angle: Radians(START_ANGLE),
                    end_angle: Radians(END_ANGLE),
                });
            });
            frame.stroke(
                &track_path,
                Stroke::default()
                    .with_width(self.track_width)
                    .with_color(back_color),
            );

            if high_angle > low_angle {
                let filled_path = Path::new(|builder| {
                    builder.arc(PathArc {
                        center: Point::new(center.x, center.y),
                        radius,
                        start_angle: Radians(START_ANGLE + low_angle),
                        end_angle: Radians(START_ANGLE + high_angle),
                    });
                });
                frame.stroke(
                    &filled_path,
                    Stroke::default()
                        .with_width(self.track_width)
                        .with_color(self.filled_color),
                );
            }

            for angle in [low_angle, high_angle] {
                let current_angle = START_ANGLE + angle;
                let handle_x = center.x + current_angle.cos() * radius;
                let handle_y = center.y + current_angle.sin() * radius;
                let handle_path = Path::circle(Point::new(handle_x, handle_y), self.handle_radius);
                frame.fill(&handle_path, self.handle_color);
            }

            renderer.draw_geometry(frame.into_geometry());
        });
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();

        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if !cursor.is_over(bounds) {
                    if state.dragging.is_some() {
                        state.dragging = None;
                        ACTIVE_DRAG_OWNER.with(|owner| {
                            if owner.get() == Some(state.owner_id) {
                                owner.set(None);
                            }
                        });
                    }
                    return;
                }

                ACTIVE_DRAG_OWNER.with(|owner| owner.set(Some(state.owner_id)));
                let now = Instant::now();
                let is_double_click = state
                    .last_click_at
                    .is_some_and(|last| now.duration_since(last) <= DOUBLE_CLICK);
                state.last_click_at = Some(now);
                if is_double_click {
                    shell.publish((self.on_change)(*self.range.start(), *self.range.end()));
                } else if let Some(cursor_position) = cursor.position() {
                    state.dragging = Some(if state.modifiers.shift() {
                        DraggedHandle::Low
                    } else {
                        DraggedHandle::High
                    });
                    state.drag_start_y = cursor_position.y;
                    state.drag_start_low = self.low;
                    state.drag_start_high = self.high;
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if !cursor.is_over(bounds) {
                    return;
                }
                if let Some(message) = self.on_right_click.as_ref() {
                    shell.publish(message.clone());
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.dragging.is_some() =>
            {
                state.dragging = None;
                ACTIVE_DRAG_OWNER.with(|owner| {
                    if owner.get() == Some(state.owner_id) {
                        owner.set(None);
                    }
                });
                if let Some(message) = self.on_release.as_ref() {
                    shell.publish(message.clone());
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.dragging.is_none() {
                    return;
                }
                let is_owner = ACTIVE_DRAG_OWNER.with(|owner| owner.get() == Some(state.owner_id));
                if !is_owner {
                    state.dragging = None;
                    return;
                }
                if let (Some(handle), Some(cursor_position)) = (state.dragging, cursor.position()) {
                    let delta_y = state.drag_start_y - cursor_position.y;
                    let range_size = self.range.end() - self.range.start();
                    let start_value = match handle {
                        DraggedHandle::Low => state.drag_start_low,
                        DraggedHandle::High => state.drag_start_high,
                    };
                    let other = match handle {
                        DraggedHandle::Low => state.drag_start_high,
                        DraggedHandle::High => state.drag_start_low,
                    };
                    let dragged = drag_value(start_value, delta_y, range_size);
                    let (low, high) = clamp_pair(
                        dragged,
                        other,
                        handle,
                        *self.range.start(),
                        *self.range.end(),
                        self.step,
                    );
                    shell.publish((self.on_change)(low, high));
                }
            }
            _ => {}
        }
    }
}

impl<'a, Message, Theme, Renderer> From<ArchRangeSlider<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: 'a + geometry::Renderer,
{
    fn from(slider: ArchRangeSlider<'a, Message>) -> Self {
        Self::new(slider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Event;
    use iced::advanced::{
        Layout, Shell, clipboard, layout,
        widget::{self, Tree, Widget},
    };
    use std::time::Instant;

    fn test_tree_with_state(state: State) -> Tree {
        Tree {
            tag: widget::tree::Tag::of::<State>(),
            state: widget::tree::State::new(state),
            children: Vec::new(),
        }
    }

    fn handle_position(value: f32, bounds_size: f32) -> Point {
        let radius = (bounds_size / 2.0 - 4.0 - 2.0).max(1.0);
        let angle = START_ANGLE + value_angle(value, 0.0, 1.0);
        Point::new(
            bounds_size / 2.0 + angle.cos() * radius,
            bounds_size / 2.0 + angle.sin() * radius,
        )
    }

    #[test]
    fn value_angle_maps_range_onto_sweep() {
        assert_eq!(value_angle(0.0, 0.0, 1.0), 0.0);
        assert!((value_angle(0.5, 0.0, 1.0) - SWEEP_ANGLE / 2.0).abs() < 1e-6);
        assert!((value_angle(1.0, 0.0, 1.0) - SWEEP_ANGLE).abs() < 1e-6);
        assert_eq!(value_angle(-1.0, 0.0, 1.0), 0.0);
        assert!((value_angle(2.0, 0.0, 1.0) - SWEEP_ANGLE).abs() < 1e-6);
    }

    #[test]
    fn clamp_pair_prevents_handles_crossing() {
        let (low, high) = clamp_pair(0.8, 0.6, DraggedHandle::Low, 0.0, 1.0, None);
        assert_eq!((low, high), (0.6, 0.6));
        let (low, high) = clamp_pair(0.2, 0.4, DraggedHandle::High, 0.0, 1.0, None);
        assert_eq!((low, high), (0.4, 0.4));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn update_drag_low_handle_publishes_pair() {
        let mut slider = ArchRangeSlider::new(0.0..=1.0, 0.2, 0.8, |low, high| (low, high))
            .width(Length::Fixed(100.0))
            .height(Length::Fixed(100.0));
        let mut tree = test_tree_with_state(State::default());
        let node = layout::Node::new(Size::new(100.0, 100.0));
        let layout = Layout::new(&node);
        let mut messages = Vec::new();
        let renderer = ();
        let mut clipboard = clipboard::Null;
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(100.0, 100.0));
        let press_at = handle_position(0.2, 100.0);

        {
            let mut shell = Shell::new(&mut messages);
            <ArchRangeSlider<'_, (f32, f32)> as Widget<(f32, f32), iced::Theme, ()>>::update(
                &mut slider,
                &mut tree,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                layout,
                mouse::Cursor::Available(press_at),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }
        assert!(messages.is_empty());

        {
            let mut shell = Shell::new(&mut messages);
            <ArchRangeSlider<'_, (f32, f32)> as Widget<(f32, f32), iced::Theme, ()>>::update(
                &mut slider,
                &mut tree,
                &Event::Mouse(mouse::Event::CursorMoved {
                    position: Point::new(press_at.x, press_at.y - 50.0),
                }),
                layout,
                mouse::Cursor::Available(Point::new(press_at.x, press_at.y - 50.0)),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }

        assert_eq!(messages.len(), 1);
        let (low, high) = messages[0];
        assert_eq!(low, 0.2);
        assert!(
            high > 0.8,
            "plain drag should move high toward the range end: {high}"
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn update_shift_drag_moves_low_handle() {
        let mut slider = ArchRangeSlider::new(0.0..=1.0, 0.2, 0.8, |low, high| (low, high))
            .width(Length::Fixed(100.0))
            .height(Length::Fixed(100.0));
        let mut tree = test_tree_with_state(State {
            modifiers: keyboard::Modifiers::SHIFT,
            ..State::default()
        });
        let node = layout::Node::new(Size::new(100.0, 100.0));
        let layout = Layout::new(&node);
        let mut messages = Vec::new();
        let renderer = ();
        let mut clipboard = clipboard::Null;
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(100.0, 100.0));
        let press_at = handle_position(0.8, 100.0);

        {
            let mut shell = Shell::new(&mut messages);
            <ArchRangeSlider<'_, (f32, f32)> as Widget<(f32, f32), iced::Theme, ()>>::update(
                &mut slider,
                &mut tree,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                layout,
                mouse::Cursor::Available(press_at),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }
        assert!(messages.is_empty());

        {
            let mut shell = Shell::new(&mut messages);
            <ArchRangeSlider<'_, (f32, f32)> as Widget<(f32, f32), iced::Theme, ()>>::update(
                &mut slider,
                &mut tree,
                &Event::Mouse(mouse::Event::CursorMoved {
                    position: Point::new(press_at.x, press_at.y - 50.0),
                }),
                layout,
                mouse::Cursor::Available(Point::new(press_at.x, press_at.y - 50.0)),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }

        assert_eq!(messages.len(), 1);
        let (low, high) = messages[0];
        assert!(
            low > 0.2 && low < 0.8,
            "shift drag should move low toward high: {low}"
        );
        assert_eq!(high, 0.8);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn update_double_click_resets_to_full_range() {
        let mut slider = ArchRangeSlider::new(0.0..=127.0, 20.0, 100.0, |low, high| (low, high))
            .width(Length::Fixed(100.0))
            .height(Length::Fixed(100.0));
        let mut tree = test_tree_with_state(State {
            dragging: None,
            last_click_at: Some(Instant::now()),
            drag_start_y: 0.0,
            drag_start_low: 0.0,
            drag_start_high: 0.0,
            ..State::default()
        });
        let node = layout::Node::new(Size::new(100.0, 100.0));
        let layout = Layout::new(&node);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        let renderer = ();
        let mut clipboard = clipboard::Null;
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(100.0, 100.0));

        <ArchRangeSlider<'_, (f32, f32)> as Widget<(f32, f32), iced::Theme, ()>>::update(
            &mut slider,
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            layout,
            mouse::Cursor::Available(Point::new(50.0, 50.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );

        assert_eq!(messages, vec![(0.0, 127.0)]);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn update_publishes_release_message() {
        let mut slider = ArchRangeSlider::new(0.0..=1.0, 0.2, 0.8, |low, high| (low, high))
            .width(Length::Fixed(100.0))
            .height(Length::Fixed(100.0))
            .on_release((0.0, 0.0));
        let mut tree = test_tree_with_state(State {
            dragging: Some(DraggedHandle::Low),
            last_click_at: None,
            drag_start_y: 0.0,
            drag_start_low: 0.2,
            drag_start_high: 0.8,
            ..State::default()
        });
        let node = layout::Node::new(Size::new(100.0, 100.0));
        let layout = Layout::new(&node);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        let renderer = ();
        let mut clipboard = clipboard::Null;
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(100.0, 100.0));

        <ArchRangeSlider<'_, (f32, f32)> as Widget<(f32, f32), iced::Theme, ()>>::update(
            &mut slider,
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            layout,
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );

        assert_eq!(messages, vec![(0.0, 0.0)]);
    }
}

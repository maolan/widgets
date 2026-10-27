use crate::consts::DOUBLE_CLICK;
#[cfg(test)]
use iced::Point;
use iced::advanced::Shell;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Tree, Widget};
use iced::{Border, Color, Element, Event, Length, Rectangle, Size};
use iced::{keyboard, mouse};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Orientation {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DraggedHandle {
    Low,
    High,
}

/// A slider with two handles editing a `low..=high` value pair (for example
/// velocity or key ranges). Defaults to a vertical orientation; call
/// [`RangeSlider::horizontal`] for a horizontal layout. Dragging moves the
/// high handle, Shift+dragging moves the low handle, and the handles cannot
/// cross. Double-click resets the pair to the full range.
pub struct RangeSlider<'a, Message> {
    range: std::ops::RangeInclusive<f32>,
    low: f32,
    high: f32,
    on_change: Box<dyn Fn(f32, f32) -> Message + 'a>,
    width: Length,
    height: Length,
    handle_size: f32,
    step: Option<f32>,
    orientation: Orientation,
}

impl<'a, Message> RangeSlider<'a, Message> {
    pub fn new<F>(range: std::ops::RangeInclusive<f32>, low: f32, high: f32, on_change: F) -> Self
    where
        F: Fn(f32, f32) -> Message + 'a,
    {
        Self {
            range,
            low,
            high,
            on_change: Box::new(on_change),
            width: Length::Fixed(14.0),
            height: Length::Fixed(300.0),
            handle_size: 4.0,
            step: None,
            orientation: Orientation::Vertical,
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

    pub fn handle_size(mut self, handle_size: f32) -> Self {
        self.handle_size = handle_size.max(1.0);
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = Some(step.abs()).filter(|step| *step > 0.0);
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.orientation = Orientation::Horizontal;
        self
    }

    fn handle_position(&self, value: f32, bounds: Rectangle) -> f32 {
        let (inner_origin, inner_length) = self.inner_track(bounds);
        handle_position(
            value,
            *self.range.start(),
            *self.range.end(),
            inner_origin,
            inner_length,
            self.orientation,
        )
    }

    fn inner_track(&self, bounds: Rectangle) -> (f32, f32) {
        match self.orientation {
            Orientation::Vertical => (
                bounds.y + self.handle_size / 2.0,
                (bounds.height - self.handle_size).max(1.0),
            ),
            Orientation::Horizontal => (
                bounds.x + self.handle_size / 2.0,
                (bounds.width - self.handle_size).max(1.0),
            ),
        }
    }
}

pub fn range_slider<'a, Message, F>(
    range: std::ops::RangeInclusive<f32>,
    low: f32,
    high: f32,
    on_change: F,
) -> RangeSlider<'a, Message>
where
    F: Fn(f32, f32) -> Message + 'a,
{
    RangeSlider::new(range, low, high, on_change)
}

fn handle_position(
    value: f32,
    start: f32,
    end: f32,
    inner_origin: f32,
    inner_length: f32,
    orientation: Orientation,
) -> f32 {
    let normalized = if end > start {
        (value - start) / (end - start)
    } else {
        0.0
    }
    .clamp(0.0, 1.0);
    match orientation {
        Orientation::Horizontal => inner_origin + normalized * inner_length,
        Orientation::Vertical => inner_origin + (1.0 - normalized) * inner_length,
    }
}

fn drag_value(
    start_value: f32,
    start_position: f32,
    cursor_position: f32,
    inner_length: f32,
    range_size: f32,
    orientation: Orientation,
) -> f32 {
    let delta = match orientation {
        Orientation::Horizontal => cursor_position - start_position,
        Orientation::Vertical => start_position - cursor_position,
    };
    start_value + (delta / inner_length.max(1.0)) * range_size
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

#[derive(Default)]
struct State {
    dragging: Option<DraggedHandle>,
    last_click_at: Option<Instant>,
    modifiers: keyboard::Modifiers,
    drag_start_x: f32,
    drag_start_y: f32,
    drag_start_low: f32,
    drag_start_high: f32,
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for RangeSlider<'a, Message>
where
    Renderer: renderer::Renderer,
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
        let border_width = 1.0;

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
        let filled_color = Color::from_rgb(
            0x29 as f32 / 255.0,
            0x66 as f32 / 255.0,
            0xA3 as f32 / 255.0,
        );
        let handle_color = Color::from_rgb(
            0x75 as f32 / 255.0,
            0xC2 as f32 / 255.0,
            0xFF as f32 / 255.0,
        );

        let border_radius = 2.0;

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x,
                    y: bounds.y,
                    width: bounds.width,
                    height: bounds.height,
                },
                border: Border {
                    radius: border_radius.into(),
                    width: border_width,
                    color: border_color,
                },
                ..Default::default()
            },
            back_color,
        );

        let low_position = self.handle_position(self.low, bounds);
        let high_position = self.handle_position(self.high, bounds);

        match self.orientation {
            Orientation::Vertical => {
                let band_top = low_position.min(high_position);
                let band_height = (high_position - low_position).abs();
                if band_height > 0.0 {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x: bounds.x,
                                y: band_top,
                                width: bounds.width,
                                height: band_height,
                            },
                            border: Border {
                                radius: border_radius.into(),
                                width: border_width,
                                color: Color::TRANSPARENT,
                            },
                            ..Default::default()
                        },
                        filled_color,
                    );
                }
                for position in [low_position, high_position] {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x: bounds.x,
                                y: position - self.handle_size / 2.0 - border_width,
                                width: bounds.width,
                                height: self.handle_size + border_width * 2.0,
                            },
                            border: Border {
                                radius: border_radius.into(),
                                width: border_width,
                                color: Color::TRANSPARENT,
                            },
                            ..Default::default()
                        },
                        handle_color,
                    );
                }
            }
            Orientation::Horizontal => {
                let band_left = low_position.min(high_position);
                let band_width = (high_position - low_position).abs();
                if band_width > 0.0 {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x: band_left,
                                y: bounds.y,
                                width: band_width,
                                height: bounds.height,
                            },
                            border: Border {
                                radius: border_radius.into(),
                                width: border_width,
                                color: Color::TRANSPARENT,
                            },
                            ..Default::default()
                        },
                        filled_color,
                    );
                }
                for position in [low_position, high_position] {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x: position - self.handle_size / 2.0 - border_width,
                                y: bounds.y,
                                width: self.handle_size + border_width * 2.0,
                                height: bounds.height,
                            },
                            border: Border {
                                radius: border_radius.into(),
                                width: border_width,
                                color: Color::TRANSPARENT,
                            },
                            ..Default::default()
                        },
                        handle_color,
                    );
                }
            }
        }
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
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(bounds) =>
            {
                let now = Instant::now();
                let is_double_click = state
                    .last_click_at
                    .is_some_and(|last| now.duration_since(last) <= DOUBLE_CLICK);
                state.last_click_at = Some(now);
                if is_double_click {
                    shell.publish((self.on_change)(*self.range.start(), *self.range.end()));
                    shell.capture_event();
                } else if let Some(cursor_position) = cursor.position() {
                    state.dragging = Some(if state.modifiers.shift() {
                        DraggedHandle::Low
                    } else {
                        DraggedHandle::High
                    });
                    state.drag_start_x = cursor_position.x;
                    state.drag_start_y = cursor_position.y;
                    state.drag_start_low = self.low;
                    state.drag_start_high = self.high;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.dragging.is_some() =>
            {
                state.dragging = None;
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(handle) = state.dragging
                    && let Some(cursor_position) = cursor.position()
                {
                    let (_, inner_length) = self.inner_track(bounds);
                    let range_size = self.range.end() - self.range.start();
                    let start_position = match self.orientation {
                        Orientation::Horizontal => state.drag_start_x,
                        Orientation::Vertical => state.drag_start_y,
                    };
                    let start_value = match handle {
                        DraggedHandle::Low => state.drag_start_low,
                        DraggedHandle::High => state.drag_start_high,
                    };
                    let cursor_position = match self.orientation {
                        Orientation::Horizontal => cursor_position.x,
                        Orientation::Vertical => cursor_position.y,
                    };
                    let dragged = drag_value(
                        start_value,
                        start_position,
                        cursor_position,
                        inner_length,
                        range_size,
                        self.orientation,
                    );
                    let other = match handle {
                        DraggedHandle::Low => state.drag_start_high,
                        DraggedHandle::High => state.drag_start_low,
                    };
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

impl<'a, Message, Theme, Renderer> From<RangeSlider<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(slider: RangeSlider<'a, Message>) -> Self {
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

    #[test]
    fn handle_position_maps_range_to_inner_track() {
        assert_eq!(
            handle_position(0.0, 0.0, 1.0, 10.0, 80.0, Orientation::Horizontal),
            10.0
        );
        assert_eq!(
            handle_position(1.0, 0.0, 1.0, 10.0, 80.0, Orientation::Horizontal),
            90.0
        );
        assert_eq!(
            handle_position(0.5, 0.0, 1.0, 10.0, 80.0, Orientation::Horizontal),
            50.0
        );
        assert_eq!(
            handle_position(0.0, 0.0, 1.0, 10.0, 80.0, Orientation::Vertical),
            90.0
        );
        assert_eq!(
            handle_position(1.0, 0.0, 1.0, 10.0, 80.0, Orientation::Vertical),
            10.0
        );
    }

    #[test]
    fn handle_position_clamps_out_of_range_values() {
        assert_eq!(
            handle_position(2.0, 0.0, 1.0, 0.0, 100.0, Orientation::Horizontal),
            100.0
        );
        assert_eq!(
            handle_position(-1.0, 0.0, 1.0, 0.0, 100.0, Orientation::Horizontal),
            0.0
        );
    }

    #[test]
    fn drag_value_scales_cursor_delta_to_range() {
        assert_eq!(
            drag_value(0.5, 50.0, 60.0, 100.0, 1.0, Orientation::Horizontal),
            0.6
        );
        assert_eq!(
            drag_value(0.5, 50.0, 40.0, 100.0, 1.0, Orientation::Horizontal),
            0.4
        );
        assert_eq!(
            drag_value(0.5, 50.0, 40.0, 100.0, 1.0, Orientation::Vertical),
            0.6
        );
        assert_eq!(
            drag_value(0.5, 50.0, 60.0, 100.0, 1.0, Orientation::Vertical),
            0.4
        );
    }

    #[test]
    fn clamp_pair_prevents_handles_crossing() {
        let (low, high) = clamp_pair(0.8, 0.6, DraggedHandle::Low, 0.0, 1.0, None);
        assert_eq!((low, high), (0.6, 0.6));
        let (low, high) = clamp_pair(0.2, 0.4, DraggedHandle::High, 0.0, 1.0, None);
        assert_eq!((low, high), (0.4, 0.4));
    }

    #[test]
    fn clamp_pair_snaps_to_step() {
        let (low, high) = clamp_pair(0.26, 0.74, DraggedHandle::Low, 0.0, 1.0, Some(0.1));
        assert_eq!((low, high), (0.3, 0.7));
    }

    #[test]
    fn clamp_pair_clamps_to_range() {
        let (low, high) = clamp_pair(1.4, 0.5, DraggedHandle::High, 0.0, 1.0, None);
        assert_eq!((low, high), (0.5, 1.0));
    }

    #[cfg(debug_assertions)]
    fn press(slider: &mut RangeSlider<'_, (f32, f32)>, tree: &mut Tree, at: Point) {
        let node = layout::Node::new(Size::new(150.0, 11.0));
        let layout = Layout::new(&node);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        let renderer = ();
        let mut clipboard = clipboard::Null;
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(150.0, 11.0));
        <RangeSlider<'_, (f32, f32)> as Widget<(f32, f32), iced::Theme, ()>>::update(
            slider,
            tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            layout,
            mouse::Cursor::Available(at),
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        assert!(messages.is_empty());
    }

    #[cfg(debug_assertions)]
    fn drag_to(
        slider: &mut RangeSlider<'_, (f32, f32)>,
        tree: &mut Tree,
        from: Point,
        to: Point,
    ) -> Vec<(f32, f32)> {
        press(slider, tree, from);
        let node = layout::Node::new(Size::new(150.0, 11.0));
        let layout = Layout::new(&node);
        let mut messages = Vec::new();
        let renderer = ();
        let mut clipboard = clipboard::Null;
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(150.0, 11.0));
        {
            let mut shell = Shell::new(&mut messages);
            <RangeSlider<'_, (f32, f32)> as Widget<(f32, f32), iced::Theme, ()>>::update(
                slider,
                tree,
                &Event::Mouse(mouse::Event::CursorMoved { position: to }),
                layout,
                mouse::Cursor::Available(to),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }
        messages
    }

    #[cfg(debug_assertions)]
    #[test]
    fn shift_drag_low_handle_stops_at_high_handle() {
        let mut slider = RangeSlider::new(0.0..=1.0, 0.2, 0.8, |low, high| (low, high))
            .horizontal()
            .width(Length::Fixed(150.0))
            .height(Length::Fixed(11.0));
        let mut tree = test_tree_with_state(State {
            modifiers: keyboard::Modifiers::SHIFT,
            ..State::default()
        });

        let messages = drag_to(
            &mut slider,
            &mut tree,
            Point::new(45.0, 5.0),
            Point::new(150.0, 5.0),
        );

        assert_eq!(messages.len(), 1);
        let (low, high) = messages[0];
        assert_eq!(
            (low, high),
            (0.8, 0.8),
            "low should clamp at high instead of crossing: {low}"
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn plain_drag_moves_high_handle_even_near_low_handle() {
        let mut slider = RangeSlider::new(0.0..=1.0, 0.2, 0.8, |low, high| (low, high))
            .horizontal()
            .width(Length::Fixed(150.0))
            .height(Length::Fixed(11.0));
        let mut tree = test_tree_with_state(State::default());

        // Press near the low handle (x=45); a plain drag must still move high.
        let messages = drag_to(
            &mut slider,
            &mut tree,
            Point::new(45.0, 5.0),
            Point::new(75.0, 5.0),
        );

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
    fn shift_drag_moves_low_handle() {
        let mut slider = RangeSlider::new(0.0..=1.0, 0.2, 0.8, |low, high| (low, high))
            .horizontal()
            .width(Length::Fixed(150.0))
            .height(Length::Fixed(11.0));
        let mut tree = test_tree_with_state(State {
            modifiers: keyboard::Modifiers::SHIFT,
            ..State::default()
        });

        // Press near the high handle (x=120); shift+drag must move low.
        let messages = drag_to(
            &mut slider,
            &mut tree,
            Point::new(120.0, 5.0),
            Point::new(90.0, 5.0),
        );

        assert_eq!(messages.len(), 1);
        let (low, high) = messages[0];
        assert!(
            low < 0.2,
            "shift drag should move low toward the range start: {low}"
        );
        assert_eq!(high, 0.8);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn double_click_resets_to_full_range() {
        let mut slider = RangeSlider::new(0.0..=127.0, 20.0, 100.0, |low, high| (low, high))
            .horizontal()
            .width(Length::Fixed(150.0))
            .height(Length::Fixed(11.0));
        let mut tree = test_tree_with_state(State {
            dragging: None,
            last_click_at: Some(Instant::now()),
            ..State::default()
        });
        let node = layout::Node::new(Size::new(150.0, 11.0));
        let layout = Layout::new(&node);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        let renderer = ();
        let mut clipboard = clipboard::Null;
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(150.0, 11.0));

        <RangeSlider<'_, (f32, f32)> as Widget<(f32, f32), iced::Theme, ()>>::update(
            &mut slider,
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            layout,
            mouse::Cursor::Available(Point::new(75.0, 5.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );

        assert_eq!(messages, vec![(0.0, 127.0)]);
    }
}

use iced::advanced::Shell;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::text;
use iced::advanced::widget::{self, Tree, Widget};
use iced::alignment;
use iced::mouse;
use iced::{Border, Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

pub struct MultiToggler<'a, Message> {
    labels: Vec<String>,
    selected: usize,
    on_change: Box<dyn Fn(usize) -> Message + 'a>,
    orientation: Orientation,
    width: Length,
    height: Length,
    text_size: Pixels,
    background_color: Color,
    border_color: Color,
    selected_color: Color,
    text_color: Color,
    selected_text_color: Color,
    size: f32,
    spacing: f32,
    label_extent: f32,
}

impl<'a, Message> MultiToggler<'a, Message> {
    pub fn new<F, L, S>(labels: L, selected: usize, orientation: Orientation, on_change: F) -> Self
    where
        F: Fn(usize) -> Message + 'a,
        L: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            labels: labels.into_iter().map(Into::into).collect(),
            selected,
            on_change: Box::new(on_change),
            orientation,
            width: Length::Shrink,
            height: Length::Shrink,
            text_size: Pixels(12.0),
            background_color: Color::from_rgb(
                0x42 as f32 / 255.0,
                0x46 as f32 / 255.0,
                0x4D as f32 / 255.0,
            ),
            border_color: Color::from_rgb(
                0x30 as f32 / 255.0,
                0x33 as f32 / 255.0,
                0x3C as f32 / 255.0,
            ),
            selected_color: Color::from_rgb(
                0x29 as f32 / 255.0,
                0x66 as f32 / 255.0,
                0xA3 as f32 / 255.0,
            ),
            text_color: Color::from_rgb(
                0xD7 as f32 / 255.0,
                0xDA as f32 / 255.0,
                0xE0 as f32 / 255.0,
            ),
            selected_text_color: Color::WHITE,
            size: 16.0,
            spacing: 8.0,
            label_extent: 72.0,
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

    pub fn text_size(mut self, text_size: impl Into<Pixels>) -> Self {
        self.text_size = text_size.into();
        self
    }

    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into().0.max(1.0);
        self
    }

    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = spacing.into().0.max(0.0);
        self
    }

    pub fn label_extent(mut self, label_extent: impl Into<Pixels>) -> Self {
        self.label_extent = label_extent.into().0.max(0.0);
        self
    }

    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub fn selected_color(mut self, color: Color) -> Self {
        self.selected_color = color;
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    pub fn selected_text_color(mut self, color: Color) -> Self {
        self.selected_text_color = color;
        self
    }

    fn selected(&self) -> usize {
        self.selected.min(self.labels.len().saturating_sub(1))
    }

    fn option_count(&self) -> usize {
        self.labels.len().max(1)
    }

    fn label_height(&self) -> f32 {
        self.text_size.0 * 1.4
    }

    fn track_bounds(&self, bounds: Rectangle) -> Rectangle {
        let count = self.option_count() as f32;
        match self.orientation {
            Orientation::Horizontal => Rectangle {
                x: bounds.x,
                y: bounds.y,
                width: self.size * count,
                height: self.size,
            },
            Orientation::Vertical => Rectangle {
                x: bounds.x,
                y: bounds.y,
                width: self.size,
                height: self.size * count,
            },
        }
    }

    fn option_center(&self, track: Rectangle, index: usize) -> Point {
        match self.orientation {
            Orientation::Horizontal => Point::new(
                track.x + self.size * (index as f32 + 0.5),
                track.y + track.height * 0.5,
            ),
            Orientation::Vertical => Point::new(
                track.x + track.width * 0.5,
                track.y + self.size * (index as f32 + 0.5),
            ),
        }
    }

    fn label_bounds(&self, bounds: Rectangle) -> Rectangle {
        let track = self.track_bounds(bounds);
        let label_height = self.label_height();
        match self.orientation {
            Orientation::Horizontal => Rectangle {
                x: track.x,
                y: track.y + track.height + self.spacing,
                width: self.label_extent,
                height: label_height,
            },
            Orientation::Vertical => {
                let center = self.option_center(track, self.selected());
                Rectangle {
                    x: track.x + track.width + self.spacing,
                    y: center.y - label_height * 0.5,
                    width: self.label_extent,
                    height: label_height,
                }
            }
        }
    }

    fn index_at(&self, cursor_position: Point, bounds: Rectangle) -> Option<usize> {
        if self.labels.is_empty() {
            return None;
        }

        let track = self.track_bounds(bounds);
        if !track.contains(cursor_position) {
            return None;
        }
        let count = self.labels.len();
        let raw = match self.orientation {
            Orientation::Horizontal => (cursor_position.x - track.x) / self.size,
            Orientation::Vertical => (cursor_position.y - track.y) / self.size,
        };

        Some((raw.floor() as isize).clamp(0, count as isize - 1) as usize)
    }

    fn publish_index(&self, state: &mut State, index: usize, shell: &mut Shell<'_, Message>) {
        if state.active_index != Some(index) {
            state.active_index = Some(index);
            shell.publish((self.on_change)(index));
        }
    }
}

pub fn horizontal_multi_toggler<'a, Message, F, L, S>(
    labels: L,
    selected: usize,
    on_change: F,
) -> MultiToggler<'a, Message>
where
    F: Fn(usize) -> Message + 'a,
    L: IntoIterator<Item = S>,
    S: Into<String>,
{
    MultiToggler::new(labels, selected, Orientation::Horizontal, on_change)
}

pub fn vertical_multi_toggler<'a, Message, F, L, S>(
    labels: L,
    selected: usize,
    on_change: F,
) -> MultiToggler<'a, Message>
where
    F: Fn(usize) -> Message + 'a,
    L: IntoIterator<Item = S>,
    S: Into<String>,
{
    MultiToggler::new(labels, selected, Orientation::Vertical, on_change)
}

#[derive(Default)]
struct State {
    is_dragging: bool,
    active_index: Option<usize>,
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for MultiToggler<'a, Message>
where
    Renderer: renderer::Renderer + text::Renderer<Font = Font>,
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
        let option_count = self.option_count() as f32;
        let default_size = match self.orientation {
            Orientation::Horizontal => Size::new(
                (self.size * option_count).max(self.label_extent),
                self.size + self.spacing + self.label_height(),
            ),
            Orientation::Vertical => Size::new(
                self.size + self.spacing + self.label_extent,
                self.size * option_count,
            ),
        };
        let size = limits.width(self.width).height(self.height).resolve(
            self.width,
            self.height,
            default_size,
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
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let track = self.track_bounds(bounds);
        let border_radius = track.width.min(track.height) / 2.0;

        renderer.fill_quad(
            renderer::Quad {
                bounds: track,
                border: Border {
                    radius: border_radius.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                ..Default::default()
            },
            self.background_color,
        );

        if !self.labels.is_empty() {
            let padding = (0.1 * self.size).round();
            let diameter = (self.size - 2.0 * padding).max(1.0);
            let center = self.option_center(track, self.selected());
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: center.x - diameter * 0.5,
                        y: center.y - diameter * 0.5,
                        width: diameter,
                        height: diameter,
                    },
                    border: Border {
                        radius: (diameter * 0.5).into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                    ..Default::default()
                },
                self.selected_color,
            );
        }

        if let Some(label) = self.labels.get(self.selected()) {
            let label_bounds = self.label_bounds(bounds);
            renderer.fill_text(
                text::Text {
                    content: label.clone(),
                    bounds: label_bounds.size(),
                    size: self.text_size,
                    line_height: text::LineHeight::default(),
                    font: renderer.default_font(),
                    align_x: text::Alignment::Left,
                    align_y: alignment::Vertical::Center,
                    shaping: text::Shaping::default(),
                    wrapping: text::Wrapping::None,
                },
                Point::new(label_bounds.x, label_bounds.center_y()),
                self.text_color,
                *viewport,
            );
        }
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if !self.labels.is_empty() && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
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
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(bounds)
                    && let Some(cursor_position) = cursor.position()
                    && let Some(index) = self.index_at(cursor_position, bounds)
                {
                    state.is_dragging = true;
                    state.active_index = Some(self.selected());
                    self.publish_index(state, index, shell);
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.is_dragging = false;
                state.active_index = None;
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.is_dragging
                    && let Some(cursor_position) = cursor.position()
                    && let Some(index) = self.index_at(cursor_position, bounds)
                {
                    self.publish_index(state, index, shell);
                    shell.capture_event();
                }
            }
            _ => {}
        }
    }
}

impl<'a, Message, Theme, Renderer> From<MultiToggler<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + text::Renderer<Font = Font> + 'a,
{
    fn from(toggler: MultiToggler<'a, Message>) -> Self {
        Self::new(toggler)
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

    fn test_tree() -> Tree {
        Tree {
            tag: widget::tree::Tag::of::<State>(),
            state: widget::tree::State::new(State::default()),
            children: Vec::new(),
        }
    }

    fn test_layout(width: f32, height: f32) -> Layout<'static> {
        let node = Box::leak(Box::new(layout::Node::new(Size::new(width, height))));
        Layout::new(node)
    }

    #[test]
    fn horizontal_index_at_uses_segments() {
        let toggler = horizontal_multi_toggler(["A", "B", "C"], 0, |index| index);
        let bounds = Rectangle {
            x: 10.0,
            y: 20.0,
            width: 90.0,
            height: 24.0,
        };

        assert_eq!(toggler.index_at(Point::new(10.0, 30.0), bounds), Some(0));
        assert_eq!(toggler.index_at(Point::new(30.0, 30.0), bounds), Some(1));
        assert_eq!(toggler.index_at(Point::new(57.0, 30.0), bounds), Some(2));
        assert_eq!(toggler.index_at(Point::new(59.0, 30.0), bounds), None);
    }

    #[test]
    fn horizontal_track_and_label_are_left_aligned() {
        let toggler = horizontal_multi_toggler(["A", "B", "C"], 0, |index| index);
        let bounds = Rectangle {
            x: 10.0,
            y: 20.0,
            width: 90.0,
            height: 48.0,
        };

        let track = toggler.track_bounds(bounds);
        assert_eq!(track.x, bounds.x);
        assert_eq!(track.width, 48.0);

        let label = toggler.label_bounds(bounds);
        assert_eq!(label.x, track.x);
        assert_eq!(label.width, 72.0);
    }

    #[test]
    fn vertical_index_at_uses_segments() {
        let toggler = vertical_multi_toggler(["A", "B", "C"], 0, |index| index);
        let bounds = Rectangle {
            x: 10.0,
            y: 20.0,
            width: 72.0,
            height: 90.0,
        };

        assert_eq!(toggler.index_at(Point::new(20.0, 20.0), bounds), Some(0));
        assert_eq!(toggler.index_at(Point::new(20.0, 45.0), bounds), Some(1));
        assert_eq!(toggler.index_at(Point::new(20.0, 67.0), bounds), Some(2));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn click_publishes_selected_index() {
        let mut toggler = horizontal_multi_toggler(["A", "B", "C"], 0, |index| index);
        let mut tree = test_tree();
        let layout = test_layout(90.0, 24.0);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        let renderer = ();
        let mut clipboard = clipboard::Null;
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(90.0, 24.0));

        <MultiToggler<'_, usize> as Widget<usize, iced::Theme, ()>>::update(
            &mut toggler,
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            layout,
            mouse::Cursor::Available(Point::new(30.0, 12.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );

        assert_eq!(messages, vec![1]);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn drag_can_publish_return_to_starting_index() {
        let mut toggler = horizontal_multi_toggler(["A", "B", "C"], 0, |index| index);
        let mut tree = test_tree();
        let layout = test_layout(90.0, 24.0);
        let renderer = ();
        let mut clipboard = clipboard::Null;
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(90.0, 24.0));
        let mut messages = Vec::new();

        {
            let mut shell = Shell::new(&mut messages);
            <MultiToggler<'_, usize> as Widget<usize, iced::Theme, ()>>::update(
                &mut toggler,
                &mut tree,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                layout,
                mouse::Cursor::Available(Point::new(30.0, 12.0)),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }

        {
            let mut shell = Shell::new(&mut messages);
            <MultiToggler<'_, usize> as Widget<usize, iced::Theme, ()>>::update(
                &mut toggler,
                &mut tree,
                &Event::Mouse(mouse::Event::CursorMoved {
                    position: Point::new(15.0, 12.0),
                }),
                layout,
                mouse::Cursor::Available(Point::new(15.0, 12.0)),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }

        assert_eq!(messages, vec![1, 0]);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn click_on_current_index_does_not_publish() {
        let mut toggler = horizontal_multi_toggler(["A", "B", "C"], 1, |index| index);
        let mut tree = test_tree();
        let layout = test_layout(90.0, 24.0);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        let renderer = ();
        let mut clipboard = clipboard::Null;
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(90.0, 24.0));

        <MultiToggler<'_, usize> as Widget<usize, iced::Theme, ()>>::update(
            &mut toggler,
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            layout,
            mouse::Cursor::Available(Point::new(30.0, 12.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );

        assert!(messages.is_empty());
    }
}

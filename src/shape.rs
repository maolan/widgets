use iced::{
    Color, Element, Length, Point, Rectangle, Renderer, Theme, mouse,
    widget::{
        canvas,
        canvas::{Frame, Geometry, Path, Program, Stroke},
    },
};

struct Shape {
    samples: Vec<f32>,
}

pub fn shape<'a, Message: 'a>(samples: Vec<f32>) -> Element<'a, Message> {
    canvas(Shape { samples })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

impl<Message> Program<Message> for Shape {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color::from_rgb(0.075, 0.09, 0.115),
        );

        let center_y = bounds.height * 0.5;
        let center = Path::line(
            Point::new(0.0, center_y),
            Point::new(bounds.width, center_y),
        );
        frame.stroke(
            &center,
            Stroke::default()
                .with_color(Color::from_rgba(0.55, 0.60, 0.66, 0.22))
                .with_width(1.0),
        );

        let denominator = self.samples.len().saturating_sub(1).max(1) as f32;
        let path = Path::new(|builder| {
            for (index, sample) in self.samples.iter().copied().enumerate() {
                let x = index as f32 / denominator * bounds.width;
                let y = center_y - sample.clamp(-1.0, 1.0) * center_y * 0.9;
                if index == 0 {
                    builder.move_to(Point::new(x, y));
                } else {
                    builder.line_to(Point::new(x, y));
                }
            }
        });
        frame.stroke(
            &path,
            Stroke::default()
                .with_color(Color::from_rgb(0.72, 0.58, 0.28))
                .with_width(1.6),
        );

        vec![frame.into_geometry()]
    }
}

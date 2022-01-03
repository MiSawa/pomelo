use pomelo_common::graphics::GraphicConfig;

use crate::{
    graphics::{
        layer::{self, LayerManager, WindowID},
        screen::{self, Screen},
        widgets::{self, console},
        Draw, Rectangle, Size, UCoordinate,
    },
    mouse,
};

pub fn create_gui(graphic_config: &GraphicConfig) -> GUI {
    let mut layer_manager = layer::create_layer_manager(graphic_config);
    let screen = screen::create_screen(graphic_config);
    let size = Size::new(
        graphic_config.horisontal_resolution as UCoordinate,
        graphic_config.vertical_resolution as UCoordinate,
    );
    layer_manager.add(widgets::Desktop::new(size));
    console::register(&mut layer_manager);
    mouse::initialize(&mut layer_manager);

    use alloc::string::ToString;
    let counter = crate::graphics::widgets::Framed::new("Counter".to_string(), Counter::new());
    let mut counter = layer_manager.add(counter);
    counter.buffer();
    counter.move_relative(crate::graphics::Vector2d::new(300, 200));

    GUI {
        layer_manager,
        screen,
        counter,
    }
}
pub struct Counter(usize);
impl Counter {
    fn new() -> Self {
        Self(0)
    }
    fn inc(&mut self) {
        self.0 += 1;
    }
}
impl Draw for Counter {
    fn size(&self) -> Size {
        Size::new(
            crate::graphics::canvas::GLYPH_WIDTH * 20,
            crate::graphics::canvas::GLYPH_HEIGHT,
        )
    }
    fn draw<C: crate::graphics::canvas::Canvas>(&self, canvas: &mut C) {
        use super::graphics::{Color, Point};
        canvas
            .draw_fmt(Color::BLACK, Point::zero(), format_args!("{:010}", self.0))
            .ok();
    }
}

pub struct GUI {
    layer_manager: LayerManager,
    screen: Screen,
    counter: widgets::Widget<widgets::Framed<Counter>>,
}

impl GUI {
    pub fn render(&mut self) {
        self.layer_manager.draw(&mut self.screen);
    }

    pub fn render_window(&mut self, id: WindowID) {
        self.layer_manager.draw_window(&mut self.screen, id);
    }

    pub fn render_area(&mut self, area: Rectangle) {
        self.layer_manager.draw_area(&mut self.screen, area);
    }

    pub fn inc_counter(&mut self) {
        self.counter.draw_mut().draw_mut().inc();
        self.counter.buffer();
        self.render_window(self.counter.window_id());
    }
}

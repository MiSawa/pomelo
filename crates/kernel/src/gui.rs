use pomelo_common::graphics::GraphicConfig;

use crate::{
    graphics::{
        layer::{self, LayerManager},
        screen::{self, Screen},
        widgets::{self, console},
        Draw, Size, UCoordinate,
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

    struct Message;
    impl Draw for Message {
        fn size(&self) -> Size {
            Size::new(300, 100)
        }
        fn draw<C: crate::graphics::canvas::Canvas>(&self, canvas: &mut C) {
            use super::graphics::{Color, Point};
            canvas.draw_string(Color::BLACK, Point::zero(), "Welcome to");
            canvas.draw_string(Color::BLACK, Point::new(0, 20), " PomeloOS world!");
        }
    }
    use alloc::string::ToString;
    let framed_message =
        crate::graphics::widgets::Framed::new("Hello PomeloOS".to_string(), Message);
    let mut w = layer_manager.add(framed_message);
    w.buffer();
    w.move_relative(crate::graphics::Vector2d::new(300, 200));

    GUI {
        layer_manager,
        screen,
    }
}

pub struct GUI {
    layer_manager: LayerManager,
    screen: Screen,
}

impl GUI {
    pub fn render(&mut self) {
        self.layer_manager.draw(&mut self.screen);
    }
}

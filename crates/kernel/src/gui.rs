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

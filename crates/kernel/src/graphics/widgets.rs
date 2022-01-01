use super::{layer::SharedWindow, Draw};

pub mod console;

pub struct Widget<D: Draw> {
    layer: SharedWindow,
    draw: D,
}

impl<D: Draw> Widget<D> {
    pub fn new(layer: SharedWindow, draw: D) -> Self {
        Self { layer, draw }
    }
}

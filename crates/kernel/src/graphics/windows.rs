use alloc::vec::Vec;

use crate::triple_buffer::Producer;

use super::{
    buffer::BufferCanvas,
    window_manager::{WindowId, WindowStateShared},
};

pub struct Window {
    id: WindowId,
    state: WindowStateShared,
    buffer: Producer<BufferCanvas<Vec<u8>>>,
}

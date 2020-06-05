use std::sync::mpsc::Receiver;
use std::sync::Mutex;

use glfw::{Glfw, Window, WindowEvent};

pub struct AppWindow {
    pub glfw: Glfw,
    pub window: Mutex<Window>,
    pub events: Mutex<Receiver<(f64, WindowEvent)>>,
}

use std::sync::{Arc, Mutex};
use crate::window_manager::WindowManager;

pub struct WMState {
    pub(crate) window_manager: Arc<Mutex<WindowManager>>,
}
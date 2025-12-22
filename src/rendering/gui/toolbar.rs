// Toolbar component with navigation controls
// This will be fully implemented once we determine the correct GPUI 0.2.2 API

pub struct Toolbar {
    can_go_back: bool,
    can_go_forward: bool,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            can_go_back: false,
            can_go_forward: false,
        }
    }

    pub fn set_navigation_state(&mut self, can_go_back: bool, can_go_forward: bool) {
        self.can_go_back = can_go_back;
        self.can_go_forward = can_go_forward;
    }
}

use std::collections::{HashMap, HashSet};
use winit::{
    dpi::{PhysicalSize, PhysicalPosition},
    event::{DeviceEvent, ElementState, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent, MouseScrollDelta},
};

const SENSITIVITY: f32 = 1e-2;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Input {
    Press(VirtualKeyCode),
    Release(VirtualKeyCode),
    Button(MouseButton),
    Scroll,
    Motion,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Action {
    #[default]
    Nop,

    ExitGame,
    Focus,
    Fullscreen,
    Place,
    Select,
    Pause,

    Resize {
        width: u32,
        height: u32,
    },

    Turn,
    Walk(Direction3),
    Stop(Direction3),
}

#[derive(Debug, Clone, Copy)]
pub enum Direction3 {
    Forward,
    Backward,
    Left,
    Right,
    Up,
    Down,
}

impl From<Direction3> for [f32; 3] {
    fn from(direction: Direction3) -> Self {
        match direction {
            Direction3::Forward => [0., 1., 0.],
            Direction3::Backward => [0., -1., 0.],
            Direction3::Left => [-1., 0., 0.],
            Direction3::Right => [1., 0., 0.],
            Direction3::Up => [0., 0., 1.],
            Direction3::Down => [0., 0., -1.],
        }
    }
}

pub struct InputHandler {
    cursor_delta: (f32, f32),
    scroll_delta: (f32, f32),
    keys_pressed: HashSet<VirtualKeyCode>,
    bindings: HashMap<Input, Action>,
}

impl InputHandler {
    pub fn handle_device(&mut self, event: DeviceEvent) -> Action {
        match event {
            DeviceEvent::MouseMotion {
                delta: (dx, dy), ..
            } => {
                self.cursor_delta = (dx as f32 * SENSITIVITY, dy as f32 * SENSITIVITY);

                self.bindings
                    .get(&Input::Motion)
                    .map(Action::clone)
                    .unwrap_or_default()
            }

            _ => Action::Nop,
        }
    }

    pub fn handle_window(&mut self, event: WindowEvent) -> Action {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(virtual_keycode),
                        state,
                        ..
                    },
                ..
            } => {
                let pressed = self.keys_pressed.contains(&virtual_keycode);

                match state {
                    ElementState::Pressed if !pressed => {
                        self.keys_pressed.insert(virtual_keycode);
                        self.bindings
                            .get(&Input::Press(virtual_keycode))
                            .map(Action::clone)
                            .unwrap_or_default()
                    }

                    ElementState::Released if pressed => {
                        self.keys_pressed.remove(&virtual_keycode);
                        self.bindings
                            .get(&Input::Release(virtual_keycode))
                            .map(Action::clone)
                            .unwrap_or_default()
                    }

                    _ => Action::Nop,
                }
            }

            WindowEvent::MouseInput { button, .. } => self
                .bindings
                .get(&Input::Button(button))
                .map(Action::clone)
                .unwrap_or_default(),

            WindowEvent::Resized(PhysicalSize { width, height }) => {
                Action::Resize { width, height }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.scroll_delta = match delta {
                    MouseScrollDelta::LineDelta(dx, dy) => (dx, dy),
                    MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => (16. * x as f32, 16. * y as f32),
                };

                self
                    .bindings
                    .get(&Input::Scroll)
                    .map(Action::clone)
                    .unwrap_or_default()
            },

            WindowEvent::Resized(PhysicalSize { width, height }) => {
                Action::Resize { width, height }
            }

            WindowEvent::Focused(true) => Action::Focus,
            WindowEvent::Focused(false) => Action::Pause,
            WindowEvent::CloseRequested => Action::ExitGame,
            _ => Action::Nop,
        }
    }

    pub fn cursor_delta(&self) -> (f32, f32) {
        self.cursor_delta
    }

    pub fn scroll_delta(&self) -> (f32, f32) {
        self.scroll_delta
    }
}

impl<const N: usize> From<[(Input, Action); N]> for InputHandler {
    fn from(bindings: [(Input, Action); N]) -> Self {
        Self {
            cursor_delta: Default::default(),
            scroll_delta: Default::default(),
            keys_pressed: HashSet::default(),
            bindings: HashMap::from(bindings),
        }
    }
}

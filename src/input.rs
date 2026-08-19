// src/input.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Task,
    Extra(u16),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MouseGesture {
    Pinch { scale: f64 },
    Rotate { angle: f64 },
    Swipe { dx: f64, dy: f64 },
    TwoFingerScroll { dx: f64, dy: f64 },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum MouseInput {
    Move { x: f64, y: f64, absolute: bool },
    ButtonDown { button: MouseButton },
    ButtonUp { button: MouseButton },
    Click { button: MouseButton },
    DoubleClick { button: MouseButton },
    Scroll { delta_x: f64, delta_y: f64 },
    Gesture(MouseGesture),
}

#[allow(dead_code)]
pub fn mouse_button_to_linux_code(button: &MouseButton) -> i32 {
    match button {
        MouseButton::Left => 0x110,
        MouseButton::Right => 0x111,
        MouseButton::Middle => 0x112,
        MouseButton::Back => 0x113,
        MouseButton::Forward => 0x114,
        MouseButton::Task => 0x117,
        MouseButton::Extra(c) => *c as i32,
    }
}

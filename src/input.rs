use serde::{Deserialize, Serialize};

#[allow(dead_code)]
pub mod loc {
    pub const MSG_MOUSE_INPUT_RECEIVED: &str = "mouse_input_received";
    pub const MSG_MOUSE_INPUT_SUCCESS: &str = "mouse_input_success";
    pub const MSG_MOUSE_INPUT_FAILED: &str = "mouse_input_failed";
    pub const MSG_PORTAL_NOT_AVAILABLE: &str = "portal_remote_desktop_not_available";
    pub const MSG_NO_ACTIVE_PORTAL: &str = "no_active_linux_portal_instance";
    pub const MSG_SEND_INPUT_FAILED: &str = "send_input_failed";
}

/// Represents mouse button types including primary, secondary, middle, side/thumb, and custom extra buttons.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,    // Side / Thumb Button 1 (X1)
    Forward, // Side / Thumb Button 2 (X2)
    Task,    // Side / Thumb Button 3
    Extra(u16),
}

/// High-level mouse gesture types.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MouseGesture {
    Pinch { scale: f64 },
    Rotate { angle: f64 },
    Swipe { dx: f64, dy: f64 },
    TwoFingerScroll { dx: f64, dy: f64 },
}

/// Inbound mouse input actions.
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

/// Maps high-level MouseButton enum variants to standard Linux event codes (`linux/input-event-codes.h`).
pub fn mouse_button_to_linux_code(button: &MouseButton) -> i32 {
    match button {
        MouseButton::Left => 0x110,    // BTN_LEFT
        MouseButton::Right => 0x111,   // BTN_RIGHT
        MouseButton::Middle => 0x112,  // BTN_MIDDLE
        MouseButton::Back => 0x113,    // BTN_SIDE
        MouseButton::Forward => 0x114, // BTN_EXTRA
        MouseButton::Task => 0x117,    // BTN_TASK
        MouseButton::Extra(c) => *c as i32,
    }
}

#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(target_os = "windows")]
static ACC_X: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "windows")]
static ACC_Y: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "windows")]
fn add_and_extract(acc: &AtomicU64, delta: f64) -> i32 {
    let mut current_bits = acc.load(Ordering::Relaxed);
    loop {
        let current = f64::from_bits(current_bits);
        let total = current + delta;
        let int_part = total.trunc();
        let rem = total - int_part;
        let new_bits = rem.to_bits();
        match acc.compare_exchange_weak(
            current_bits,
            new_bits,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return int_part as i32,
            Err(actual) => current_bits = actual,
        }
    }
}

#[cfg(target_os = "windows")]
pub fn handle_mouse_windows(input: &MouseInput) {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SetCursorPos,
    };

    unsafe {
        match input {
            MouseInput::Move { x, y, absolute } => {
                if *absolute {
                    ACC_X.store(0f64.to_bits(), Ordering::Relaxed);
                    ACC_Y.store(0f64.to_bits(), Ordering::Relaxed);

                    let sw = GetSystemMetrics(SM_CXSCREEN).max(1) as f64;
                    let sh = GetSystemMetrics(SM_CYSCREEN).max(1) as f64;

                    // Properly distinguish between normalized (0.0..1.0) and pixel coordinates
                    let (abs_x, abs_y, px, py) = if *x <= 1.0 && *y <= 1.0 && *x >= 0.0 && *y >= 0.0 && (*x > 0.0 || *y > 0.0) {
                        let norm_x = x.clamp(0.0, 1.0);
                        let norm_y = y.clamp(0.0, 1.0);
                        (norm_x, norm_y, (norm_x * (sw - 1.0)) as i32, (norm_y * (sh - 1.0)) as i32)
                    } else {
                        let norm_x = (x / sw).clamp(0.0, 1.0);
                        let norm_y = (y / sh).clamp(0.0, 1.0);
                        (norm_x, norm_y, x.clamp(0.0, sw - 1.0) as i32, y.clamp(0.0, sh - 1.0) as i32)
                    };

                    let _ = SetCursorPos(px, py);

                    let dx = (abs_x * 65535.0) as i32;
                    let dy = (abs_y * 65535.0) as i32;

                    let flags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE;

                    let input_evt = INPUT {
                        r#type: INPUT_MOUSE,
                        Anonymous: INPUT_0 {
                            mi: MOUSEINPUT {
                                dx,
                                dy,
                                mouseData: 0,
                                dwFlags: flags,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    };
                    if SendInput(&[input_evt], std::mem::size_of::<INPUT>() as i32) == 0 {
                        tracing::warn!("{}", loc::MSG_SEND_INPUT_FAILED);
                    }
                } else {
                    let dx = add_and_extract(&ACC_X, *x);
                    let dy = add_and_extract(&ACC_Y, *y);

                    if dx != 0 || dy != 0 {
                        let input_evt = INPUT {
                            r#type: INPUT_MOUSE,
                            Anonymous: INPUT_0 {
                                mi: MOUSEINPUT {
                                    dx,
                                    dy,
                                    mouseData: 0,
                                    dwFlags: MOUSEEVENTF_MOVE,
                                    time: 0,
                                    dwExtraInfo: 0,
                                },
                            },
                        };
                        if SendInput(&[input_evt], std::mem::size_of::<INPUT>() as i32) == 0 {
                            tracing::warn!("{}", loc::MSG_SEND_INPUT_FAILED);
                        }
                    }
                }
            }
            MouseInput::ButtonDown { button } => {
                let (flags, mouse_data) = match button {
                    MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, 0),
                    MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, 0),
                    MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, 0),
                    MouseButton::Back => (MOUSEEVENTF_XDOWN, 1u32),
                    MouseButton::Forward => (MOUSEEVENTF_XDOWN, 2u32),
                    MouseButton::Task => (MOUSEEVENTF_XDOWN, 2u32),
                    MouseButton::Extra(val) => (MOUSEEVENTF_XDOWN, *val as u32),
                };
                let input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouseData: mouse_data,
                            dwFlags: flags,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                if SendInput(&[input], std::mem::size_of::<INPUT>() as i32) == 0 {
                    tracing::warn!("{}", loc::MSG_SEND_INPUT_FAILED);
                }
            }
            MouseInput::ButtonUp { button } => {
                let (flags, mouse_data) = match button {
                    MouseButton::Left => (MOUSEEVENTF_LEFTUP, 0),
                    MouseButton::Right => (MOUSEEVENTF_RIGHTUP, 0),
                    MouseButton::Middle => (MOUSEEVENTF_MIDDLEUP, 0),
                    MouseButton::Back => (MOUSEEVENTF_XUP, 1u32),
                    MouseButton::Forward => (MOUSEEVENTF_XUP, 2u32),
                    MouseButton::Task => (MOUSEEVENTF_XUP, 2u32),
                    MouseButton::Extra(val) => (MOUSEEVENTF_XUP, *val as u32),
                };
                let input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx: 0,
                            dy: 0,
                            mouseData: mouse_data,
                            dwFlags: flags,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                if SendInput(&[input], std::mem::size_of::<INPUT>() as i32) == 0 {
                    tracing::warn!("{}", loc::MSG_SEND_INPUT_FAILED);
                }
            }
            MouseInput::Click { button } => {
                handle_mouse_windows(&MouseInput::ButtonDown {
                    button: button.clone(),
                });
                std::thread::sleep(std::time::Duration::from_millis(10));
                handle_mouse_windows(&MouseInput::ButtonUp {
                    button: button.clone(),
                });
            }
            MouseInput::DoubleClick { button } => {
                handle_mouse_windows(&MouseInput::Click {
                    button: button.clone(),
                });
                std::thread::sleep(std::time::Duration::from_millis(50));
                handle_mouse_windows(&MouseInput::Click {
                    button: button.clone(),
                });
            }
            MouseInput::Scroll { delta_x, delta_y } => {
                if *delta_y != 0.0 {
                    let input = INPUT {
                        r#type: INPUT_MOUSE,
                        Anonymous: INPUT_0 {
                            mi: MOUSEINPUT {
                                dx: 0,
                                dy: 0,
                                mouseData: ((*delta_y * 120.0) as i32) as u32,
                                dwFlags: MOUSEEVENTF_WHEEL,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    };
                    if SendInput(&[input], std::mem::size_of::<INPUT>() as i32) == 0 {
                        tracing::warn!("{}", loc::MSG_SEND_INPUT_FAILED);
                    }
                }
                if *delta_x != 0.0 {
                    let input = INPUT {
                        r#type: INPUT_MOUSE,
                        Anonymous: INPUT_0 {
                            mi: MOUSEINPUT {
                                dx: 0,
                                dy: 0,
                                mouseData: ((*delta_x * 120.0) as i32) as u32,
                                dwFlags: MOUSEEVENTF_HWHEEL,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    };
                    if SendInput(&[input], std::mem::size_of::<INPUT>() as i32) == 0 {
                        tracing::warn!("{}", loc::MSG_SEND_INPUT_FAILED);
                    }
                }
            }
            MouseInput::Gesture(gesture) => match gesture {
                MouseGesture::Pinch { scale } => {
                    let ctrl_down = INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: VK_CONTROL,
                                wScan: 0,
                                dwFlags: KEYBD_EVENT_FLAGS(0u32),
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    };
                    SendInput(&[ctrl_down], std::mem::size_of::<INPUT>() as i32);
                    let scroll_delta = if *scale > 1.0 { 1.0 } else { -1.0 };
                    handle_mouse_windows(&MouseInput::Scroll {
                        delta_x: 0.0,
                        delta_y: scroll_delta,
                    });
                    let ctrl_up = INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: VK_CONTROL,
                                wScan: 0,
                                dwFlags: KEYEVENTF_KEYUP,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    };
                    SendInput(&[ctrl_up], std::mem::size_of::<INPUT>() as i32);
                }
                MouseGesture::TwoFingerScroll { dx, dy } => {
                    handle_mouse_windows(&MouseInput::Scroll {
                        delta_x: *dx,
                        delta_y: *dy,
                    });
                }
                MouseGesture::Swipe { dx, dy } => {
                    handle_mouse_windows(&MouseInput::Move {
                        x: *dx,
                        y: *dy,
                        absolute: false,
                    });
                }
                MouseGesture::Rotate { angle } => {
                    let scroll_delta = if *angle > 0.0 { 1.0 } else { -1.0 };
                    handle_mouse_windows(&MouseInput::Scroll {
                        delta_x: scroll_delta,
                        delta_y: 0.0,
                    });
                }
            },
        }
    }
}
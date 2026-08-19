// src/platform/windows/input.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use std::sync::atomic::{AtomicU64, Ordering};
use tracing::info;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SetCursorPos,
};

use crate::input::{MouseButton, MouseGesture, MouseInput};
use crate::loc;

static ACC_X: AtomicU64 = AtomicU64::new(0);
static ACC_Y: AtomicU64 = AtomicU64::new(0);

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

pub fn handle_mouse_windows(input: &MouseInput) {
    info!("{}: {:?}", loc::MSG_EXECUTING_WIN32_MOUSE, input);

    unsafe {
        match input {
            MouseInput::Move { x, y, absolute } => {
                if *absolute {
                    ACC_X.store(0f64.to_bits(), Ordering::Relaxed);
                    ACC_Y.store(0f64.to_bits(), Ordering::Relaxed);

                    let sw = GetSystemMetrics(SM_CXSCREEN).max(1) as f64;
                    let sh = GetSystemMetrics(SM_CYSCREEN).max(1) as f64;

                    let (abs_x, abs_y, px, py) = if *x <= 1.0 && *y <= 1.0 && *x >= 0.0 && *y >= 0.0
                    {
                        let norm_x = x.clamp(0.0, 1.0);
                        let norm_y = y.clamp(0.0, 1.0);
                        (
                            norm_x,
                            norm_y,
                            (norm_x * (sw - 1.0)) as i32,
                            (norm_y * (sh - 1.0)) as i32,
                        )
                    } else {
                        let norm_x = (x / sw).clamp(0.0, 1.0);
                        let norm_y = (y / sh).clamp(0.0, 1.0);
                        (
                            norm_x,
                            norm_y,
                            x.clamp(0.0, sw - 1.0) as i32,
                            y.clamp(0.0, sh - 1.0) as i32,
                        )
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

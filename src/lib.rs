slint::include_modules!();
use i_slint_backend_winit::WinitWindowAccessor;
use slint::Window;
use std::ffi::c_void;
use std::mem::size_of;
use tracing::warn;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWM_WINDOW_CORNER_PREFERENCE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
};
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCLIENT, HTLEFT, HTRIGHT, HTTOP,
    HTTOPLEFT, HTTOPRIGHT, WM_NCCALCSIZE, WM_NCHITTEST,
};

pub struct WindowFrame<T: slint::ComponentHandle + 'static> {
    weak: slint::Weak<T>,
}

impl<T: slint::ComponentHandle + 'static> Clone for WindowFrame<T> {
    fn clone(&self) -> Self {
        Self {
            weak: self.weak.clone(),
        }
    }
}

impl<T: slint::ComponentHandle + 'static> WindowFrame<T> {
    const BORDER_WIDTH: i32 = 8;
    const SUBCLASS_ID: usize = 1;

    fn new(component: &T) -> Self {
        Self {
            weak: component.as_weak(),
        }
    }

    fn with_window<R>(&self, f: impl FnOnce(&Window) -> R) -> Option<R> {
        self.weak.upgrade().map(|c| f(c.window()))
    }

    pub fn maximize(&self, is_maximized: bool) {
        self.with_window(|w| w.set_maximized(is_maximized));
    }
    pub fn toggle_maximized(&self) {
        self.with_window(|w| w.set_maximized(!w.is_maximized()));
    }
    pub fn minimize(&self) {
        self.with_window(|w| w.set_minimized(true));
    }
    pub fn close(&self) {
        slint::quit_event_loop().expect("Failed to quit event loop");
    }
    pub fn drag(&self) {
        self.with_winit_window(|window| {
            let _ = window.drag_window();
        });
    }

    fn with_winit_window<R>(&self, f: impl FnOnce(&winit::window::Window) -> R) -> Option<R> {
        self.weak.upgrade().and_then(|c| {
            c.window().with_winit_window(|w| f(w))
        })
    }

    /// Applies Windows 11 custom frame styling to a winit window.
    ///
    /// This enables DWM rounded corners, a drop shadow, and installs a
    /// window subclass for edge-resize hit testing.
    fn apply(&self) {
        self.with_winit_window(|window| {
            let Some(hwnd) = Self::get_hwnd(window) else {
                warn!("Failed to extract HWND from winit window");
                return;
            };
            Self::apply_rounded_corners(hwnd);
            Self::apply_drop_shadow(hwnd);
            Self::install_custom_frame(hwnd);
        });
    }

    fn get_hwnd(window: &winit::window::Window) -> Option<HWND> {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let handle = window.window_handle().ok()?;
        match handle.as_raw() {
            RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as *mut c_void)),
            _ => None,
        }
    }

    fn apply_rounded_corners(hwnd: HWND) {
        let preference = DWMWCP_ROUND;
        unsafe {
            if let Err(e) = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &preference as *const DWM_WINDOW_CORNER_PREFERENCE as *const c_void,
                size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
            ) {
                warn!("DwmSetWindowAttribute (rounded corners) failed: {e}");
            }
        }
    }

    fn apply_drop_shadow(hwnd: HWND) {
        let margins = MARGINS {
            cxLeftWidth: 0,
            cxRightWidth: 0,
            cyTopHeight: 0,
            cyBottomHeight: 1,
        };
        unsafe {
            if let Err(e) = DwmExtendFrameIntoClientArea(hwnd, &margins) {
                warn!("DwmExtendFrameIntoClientArea (drop shadow) failed: {e}");
            }
        }
    }

    fn install_custom_frame(hwnd: HWND) {
        unsafe {
            if !SetWindowSubclass(hwnd, Some(Self::custom_frame_proc), Self::SUBCLASS_ID, 0)
                .as_bool()
            {
                warn!("SetWindowSubclass (custom frame) failed");
            }
        }
    }

    // SAFETY: This callback is registered via SetWindowSubclass and invoked by the
    // Windows message loop with a valid hwnd. lparam encodes screen coordinates as
    // (x | (y << 16)) per the WM_NCHITTEST convention.
    unsafe extern "system" fn custom_frame_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _uid_subclass: usize,
        _ref_data: usize,
    ) -> LRESULT {
        match msg {
            WM_NCCALCSIZE if wparam.0 != 0 => LRESULT(0),
            WM_NCHITTEST => {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

                let mut rect = RECT::default();
                if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
                    return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
                }

                let left = x - rect.left < Self::BORDER_WIDTH;
                let right = rect.right - x <= Self::BORDER_WIDTH;
                let top = y - rect.top < Self::BORDER_WIDTH;
                let bottom = rect.bottom - y <= Self::BORDER_WIDTH;

                let hit = if top && left {
                    HTTOPLEFT
                } else if top && right {
                    HTTOPRIGHT
                } else if bottom && left {
                    HTBOTTOMLEFT
                } else if bottom && right {
                    HTBOTTOMRIGHT
                } else if top {
                    HTTOP
                } else if bottom {
                    HTBOTTOM
                } else if left {
                    HTLEFT
                } else if right {
                    HTRIGHT
                } else {
                    HTCLIENT
                };

                LRESULT(hit as isize)
            }
            _ => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },
        }
    }
}

pub trait TitlebarSetup<T: slint::ComponentHandle> {
    fn setup_borderless(&self) -> Result<WindowFrame<T>, slint::PlatformError>;
}

impl<T: slint::ComponentHandle + 'static> TitlebarSetup<T> for slint::Weak<T> {
    fn setup_borderless(&self) -> Result<WindowFrame<T>, slint::PlatformError> {
        self.upgrade_in_event_loop(|win|{
            let frame = WindowFrame::new(&win);
            frame.apply();
        }).expect("Failed to upgrade window");
        let component = self.upgrade().ok_or_else(|| {
            slint::PlatformError::Other("Failed to upgrade component handle".to_string())
        })?;
        let frame = WindowFrame::new(&component);
        frame.apply();
        Ok(frame)
    }
}

#![doc = include_str!("../README.md")]

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

    /// Sets the maximized state of the window.
    ///
    /// This function toggles the maximized state of the window based on the
    /// `is_maximized` parameter. If `is_maximized` is `true`, the window will
    /// be maximized; if `false`, the window will be restored to its normal state.
    ///
    /// # Parameters
    /// - `is_maximized`: A boolean indicating the desired maximized state of the window.
    ///   - `true`: Maximizes the window.
    ///   - `false`: Restores the window to its normal size.
    ///
    /// # Example
    /// ```
    /// some_object.maximize(true); // Maximizes the window
    /// some_object.maximize(false); // Restores the window to its original size
    /// ```
    ///
    /// # Implementation Details
    /// Internally, this function uses the `with_window` method to obtain the
    /// current window instance and calls `set_maximized` on it with the value
    /// of `is_maximized`.
    pub fn maximize(&self, is_maximized: bool) {
        self.with_window(|w| w.set_maximized(is_maximized));
    }
    /// Toggles the maximized state of the window.
    ///
    /// This function inverts the current maximized state of the window.
    /// If the window is currently maximized, it will be restored to its normal size.
    /// Conversely, if the window is currently in its normal state, it will be maximized.
    ///
    /// # Example
    /// ```rust
    /// // Assuming `self` is an instance with the `toggle_maximized` method
    /// self.toggle_maximized();
    /// ```
    ///
    /// # Notes
    /// - This method relies on the `with_window` method to access the underlying window object.
    /// - The `set_maximized` method is used to update the maximized state, and the
    ///   current state is determined by the `is_maximized` method.
    ///
    /// # Panics
    /// This method may panic if `with_window` fails to provide access to a valid window object.
    pub fn toggle_maximized(&self) {
        self.with_window(|w| w.set_maximized(!w.is_maximized()));
    }
    /// Minimizes the window associated with the current instance.
    ///
    /// This method utilizes the `with_window` function to access the underlying window
    /// and sets its `minimized` state to `true`, effectively minimizing the window on the screen.
    ///
    /// # Example
    /// ```rust
    /// my_window_instance.minimize();
    /// ```
    ///
    /// # Note
    /// Ensure that the instance has a valid window context before calling this method
    /// to avoid unexpected behavior.
    pub fn minimize(&self) {
        self.with_window(|w| w.set_minimized(true));
    }
    /// Closes the application by terminating the event loop.
    ///
    /// This function calls `slint::quit_event_loop()` to stop the active event loop.
    /// It should be used to gracefully shut down the application when it is no longer needed.
    ///
    /// # Panics
    /// If the event loop fails to quit, this function will panic with the message
    /// `"Failed to quit event loop"`.
    ///
    /// # Example
    /// ```rust
    /// my_application.close();
    /// ```
    ///
    /// Ensure that this method is called when appropriate to avoid unnecessary panics.
    pub fn close(&self) {
        slint::quit_event_loop().expect("Failed to quit event loop");
    }
    /// Initiates a drag operation for the current window.
    ///
    /// This method triggers the drag functionality of the window,
    /// allowing the user to click and drag the window across the screen.
    ///
    /// # Implementation Details
    /// - Internally, the method uses the `winit` crate to access the window instance
    ///   and call its `drag_window` method.
    /// - If the drag operation fails (e.g., if the platform does not support it),
    ///   the error is ignored.
    ///
    /// # Usage
    /// ```rust
    /// // Assuming `self` is an instance with access to this method:
    /// self.drag();
    /// ```
    ///
    /// # Caveats
    /// - Platform-specific behavior: Dragging may not be available or behave
    ///   differently on certain operating systems.
    /// - Silent failure: If the drag operation fails, it will not propagate an error.
    ///
    /// # Dependencies
    /// Requires the `winit` crate for window handling.
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
    /// Sets up a borderless window frame for rendering.
    ///
    /// This function configures the window to operate without a standard border,
    /// which is particularly useful for custom drawing or specialized window designs.
    ///
    /// # Returns
    ///
    /// * `Ok(WindowFrame<T>)` - If the borderless window setup is successful, an instance
    ///   of `WindowFrame` is returned.
    /// * `Err(slint::PlatformError)` - If an error occurs during the setup process, a
    ///   platform-specific `PlatformError` is returned.
    ///
    /// # Errors
    ///
    /// This method will return an error if the platform does not support borderless
    /// window frames or if there is a failure in the window setup process.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let window_frame = my_window.setup_borderless()?;
    /// // Use `window_frame` for further customization or operations.
    /// ```
    ///
    /// # Note
    /// Ensure that the environment in which the application is running supports borderless
    /// window configurations. This function might rely on platform-specific APIs or extensions.
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

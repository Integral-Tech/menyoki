use crate::util::device::DeviceState;
use crate::x11::window::Window;
use std::mem::MaybeUninit;
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};
use x11::xlib;

/* Timeout value for the window selection */
const SELECT_WINDOW_TIMEOUT: u128 = 30 * 1000;
/* Time interval between focused window checks */
const SELECTION_INTERVAL: u64 = 10;

/* X11 display */
pub struct Display {
	display: *mut xlib::Display,
}

/* Implementation for thread-safe usage */
unsafe impl Send for Display {}

impl Display {
	/**
	 * Open a display.
	 *
	 * @return Display (Option)
	 */
	pub fn open() -> Option<Self> {
		let display = unsafe { xlib::XOpenDisplay(ptr::null()) };
		if !display.is_null() {
			Some(Self { display })
		} else {
			None
		}
	}

	/**
	 * Get the default screen of display.
	 *
	 * @return Screen
	 */
	pub unsafe fn get_default_screen(&self) -> *mut xlib::Screen {
		xlib::XDefaultScreenOfDisplay(self.display)
	}

	/**
	 * Get the root window of the default screen.
	 *
	 * @return Window
	 */
	pub fn get_root_window(&self) -> Window {
		unsafe {
			Window::new(
				xlib::XRootWindowOfScreen(self.get_default_screen()),
				self.display,
			)
		}
	}

	/**
	 * Get the focused window.
	 *
	 * @return Window
	 */
	pub fn get_focused_window(&self) -> Option<Window> {
		unsafe {
			let mut focus_window = MaybeUninit::<u64>::uninit();
			let mut focus_state = MaybeUninit::<i32>::uninit();
			xlib::XGetInputFocus(
				self.display,
				focus_window.as_mut_ptr(),
				focus_state.as_mut_ptr(),
			);
			if focus_state.assume_init() != xlib::RevertToNone {
				Some(Window::new(*focus_window.as_ptr(), self.display))
			} else {
				None
			}
		}
	}

	/**
	 * Select a Window from the display with the user interaction.
	 *
	 * @param  device_state
	 * @param  fg_color
	 * @return Window (Option)
	 */
	pub fn select_window(
		&self,
		device_state: &mut DeviceState,
		fg_color: u64,
	) -> Option<Window> {
		let mut focused_window = self
			.get_focused_window()
			.expect("Failed to get the focused window");
		let mut xid = 0;
		let mut selection_canceled = false;
		let now = Instant::now();
		while !(device_state.mouse_clicked
			|| device_state.exit_keys_pressed
			|| selection_canceled)
		{
			focused_window = self
				.get_focused_window()
				.expect("Failed to get the focused window");
			focused_window.draw_borders(fg_color, 5);
			device_state.update();
			if device_state.exit_keys_pressed {
				warn!("User interrupt detected. Have a good day!");
				selection_canceled = true;
			} else if now.elapsed().as_millis() > SELECT_WINDOW_TIMEOUT {
				warn!("The operation timed out. Have a good day!");
				selection_canceled = true;
			} else if xid != focused_window.xid {
				debug!("Window ID: {:?}", focused_window.xid);
				info!("{}", focused_window);
				xid = focused_window.xid;
			}
			thread::sleep(Duration::from_millis(SELECTION_INTERVAL));
		}
		focused_window.clear_area();
		if !selection_canceled {
			Some(focused_window)
		} else {
			None
		}
	}
}

/* Close the display when Display object went out of scope. */
impl Drop for Display {
	fn drop(&mut self) {
		unsafe {
			xlib::XCloseDisplay(self.display);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_display_mod() {
		let display = Display::open().unwrap();
		assert!(display.get_root_window().xid > 0);
		assert!(display.get_focused_window().is_none());
	}
}

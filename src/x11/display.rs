use crate::image::geometry::Geometry;
use crate::record::fps::FpsClock;
use crate::record::settings::{RecordSettings, RecordWindow};
use crate::util::state::InputState;
use crate::x11::window::Window;
use device_query::{DeviceQuery, Keycode};
use std::ffi::CString;
use std::io::{self, Write};
use std::mem::MaybeUninit;
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};
use x11::xlib;

/* X11 display */
pub struct Display {
	pub display: *mut xlib::Display,
	settings: RecordSettings,
}

/* Implementation for thread-safe usage */
unsafe impl Send for Display {}

impl Display {
	/**
	 * Open a display.
	 *
	 * @param  settings (Option)
	 * @return Display  (Option)
	 */
	pub fn open(settings: Option<RecordSettings>) -> Option<Self> {
		let display = unsafe { xlib::XOpenDisplay(ptr::null()) };
		if !display.is_null() {
			Some(Self {
				display,
				settings: settings.unwrap_or_default(),
			})
		} else {
			None
		}
	}

	/**
	 * Get the root window of the default screen.
	 *
	 * @return Window
	 */
	pub fn get_root_window(&self) -> Window {
		unsafe {
			Window::new(
				xlib::XRootWindowOfScreen(xlib::XDefaultScreenOfDisplay(
					self.display,
				)),
				self.display,
				self.settings,
			)
		}
	}

	/**
	 * Get the focused window.
	 *
	 * @return Window (Option)
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
				Some(Window::new(
					*focus_window.as_ptr(),
					self.display,
					self.settings,
				))
			} else {
				None
			}
		}
	}

	/**
	 * Set the focused window.
	 *
	 * @param  xid
	 * @param  focus_state
	 */
	#[allow(dead_code)]
	pub fn set_focused_window(&self, xid: u64, focus_state: i32) {
		unsafe {
			xlib::XSetInputFocus(self.display, xid, focus_state, xlib::CurrentTime)
		};
	}

	/**
	 * Get the type of Window given with RecordWindow enum.
	 *
	 * @return Tuple (Window, Geometry)
	 */
	fn get_window(&self) -> (Window, Geometry) {
		match self.settings.window {
			RecordWindow::Focus(geometry) => (
				self.get_focused_window().expect("Failed to get the window"),
				geometry.unwrap_or_default(),
			),
			RecordWindow::Root(geometry) => {
				(self.get_root_window(), geometry.unwrap_or_default())
			}
		}
	}

	/**
	 * Get the corresponding key symbol from keycode.
	 *
	 * @param  keycode
	 * @return u64
	 */
	fn get_symbol_from_keycode(&self, keycode: &Keycode) -> u64 {
		let mut key = format!("{:?}", keycode)
			.trim_start_matches("Key")
			.to_string();
		if (key.starts_with('L') | key.starts_with('R')) && key.len() > 3 {
			key = format!(
				"{}_{}",
				key.chars()
					.next()
					.map(|c| &key[c.len_utf8()..])
					.unwrap_or_default(),
				key.chars().next().unwrap_or_default()
			);
		}
		let key = CString::new(key).expect("Failed to create CString");
		unsafe { xlib::XStringToKeysym(key.as_ptr()) }
	}

	/**
	 * Ungrab the keys in the given window.
	 *
	 * @param xid (Option)
	 */
	fn ungrab_keys(&self, xid: Option<u64>) {
		if let Some(window) = xid {
			unsafe {
				xlib::XUngrabKey(
					self.display,
					xlib::AnyKey,
					xlib::AnyModifier,
					window,
				);
			}
			trace!("Ungrabbed the keys of {:?}", xid);
		}
	}

	/**
	 * Select a Window from display with user interaction.
	 *
	 * @param  input_state
	 * @return Window (Option)
	 */
	pub fn select_window(&mut self, input_state: &InputState) -> Option<Window> {
		let (mut window, size) = self.get_window();
		let mut xid = None;
		let window_padding = self.settings.padding;
		let mut change_factor = 3;
		let font_context =
			textwidth::Context::with_misc().expect("Failed to create font context");
		let start_time = Instant::now();
		while !input_state.check_action_keys() {
			window = self.get_window().0;
			window.draw_borders();
			window.show_text_centered(Some(window.area.to_string()), &font_context);
			let reset_area =
				self.update_area(window, input_state, &mut change_factor);
			if input_state.check_cancel_keys() {
				warn!("User interrupt detected.");
				xid = None;
				break;
			} else if start_time.elapsed().as_secs() > self.settings.time.timeout {
				warn!("The operation timed out.");
				xid = None;
				break;
			} else if xid != Some(window.xid) || reset_area {
				if !reset_area {
					debug!("Window ID: {}", window.xid);
					info!("{}", window);
				}
				self.ungrab_keys(xid);
				self.settings.padding = window_padding;
				self.update_padding(size, window.geometry);
				window.clear_area();
				window.grab_key(
					self.get_symbol_from_keycode(&input_state.action_keys.main_key),
				);
				xid = Some(window.xid);
			}
			thread::sleep(Duration::from_millis(self.settings.time.interval));
		}
		trace!("{:?}", input_state);
		debug!("Selected window: {:?}", xid);
		self.ungrab_keys(xid);
		if self.settings.border.is_some() {
			window.clear_area();
			window.show_text(Some(String::from(" ")), FpsClock::new(500));
		}
		if xid.is_some() {
			Some(window)
		} else {
			None
		}
	}

	/**
	 * Update padding to set the given width and height.
	 *
	 * @param size
	 * @param window_geometry
	 */
	fn update_padding(&mut self, size: Geometry, window_geometry: Geometry) {
		if !size.is_zero() {
			self.settings.padding.top = 0;
			self.settings.padding.right = window_geometry
				.width
				.checked_sub(size.width)
				.unwrap_or_default();
			self.settings.padding.bottom = window_geometry
				.height
				.checked_sub(size.height)
				.unwrap_or_default();
			self.settings.padding.left = 0;
		}
	}

	/**
	 * Update the recording area on associated key presses.
	 *
	 * @param  window
	 * @param  input_state
	 * @param  change
	 * @return bool
	 */
	fn update_area(
		&mut self,
		window: Window,
		input_state: &InputState,
		change: &mut u32,
	) -> bool {
		let mut reset_area = false;
		let modifiers = self.settings.padding.get_modifiers();
		for (value, increase, decrease) in modifiers {
			match input_state.state.get_keys().as_slice() {
				[Keycode::R, Keycode::LAlt] => reset_area = true,
				[Keycode::LAlt, key] | [key, Keycode::LAlt] => {
					if (key == &increase[0] || key == &increase[1])
						&& (window.area.height > 5 && window.area.width > 5)
					{
						*value = value.checked_add(*change).unwrap_or(*value);
						window.clear_area();
					} else {
						let key = format!("{:?}", key);
						if key.contains("Key") {
							*change = key
								.trim_start_matches("Key")
								.parse::<u32>()
								.unwrap_or(*change);
						}
					}
				}
				[Keycode::LControl, Keycode::LAlt, key]
				| [Keycode::LControl, key, Keycode::LAlt] => {
					if key == &decrease[0] || key == &decrease[1] {
						*value = value.checked_sub(*change).unwrap_or(*value);
						window.clear_area();
					}
				}
				[Keycode::LShift, Keycode::LAlt, key]
				| [key, Keycode::LShift, Keycode::LAlt] => {
					if (key == &increase[0] || key == &increase[1])
						&& (window.area.height > 10 && window.area.width > 10)
					{
						*value = value.checked_add(*change).unwrap_or(*value);
						window.clear_area();
					}
					if (key == &decrease[0] || key == &decrease[1])
						&& (window.area.height > 10 && window.area.width > 10)
					{
						*value = value.checked_sub(*change).unwrap_or(*value);
						window.clear_area();
					}
				}
				_ => {}
			}
		}
		info!(
			" Selected area -> [{}x{}] {}\r#",
			window.area.width,
			window.area.height,
			if window.settings.padding.is_zero() {
				String::new()
			} else {
				format!("p:[{}]{:<10}", window.settings.padding, " ")
			},
		);
		io::stdout().flush().expect("Failed to flush stdout");
		reset_area
	}
}

#[cfg(test)]
#[cfg(feature = "test_ws")]
mod tests {
	use super::*;
	use crate::record::settings::RecordTime;
	use crate::record::Record;
	use pretty_assertions::assert_eq;
	use std::convert::TryFrom;
	use x11::keysym;
	#[test]
	fn test_x11_display() {
		let mut settings = RecordSettings::default();
		settings.time = RecordTime::new(Some(0.0), 0, 0, 10);
		let mut display = Display::open(Some(settings)).unwrap();
		display
			.set_focused_window(display.get_root_window().xid, xlib::RevertToParent);
		display.update_padding(Geometry::new(0, 0, 10, 10), Geometry::default());
		assert_eq!(
			display.get_root_window().xid,
			display.get_focused_window().unwrap().xid
		);
		let input_state = InputState::default();
		assert!(display.select_window(&input_state).is_none());
		assert_eq!(
			u64::try_from(keysym::XK_Alt_L).unwrap(),
			display.get_symbol_from_keycode(&input_state.action_keys.main_key)
		);
		assert_eq!(
			u64::try_from(keysym::XK_Control_R).unwrap(),
			display.get_symbol_from_keycode(&Keycode::RControl)
		);
		assert_eq!(
			u64::try_from(keysym::XK_X).unwrap(),
			display.get_symbol_from_keycode(&Keycode::X)
		);
		display.get_root_window().release();
	}
}

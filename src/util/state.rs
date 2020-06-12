use device_query::{DeviceQuery, DeviceState, Keycode};
use std::fmt;

/* State of the mouse and keyboard inputs */
pub struct InputState {
	state: DeviceState,
}

/* Debug implementation for programmer-facing output */
impl fmt::Debug for InputState {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("InputState")
			.field("mouse", &self.state.get_mouse())
			.field("keys", &self.state.get_keys())
			.finish()
	}
}

impl InputState {
	/**
	 * Create a new InputState object.
	 *
	 * @return InputState
	 */
	pub fn new() -> Self {
		Self {
			state: DeviceState::new(),
		}
	}

	/**
	 * Check if the mouse left/right buttons are clicked.
	 *
	 * @return bool
	 */
	pub fn check_mouse(&self) -> bool {
		let mouse = self.state.get_mouse().button_pressed;
		mouse[1] || mouse[3]
	}

	/**
	 * Check if the cancel keys are pressed.
	 *
	 * @return bool
	 */
	pub fn check_keys(&self) -> bool {
		let keys = self.state.get_keys();
		keys.contains(&Keycode::Escape)
			|| (keys.contains(&Keycode::LControl) && keys.contains(&Keycode::D))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_input_state() {
		let input_state = InputState::new();
		assert!(!input_state.check_mouse());
		assert!(!input_state.check_keys());
	}
}
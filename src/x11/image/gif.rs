use crate::x11::image::Image;
use crate::x11::window::Rect;
use gif::{Encoder, Frame, Repeat, SetParameter};

pub struct Gif<'a> {
	pub rect: Rect,
	pub frames: Vec<Frame<'a>>,
	pub speed: i32,
}

impl Gif<'_> {
	pub fn new(rect: Rect, speed: i32) -> Self {
		Self {
			rect,
			frames: Vec::new(),
			speed,
		}
	}
	pub fn add_frame(&mut self, image: Image) {
		self.frames.push(Frame::from_rgb_speed(
			self.rect.width as u16,
			self.rect.height as u16,
			&image.data,
			30,
		))
	}
}
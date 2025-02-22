use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct Skills {
	pub woodworking: f32,
	pub metalworking: f32,
	pub masonry: f32,
	pub cooking: f32,
	pub construction: f32,
	pub science: f32,
	pub engineering: f32,
	pub writing: f32,
	pub research: f32,
}

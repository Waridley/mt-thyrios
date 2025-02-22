use crate::game::tools::{Tool, ReflectTool};
use bevy::prelude::*;

pub struct RoadsPlugin;

impl Plugin for RoadsPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<BuildRoads>();
	}
}

#[derive(Debug, Reflect)]
#[reflect(Tool)]
pub struct BuildRoads {}

impl Tool for BuildRoads {}

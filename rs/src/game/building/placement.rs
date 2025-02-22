use crate::game::tools::{Tool, ReflectTool};
use bevy::prelude::*;

pub struct PlacementPlugin;

impl Plugin for PlacementPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<PlaceBuilding>();
	}
}

#[derive(Debug, Reflect)]
#[reflect(Tool)]
pub struct PlaceBuilding {}

impl Tool for PlaceBuilding {}

use bevy::prelude::*;

pub mod kinds;
pub mod placement;
pub mod roads;

pub struct BuildingPlugin;

impl Plugin for BuildingPlugin {
	fn build(&self, app: &mut App) {}
}

use bevy::core_pipeline::bloom::Bloom;
use bevy::prelude::*;
use super::mtn::{spawn_mountain, Mountain};
use crate::state::GlobalState;

pub struct GameCamPlugin;

impl Plugin for GameCamPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(OnEnter(GlobalState::InGame), setup_cam.after(spawn_mountain));
	}
}

pub fn setup_cam(
	mut cmds: Commands,
	mountain: Single<Entity, With<Mountain>>,
) {
	cmds
		.entity(*mountain)
		.with_children(|mut cmds| {
			cmds.spawn((
				CamAnchor,
				Transform::default(),
				StateScoped(GlobalState::InGame),
			))
				.with_child((
					Camera3d::default(),
					Camera {
						clear_color: ClearColorConfig::Custom(Color::BLACK),
						..default()
					},
					Bloom::default(),
					Transform {
						translation: Vec3::new(0.0, -20.0, 20.0),
						rotation: Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::new(0.0, 1.0, -1.0).normalize()),
						..default()
					},
				));
		});
}

#[derive(Component, Debug)]
pub struct CamAnchor;

use super::mtn::{spawn_mountain, Mountain};
use crate::state::GlobalState::{self, InGame};
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::prelude::*;

pub struct GameCamPlugin;

impl Plugin for GameCamPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(OnEnter(InGame), setup_cam.after(spawn_mountain));
	}
}

pub fn setup_cam(mut cmds: Commands, mountain: Single<Entity, With<Mountain>>) {
	cmds.entity(*mountain).with_children(|cmds| {
		cmds.spawn((CamAnchor, Transform::default(), Visibility::default()))
			.with_children(|cmds| {
				let rotation =
					Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::new(0.0, 1.0, -1.0).normalize());
				cmds.spawn((
					Camera3d::default(),
					Camera {
						clear_color: ClearColorConfig::Custom(Color::BLACK),
						..default()
					},
					Bloom::default(),
					Transform {
						translation: Vec3::new(0.0, -100.0, 100.0),
						rotation,
						..default()
					},
					DepthPrepass,
				));
				// cmds.spawn((
				// 	SpotLight {
				// 		intensity: 2_000_000_000.0,
				// 		range: 1200.0 * 1200.0,
				// 		..default()
				// 	},
				// 	Transform {
				// 		translation: Vec3::new(0.0, -600.0, 600.0),
				// 		rotation,
				// 		..default()
				// 	},
				// ));
			});
	});
}

#[derive(Component, Debug)]
#[require(StateScoped<GlobalState>(|| StateScoped(InGame)))]
pub struct CamAnchor;

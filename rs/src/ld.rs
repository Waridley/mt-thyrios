use crate::game::GameSetupKey;
use crate::setup_tracking::{SetupKey, SetupTracker};
use crate::state::GlobalState::LoadingGame;
use bevy::prelude::*;
use bevy_egui::PrimaryEguiContext;

pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(OnEnter(LoadingGame), spawn_loading_screen)
			.add_systems(
				Update,
				(update_progress_bar::<GameSetupKey>, update_spinner).run_if(in_state(LoadingGame)),
			);
		
		#[cfg(feature = "dev_tools")]
		{
			app.add_systems(OnExit(LoadingGame), |mut cmds: Commands| {
				cmds.remove_resource::<crate::dev_tools::setup_graph_vis::SetupGraphVisState<GameSetupKey>>();
			});
		}
	}
}

pub fn spawn_loading_screen(
	mut cmds: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mats: ResMut<Assets<ColorMaterial>>,
) {
	// cmds.spawn((Camera2d, PrimaryEguiContext, StateScoped(LoadingGame)));
	let mat = mats.add(ColorMaterial::default());
	cmds.spawn((
		LoadingSpinner,
		Mesh2d(meshes.add(Rectangle::new(5.0, 30.0).mesh())),
		MeshMaterial2d(mat.clone()),
		StateScoped(LoadingGame),
	));
	cmds.spawn((
		LoadingProgressBar,
		Transform {
			translation: Vec3::new(-480.0, -240.0, 0.0),
			..default()
		},
		Mesh2d(meshes.add(Circle::new(20.0).mesh())),
		MeshMaterial2d(mat),
		StateScoped(LoadingGame),
	));
}

pub fn update_progress_bar<K: SetupKey>(
	mut bar: Single<&mut Transform, With<LoadingProgressBar>>,
	tracker: Res<SetupTracker<K>>,
) {
	bar.translation.x = (*tracker.last_progress() * 960.0) - 480.0;
}

pub fn update_spinner(mut spinner: Single<&mut Transform, With<LoadingSpinner>>, t: Res<Time>) {
	spinner.rotation *= Quat::from_axis_angle(Vec3::Z, t.delta_secs() * 6.0);
}

#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub struct LoadingProgressBar;

#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub struct LoadingSpinner;

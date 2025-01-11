use crate::state::GlobalState;
use bevy::prelude::EaseFunction::Linear;
use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::time::TimerMode::Once;
use std::time::Duration;

pub struct SplashPlugin;

impl Plugin for SplashPlugin {
	fn build(&self, app: &mut App) {
		app.add_sub_state::<SplashState>()
			.add_systems(OnEnter(GlobalState::Splash), spawn_splash)
			.enable_state_scoped_entities::<GlobalState>()
			.add_systems(Update, fade_splash.run_if(in_state(GlobalState::Splash)))
			.add_systems(OnExit(GlobalState::Splash), cleanup_splash_resources);
	}
}

pub fn spawn_splash(mut cmds: Commands, asset_server: Res<AssetServer>) {
	cmds.insert_resource(SplashTimer(Timer::new(Duration::from_secs(2), Once)));
	cmds.spawn((
		Camera2d,
		Camera {
			clear_color: ClearColorConfig::Custom(Color::BLACK),
			..default()
		},
		OrthographicProjection {
			scaling_mode: ScalingMode::AutoMin {
				min_width: 1920.0,
				min_height: 1080.0,
			},
			..OrthographicProjection::default_2d()
		},
		StateScoped(GlobalState::Splash),
	));
	let logo_handle = asset_server.load("splashes/sonday_logo.png");
	cmds.spawn((
		Sprite::from_image(logo_handle),
		StateScoped(GlobalState::Splash),
	));
}

pub fn cleanup_splash_resources(mut cmds: Commands) {
	cmds.remove_resource::<SplashTimer>();
}

pub fn fade_splash(
	mut splash_timer: ResMut<SplashTimer>,
	mut q: Single<&mut Sprite>,
	mut next_global_state: ResMut<NextState<GlobalState>>,
	splash_state: Res<State<SplashState>>,
	mut next_splash_state: ResMut<NextState<SplashState>>,
	asset_server: Res<AssetServer>,
	t: Res<Time>,
) {
	splash_timer.tick(t.delta());
	let progress = splash_timer.elapsed_secs() / splash_timer.duration().as_secs_f32();
	let curve = EasingCurve::new(0.0, 1.0, Linear)
		.reparametrize_linear(Interval::new(0.0, 0.2).unwrap())
		.unwrap()
		.chain(ConstantCurve::new(Interval::new(0.2, 0.8).unwrap(), 1.0))
		.unwrap()
		.chain(
			EasingCurve::new(1.0, 0.0, Linear)
				.reparametrize_linear(Interval::new(0.8, 1.0).unwrap())
				.unwrap(),
		)
		.unwrap();
	let fade = curve.sample(progress).expect("domain is `EVERYWHERE`");
	q.color.set_alpha(fade);
	if splash_timer.just_finished() {
		match splash_state.get() {
			SplashState::SondayLogo => {
				q.image = asset_server.load("splashes/bevy_logo.png");
				splash_timer.reset();
				next_splash_state.set(SplashState::BevyLogo);
			}
			SplashState::BevyLogo => next_global_state.set(GlobalState::MainMenu),
		}
	}
}

#[derive(Resource, Debug, Deref, DerefMut)]
pub struct SplashTimer(Timer);

#[derive(SubStates, Debug, Default, Clone, PartialEq, Eq, Hash)]
#[source(GlobalState = GlobalState::Splash)]
pub enum SplashState {
	#[default]
	SondayLogo,
	BevyLogo,
}

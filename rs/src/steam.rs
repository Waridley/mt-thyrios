use bevy::prelude::*;
use bevy_steamworks::{SteamAPIInitError, SteamworksPlugin};

pub struct SteamIntegrationPlugin;

impl Plugin for SteamIntegrationPlugin {
	fn build(&self, app: &mut App) {
		match SteamworksPlugin::init() {
			Ok(plugin) => {
				app.add_plugins(plugin);
			}
			Err(e) => error!("Couldn't init SteamworksPlugin: {e}"),
		}
	}
}

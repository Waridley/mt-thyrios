use crate::game::GameSetupKey;
use crate::setup_tracking::{SetupKey, SetupTracker};
use crate::ui::egui::Ui;
use bevy::ecs::system::SystemId;
use bevy::platform_support::collections::HashMap;
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use bevy_egui::egui::Color32;
use egui_snarl::ui::{NodeLayout, PinInfo, SnarlStyle, SnarlViewer, WireStyle};
use egui_snarl::{InPin, InPinId, Node as SnarlNode, NodeId, OutPin, OutPinId, Snarl};
use std::fmt::Debug;
use std::marker::PhantomData;
use tiny_bail::prelude::r;

pub struct SetupGraphVisPlugin;

impl Plugin for SetupGraphVisPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			PreUpdate,
			sync_snarl::<GameSetupKey>.run_if(resource_exists::<SetupGraphVisState<GameSetupKey>>),
		)
		.add_systems(Update, visualize_setup_graph::<GameSetupKey>);
	}
}

#[derive(Deref)]
pub struct SetupGraphViewer<'a, K: SetupKey>(&'a SetupTracker<K>);

impl<'a, K: SetupKey> SetupGraphViewer<'a, K> {
	pub fn key_color(&self, key: &K) -> Option<Color32> {
		// Final outputs are white
		self.dependants_of(key).next()?;

		let mut i = 0;
		for (k, _) in self.entries().iter() {
			if self.dependants_of(k).next().is_none() {
				// Avoid skipping colors for outputs that will be white anyway
				continue;
			}
			if *k == *key {
				break;
			}
			i += 1;
		}

		Some(COLORS[i % COLORS.len()])
	}
}

const COLORS: &[Color32] = &[
	Color32::RED,
	Color32::from_rgb(255, 127, 0),
	Color32::YELLOW,
	Color32::GREEN,
	Color32::from_rgb(0, 255, 127),
	Color32::from_rgb(0, 255, 255),
	Color32::from_rgb(0, 127, 255),
	Color32::BLUE,
	Color32::from_rgb(127, 0, 255),
	Color32::from_rgb(255, 0, 255),
	Color32::from_rgb(255, 0, 127),
];

impl<K: SetupKey + Debug> SnarlViewer<SystemId> for SetupGraphViewer<'_, K> {
	fn title(&mut self, node: &SystemId) -> String {
		self.providers()[node].name().to_owned()
	}

	fn outputs(&mut self, node: &SystemId) -> usize {
		self.providers()[node].provides().len()
	}

	fn inputs(&mut self, node: &SystemId) -> usize {
		self.providers()[node].requires().len()
	}

	fn show_input(
		&mut self,
		pin: &InPin,
		ui: &mut Ui,
		scale: f32,
		snarl: &mut Snarl<SystemId>,
	) -> PinInfo {
		let key = &self.providers()[&snarl[pin.id.node]].requires()[pin.id.input];
		let fill = self.key_color(key);
		ui.label(format!("{key:?}"));
		PinInfo { fill, ..default() }
	}

	fn show_output(
		&mut self,
		pin: &OutPin,
		ui: &mut Ui,
		scale: f32,
		snarl: &mut Snarl<SystemId>,
	) -> PinInfo {
		let key = &self.providers()[&snarl[pin.id.node]].provides()[pin.id.output];
		let fill = self.key_color(key);
		ui.label(format!("{key:?}"));
		PinInfo { fill, ..default() }
	}
}

#[derive(Resource, Debug)]
pub struct SetupGraphVisState<K: SetupKey> {
	snarl: Snarl<SystemId>,
	_marker: PhantomData<K>,
}

impl<K: SetupKey> Default for SetupGraphVisState<K> {
	fn default() -> Self {
		Self {
			snarl: default(),
			_marker: PhantomData,
		}
	}
}

pub fn sync_snarl<K: SetupKey>(
	mut snarl: ResMut<SetupGraphVisState<K>>,
	tracker: Res<SetupTracker<K>>,
) {
	let mut nodes = snarl
		.snarl
		.nodes_ids_data()
		.map(|(id, node)| (id, node.value))
		.collect::<HashMap<NodeId, SystemId>>();
	if tracker.is_changed() {
		for (i, stage) in tracker.stages().into_iter().enumerate() {
			for (j, id) in stage.into_iter().enumerate() {
				if !nodes.iter().any(|(_, node)| *node == id) {
					let node = snarl.snarl.insert_node(
						bevy_egui::egui::Pos2::new(i as f32 * 400.0, j as f32 * 96.0),
						id,
					);
					nodes.insert(node, id);
				}
			}
		}
	}
	for (id, info) in tracker.providers() {
		let Some(provider_node) = nodes
			.iter()
			.find_map(|(nid, node)| (*node == *id).then_some(*nid))
		else {
			error!("Missing Snarl node for provider: {id:?}");
			continue;
		};
		for (output_idx, provision) in info.provides().iter().enumerate() {
			for (dependant, input_idx) in tracker.dependants_of(provision) {
				let Some(dependant_node) = nodes
					.iter()
					.find_map(|(nid, node)| (*node == dependant).then_some(*nid))
				else {
					error!("Missing Snarl node for dependency: {dependant:?}");
					continue;
				};
				snarl.snarl.connect(
					OutPinId {
						node: provider_node,
						output: output_idx,
					},
					InPinId {
						node: dependant_node,
						input: input_idx,
					},
				);
			}
		}
	}
}

pub fn visualize_setup_graph<K: SetupKey + Debug>(
	mut cmds: Commands,
	graph: Res<SetupTracker<K>>,
	mut ctx: EguiContexts,
	mut state: Option<ResMut<SetupGraphVisState<K>>>,
) {
	let ctx = r!(ctx.try_ctx_mut());
	let style = SnarlStyle {
		node_layout: Some(NodeLayout::Sandwich),
		pin_fill: Some(Color32::WHITE),
		wire_width: Some(2.0),
		wire_style: Some(WireStyle::AxisAligned { corner_radius: 8.0 }),
		bg_pattern_stroke: Some(bevy_egui::egui::Stroke {
			width: 1.0,
			color: Color32::BLACK,
		}),
		centering: Some(true),
		..default()
	};
	let mut open = state.is_some();
	bevy_egui::egui::Window::new(format!(
		"SetupTracker<{}> Graph",
		disqualified::ShortName::of::<K>()
	))
	.open(&mut open)
	.default_width(1200.0)
	.show(ctx, |ui| {
		if let Some(state) = &mut state {
			state.snarl.show(
				&mut SetupGraphViewer(&*graph),
				&style,
				std::any::type_name::<SetupTracker<K>>(),
				ui,
			);
		}
	});
	if !open {
		cmds.remove_resource::<SetupGraphVisState<K>>();
	}
}

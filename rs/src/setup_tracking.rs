use bevy::asset::UntypedAssetId;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemId;
use bevy::platform_support::collections::{HashMap, HashSet};
use bevy::prelude::*;
use nutype::nutype;
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::FromResidual;
use std::sync::Mutex;

pub struct SetupTrackingPlugin<K: SetupKey, C: Condition<M>, M, Fin: IntoSystem<(), (), ()>> {
	condition: Mutex<Option<C>>,
	on_finished: Mutex<Option<Fin>>,
	_marker: PhantomData<(K, M)>,
}

impl<K: SetupKey, C: Condition<M>, M, Fin: IntoSystem<(), (), ()>>
	SetupTrackingPlugin<K, C, M, Fin>
{
	pub fn new(condition: C, on_finished: Fin) -> Self {
		Self {
			condition: Mutex::new(Some(condition)),
			on_finished: Mutex::new(Some(on_finished)),
			_marker: PhantomData,
		}
	}
}

impl<
	K: SetupKey + Debug,
	C: Condition<M> + Send + 'static,
	M: Send + Sync + 'static,
	Fin: IntoSystem<(), (), ()> + Send + 'static,
> Plugin for SetupTrackingPlugin<K, C, M, Fin>
{
	fn build(&self, app: &mut App) {
		let on_finished = self.on_finished.lock().unwrap().take().unwrap();
		let fin = app.register_system(on_finished);
		app.insert_resource(SetupTracker::<K>::new(fin))
			.add_systems(Startup, validate_tracker::<K>)
			.add_systems(
				Update,
				advance_setup::<K>.run_if(self.condition.lock().unwrap().take().unwrap()),
			);
	}
}

pub fn validate_tracker<K: SetupKey + Debug>(world: &mut World) {
	SetupTracker::<K>::validate(world).unwrap()
}

pub fn advance_setup<K: SetupKey + Debug>(world: &mut World) {
	world.resource_scope::<SetupTracker<K>, _>(|world, mut tracker| {
		let mut pending = HashSet::new();
		let ready = tracker
			.entries
			.iter()
			.filter_map(|(key, checker)| {
				if world.run_system(*checker).unwrap().finished() {
					Some(key.clone())
				} else {
					pending.insert(key.clone());
					None
				}
			})
			.collect::<HashSet<_>>();
		debug!(?ready, ?pending);
		let should_run = move |info: &ProviderInfo<K>| {
			for provision in &info.provides {
				if ready.contains(provision) {
					return false;
				}
			}
			for requirement in &info.requires {
				if !ready.contains(requirement) {
					return false;
				}
			}
			true
		};
		for (system, info) in tracker.providers.iter() {
			if should_run(info) {
				if let Err(e) = world.run_system(*system) {
					error_once!("{e}");
				}
			}
		}
		let progress = tracker.progress(world);
		debug!(?progress);
		if progress.finished() {
			world.run_system(tracker.on_finished).unwrap();
		}
		if tracker.last_progress != progress {
			tracker.last_progress = progress;
		}
	});
}

pub trait SetupKey: Eq + Hash + Clone + Send + Sync + 'static {
	/// Returns the system that calculates the progress of this setup entry. Will be called
	/// the first time each key appears in a [Provider] `requires` or `provides` list. The
	/// SystemId will be cached and used for any further appearances.
	fn register_progress_checker(&self, world: &mut World) -> SystemId<(), Progress>;
	/// A scale factor to apply to this entry when calculating total progress.
	/// Defaults to `1.0`
	fn relative_time_estimate(&self) -> f32 {
		1.0
	}
}

pub type ProgressCheckerId = SystemId<(), Progress>;

#[derive(Resource, Debug)]
pub struct SetupTracker<K: SetupKey> {
	entries: HashMap<K, ProgressCheckerId>,
	providers: HashMap<SystemId, ProviderInfo<K>>,
	on_finished: SystemId,
	last_progress: Progress,
}

impl<K: SetupKey> SetupTracker<K> {
	pub fn new(on_finished: SystemId) -> Self {
		Self {
			entries: default(),
			providers: default(),
			on_finished,
			last_progress: default(),
		}
	}

	pub fn register_provider(
		&mut self,
		system: SystemId,
		provider: ProviderInfo<K>,
		world: &mut World,
	) {
		for req in &provider.requires {
			if !self.entries.contains_key(req) {
				self.entries
					.insert(req.clone(), req.register_progress_checker(world));
			}
		}
		for prov in &provider.provides {
			if !self.entries.contains_key(prov) {
				self.entries
					.insert(prov.clone(), prov.register_progress_checker(world));
			}
		}
		self.providers.insert(system, provider);
	}

	pub fn validate(world: &mut World) -> Result<(), InvalidSetupGraph<K>>
	where
		K: Debug,
	{
		world.resource_scope::<SetupTracker<K>, _>(|_, tracker| {
			let mut unprovided = tracker.entries.keys().cloned().collect::<HashSet<_>>();
			let mut providers = HashMap::<K, Vec<SystemId>>::default();
			let cyclic_dependencies = HashSet::<K>::default();
			for (system, info) in tracker.providers.iter() {
				for provision in &info.provides {
					providers
						.entry(provision.clone())
						.or_insert_with(Vec::new)
						.push(*system);
					unprovided.remove(provision);
				}
			}
			providers.retain(|_, providers| providers.len() > 1);
			warn!("TODO: Detect cycles");
			if !unprovided.is_empty() || !providers.is_empty() || !cyclic_dependencies.is_empty() {
				Err(InvalidSetupGraph {
					unprovided,
					duplicate_providers: providers,
					cyclic_dependencies,
				})
			} else {
				Ok(())
			}
		})
	}

	pub fn progress(&self, world: &mut World) -> Progress {
		let total: f32 = self.entries.keys().map(K::relative_time_estimate).sum();
		let sum: f32 = self
			.entries
			.iter()
			.map(|(key, checker)| {
				*world.run_system(*checker).unwrap() * key.relative_time_estimate()
			})
			.sum();
		Progress::new(sum / total)
	}

	pub fn last_progress(&self) -> Progress {
		self.last_progress
	}

	pub fn entries(&self) -> &HashMap<K, ProgressCheckerId> {
		&self.entries
	}

	pub fn providers(&self) -> &HashMap<SystemId, ProviderInfo<K>> {
		&self.providers
	}

	pub fn providers_of<'a, 'b>(
		&'a self,
		key: &'b K,
	) -> impl Iterator<Item = (SystemId, usize)> + use<'a, 'b, K> {
		self.providers.iter().filter_map(|(id, info)| {
			info.provides
				.iter()
				.enumerate()
				.find(|(_, item)| **item == *key)
				.map(|(i, _)| (*id, i))
		})
	}

	pub fn dependants_of<'a, 'b>(
		&'a self,
		key: &'b K,
	) -> impl Iterator<Item = (SystemId, usize)> + use<'a, 'b, K> {
		self.providers.iter().filter_map(|(id, info)| {
			info.requires
				.iter()
				.enumerate()
				.find(|(_, item)| **item == *key)
				.map(|(i, _)| (*id, i))
		})
	}

	pub fn stages(&self) -> Vec<Vec<SystemId>> {
		let mut provided_so_far = HashSet::new();
		let mut stages: Vec<Vec<SystemId>> = Vec::new();
		let mut providers = self.providers.clone();

		while !providers.is_empty() {
			let mut stage = Vec::new();
			let mut provided_this_stage = Vec::new();
			providers.retain(|id, info| {
				for req in info.requires.iter() {
					if !provided_so_far.contains(req) {
						return true;
					}
				}
				stage.push(*id);
				provided_this_stage.extend_from_slice(&info.provides);
				false
			});
			if stage.is_empty() {
				error!("Not all keys are provided");
				break;
			}
			provided_so_far.extend(provided_this_stage);
			stages.push(stage);
		}

		stages
	}
}

#[derive(Debug, Clone)]
pub struct ProviderInfo<K: SetupKey> {
	requires: Vec<K>,
	provides: Vec<K>,
	name: Cow<'static, str>,
}

impl<K: SetupKey> ProviderInfo<K> {
	pub fn should_run(&self, entries: &HashMap<K, ProgressCheckerId>, world: &mut World) -> bool {
		for provision in &self.provides {
			if world.run_system(entries[provision]).unwrap().finished() {
				return false;
			}
		}
		for requirement in &self.requires {
			if !world.run_system(entries[requirement]).unwrap().finished() {
				return false;
			}
		}
		true
	}

	pub fn requires(&self) -> &[K] {
		&self.requires
	}

	pub fn provides(&self) -> &[K] {
		&self.provides
	}

	pub fn name(&self) -> &str {
		&self.name
	}
}

pub struct Provider<K: SetupKey, S: IntoSystem<(), (), M>, M> {
	requires: Vec<K>,
	provides: Vec<K>,
	system: S,
	name: Option<Cow<'static, str>>,
	_marker: PhantomData<M>,
}

impl<K: SetupKey, S: IntoSystem<(), (), M> + 'static, M> Provider<K, S, M> {
	fn register(self, world: &mut World) {
		let Self {
			requires,
			provides,
			system,
			name,
			..
		} = self;
		let name = name.unwrap_or_else(|| {
			let full_name = std::any::type_name_of_val(&system);
			let full_name: &'static str = if full_name.starts_with('<') && full_name.ends_with('>')
			{
				&full_name[1..full_name.len() - 2]
			} else {
				full_name
			};
			let full_name: &'static str = full_name
				.trim_start_matches("mt_thyrios::")
				.trim_start_matches("setup_tracking::");
			let mut full_name = Cow::<'static, str>::Borrowed(full_name);
			if full_name.contains("mt_thyrios::") {
				full_name = Cow::Owned(full_name.replace("mt_thyrios::", ""));
			}
			if full_name.contains("setup_tracking::") {
				full_name = Cow::Owned(full_name.replace("setup_tracking::", ""));
			}
			full_name
		});
		let info = ProviderInfo {
			requires,
			provides,
			name,
		};
		let system = world.register_system(system);
		world.resource_scope::<SetupTracker<K>, _>(|world, mut tracker| {
			tracker.register_provider(system, info, world);
		})
	}
}

pub trait RegisterProvider {
	fn register_provider<K: SetupKey, S: IntoSystem<(), (), M> + 'static, M>(
		&mut self,
		provider: Provider<K, S, M>,
	) -> &mut Self;
}

impl RegisterProvider for World {
	fn register_provider<K: SetupKey, S: IntoSystem<(), (), M> + 'static, M>(
		&mut self,
		provider: Provider<K, S, M>,
	) -> &mut Self {
		provider.register(self);
		self
	}
}

impl RegisterProvider for App {
	fn register_provider<K: SetupKey, S: IntoSystem<(), (), M> + 'static, M>(
		&mut self,
		provider: Provider<K, S, M>,
	) -> &mut Self {
		provider.register(self.world_mut());
		self
	}
}

pub trait IntoDependencyProvider<K: SetupKey, S: IntoSystem<(), (), M>, M> {
	fn provides(self, keys: impl IntoIterator<Item = K>) -> Provider<K, S, M>;
	fn requires(self, keys: impl IntoIterator<Item = K>) -> Provider<K, S, M>;
}

impl<K: SetupKey, S: IntoSystem<(), (), M>, M> IntoDependencyProvider<K, S, M> for S {
	fn provides(self, keys: impl IntoIterator<Item = K>) -> Provider<K, S, M> {
		Provider {
			provides: keys.into_iter().collect(),
			requires: Vec::new(),
			system: self,
			name: None,
			_marker: PhantomData,
		}
	}

	fn requires(self, keys: impl IntoIterator<Item = K>) -> Provider<K, S, M> {
		Provider {
			provides: Vec::new(),
			requires: keys.into_iter().collect(),
			system: self,
			name: None,
			_marker: PhantomData,
		}
	}
}

impl<K: SetupKey, S: IntoSystem<(), (), M>, M> IntoDependencyProvider<K, S, M>
	for Provider<K, S, M>
{
	fn provides(mut self, keys: impl IntoIterator<Item = K>) -> Self {
		self.provides.extend(keys);
		self
	}
	fn requires(mut self, keys: impl IntoIterator<Item = K>) -> Self {
		self.requires.extend(keys);
		self
	}
}

#[nutype(
	const_fn,
	sanitize(with = clamp_0_to_1),
	validate(finite),
	derive(Default, Debug, Deref, Clone, Copy, PartialEq, PartialOrd),
	default = 0.0,
)]
pub struct Progress(f32);

impl Progress {}

const fn clamp_0_to_1(val: f32) -> f32 {
	val.clamp(0.0, 1.0)
}

impl Progress {
	pub const ZERO: Self = Self::new(0.0);
	pub const DONE: Self = Self::new(1.0);

	pub fn finished(self) -> bool {
		*self >= 1.0 - f32::EPSILON
	}

	pub const fn new(value: f32) -> Self {
		match Self::try_new(value) {
			Ok(val) => val,
			Err(e) => match e {
				ProgressError::FiniteViolated => panic!("value is not finite"),
			},
		}
	}
}

impl From<bool> for Progress {
	fn from(val: bool) -> Self {
		Self::new(if val { 1.0 } else { 0.0 })
	}
}

/// For question mark operator
/// ```
/// # #[derive(Resource)]
/// # struct Foo;
/// fn foo_exists(foo: Option<Res<Foo>>) -> Progress {
///     let foo = foo?;
/// }
/// ```
impl<T> FromResidual<Option<T>> for Progress {
	fn from_residual(val: Option<T>) -> Self {
		if val.is_some() {
			#[cfg(debug_assertions)]
			{ unreachable!(); }
			#[cfg(not(debug_assertions))]
			{ Self::DONE }
		} else {
			Self::ZERO
		}
	}
}

#[allow(unused)] // Fields are specifically for debug output
#[derive(Debug, Clone)]
pub struct InvalidSetupGraph<K: SetupKey + Debug> {
	unprovided: HashSet<K>,
	duplicate_providers: HashMap<K, Vec<SystemId>>,
	cyclic_dependencies: HashSet<K>,
}

impl<K: SetupKey + Debug> std::fmt::Display for InvalidSetupGraph<K> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		<Self as std::fmt::Debug>::fmt(self, f)
	}
}

pub fn single_spawn_progress<F: QueryFilter>(q: Option<Single<(), F>>) -> Progress {
	q.is_some().into()
}

pub fn resource_progress<R: Resource>(res: Option<Res<R>>) -> Progress {
	res.is_some().into()
}

pub fn state_progress<S: States>(state: S) -> impl System<In = (), Out = Progress> {
	IntoSystem::into_system(move |curr: Option<Res<State<S>>>| {
		curr.map(|curr| (*curr.get() == state).into())
			.unwrap_or_default()
	})
}

pub fn assets_progress<C: AssetCollection>(
	collection: Option<Res<C>>,
	server: Res<AssetServer>,
) -> Progress {
	let Some(collection) = collection else {
		return Progress::ZERO;
	};

	let (done, total) = collection.iter_ids().fold((0, 0), |(done, total), id| {
		let Some(state) = server.get_load_state(id) else {
			return (done, total + 1);
		};

		let done = if state.is_loaded() { done + 1 } else { done };

		(done, total + 1)
	});

	Progress::new(done as f32 / total as f32)
}

pub trait AssetCollection: Resource {
	fn iter_ids(&self) -> impl Iterator<Item = UntypedAssetId>;
}

pub fn load_assets<C: AssetCollection + FromWorld>(mut cmds: Commands, collection: Option<Res<C>>) {
	if collection.is_some() {
		return;
	}

	cmds.init_resource::<C>();
}

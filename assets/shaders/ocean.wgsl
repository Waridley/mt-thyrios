#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_normal_local_to_world},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{globals, view},
    pbr_bindings::{depth_map_texture, depth_map_sampler},
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
    prepass_utils,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct Wave {
	origin: vec2<f32>,
	direction: vec2<f32>,
	frequency: f32,
	amplitude: f32,
	steepness: f32,
	speed: f32,
}

struct Tide {
	frequency: f32,
	amplitude: f32,
	steepness: f32,
	speed: f32,
}

const WAVE_COUNT = 10;
const TIDE_COUNT = 4;

struct OceanMaterial {
	waves: array<Wave, WAVE_COUNT>,
	tides: array<Tide, TIDE_COUNT>,
    size: f32,
    storm_intensity: f32,
	horizon_color: vec4<f32>,
}

@group(2) @binding(100) var<uniform> ocean: OceanMaterial;

@vertex
fn vertex(
	@location(0) position: vec3<f32>,
	@builtin(instance_index) instance_index: u32,
) -> VertexOutput {
	let xy = position.xy;
	let len = length(xy) / ocean.size;
	var p = position;
	#ifdef LIGHTING
	#ifndef FRAGMENT_NORMALS
		var n = vec3(xy * 0.002, 0.0);
	#endif
	#endif
	var max_amp = 0.0;
	for(var i = 0; i < WAVE_COUNT; i++) {
		p += gerstner(xy, ocean.waves[i]);
		#ifdef LIGHTING
		#ifndef FRAGMENT_NORMALS
			n += gerstner_normal(xy, ocean.waves[i]);
		#endif
		#endif
		max_amp += (ocean.waves[i].amplitude + 1.0) * 0.5;
	}
	for(var i = 0; i < TIDE_COUNT; i++) {
		p += gerstner_tide(xy, ocean.tides[i]);
		#ifdef LIGHTING
		#ifndef FRAGMENT_NORMALS
			n += gerstner_tide_normal(xy, ocean.tides[i]);
		#endif
		#endif
		max_amp += (ocean.tides[i].amplitude + 1.0) * 0.5;
	}

	#ifdef LIGHTING
	#ifndef FRAGMENT_NORMALS
		let norm = normalize(vec3(-n.x, -n.y, 1.0 - n.z));
	#endif
	#endif

	var out: VertexOutput;
	let world_from_local = get_world_from_local(instance_index);
	out.world_position = mesh_position_local_to_world(
		world_from_local,
		vec4<f32>(p, 1.0),
	);
	out.position = position_world_to_clip(out.world_position.xyz);

	var peak = ((p.z / max_amp) + 1.0) * 0.5;
	peak = pow(peak * 1.2, 4.0);
	out.color = vec4(
		min(peak * 0.25, 1.0),
		min(peak * 0.25, 1.0),
		0.25,
		min(0.1 + (peak * 0.9), 1.0),
	);

	#ifdef LIGHTING
		#ifdef FRAGMENT_NORMALS
			// Hack to pass original position to fragment shader for normal calculations.
			// Could be done in a color channel if necessary.
			out.world_normal = position;
		#else
			out.world_normal = mesh_normal_local_to_world(norm, instance_index);
		#endif
	#endif

	return out;
}

@fragment
fn fragment(
	in: VertexOutput,
	@builtin(front_facing) is_front: bool,
) -> FragmentOutput {
	var input = in;

	//<editor-fold desc="Normal calculation">
	#ifdef LIGHTING
	#ifdef FRAGMENT_NORMALS
		// "world_normal" is actually original vertex position
		let xy = in.world_normal.xy;

		var n = vec3(vec2(xy * 0.0015), 0.0);
		for(var i = 0; i < WAVE_COUNT; i++) {
			n += gerstner_normal(xy, ocean.waves[i]);
		}
		for(var i = 0; i < TIDE_COUNT; i++) {
			n += gerstner_tide_normal(xy, ocean.tides[i]);
		}
		let norm = normalize(vec3(-n.x, -n.y, 1.0 - n.z));
		input.world_normal = norm;
	#endif
	#endif
	//</editor-fold>

	//<editor-fold desc="Shallow water transparency">
	let prepass_depth = bevy_pbr::prepass_utils::prepass_depth(in.position, 0u);
	let unscaled_pre_depth = prepass_depth / input.position.w;
	let ocean_z = input.position.z / input.position.w;
	let ocean_depth = (ocean_z - unscaled_pre_depth) / input.position.w;

	if ocean_depth < 0.15 {
		let t = 0.15 - ocean_depth;
		input.color = vec4(mix(input.color.rgb, vec3(20.0), t * 2.0), t * 0.1);
	} else {
		let a = pow(ocean_depth, 3.0) * 0.2;
		input.color.a = min(input.color.a + a + 0.01, 1.0);
	}
	//</editor-fold>

	// Setting this before lighting results in a different tone, worth experimenting with later.
//	input.color = vec4(vec2(input.color.rg * (0.2 * ocean.storm_intensity + 0.2)), input.color.ba);


	//<editor-fold desc="Standard shading">
	var pbr_input = pbr_input_from_standard_material(input, is_front);

	var out: FragmentOutput;
	#ifdef LIGHTING
		out.color = apply_pbr_lighting(pbr_input);
	#else
		out.color = pbr_input.material.base_color;
		out.color = vec4(vec3(out.color.rgb * 0.1), out.color.a);
	#endif
	//</editor-fold>

	out.color = vec4(vec2(out.color.rg * (0.2 * ocean.storm_intensity + 0.2)), out.color.ba);
	out.color = mix(out.color, ocean.horizon_color, length(in.world_position.xy) / ocean.size);

	out.color = main_pass_post_lighting_processing(pbr_input, out.color);

	return out;
}

fn get_phase(pos: vec2<f32>, dir: vec2<f32>, w: Wave) -> f32 {
	return pos.x * dir.x + pos.y * dir.y;
}

fn get_time(speed: f32) -> f32 {
	return globals.time * speed * ((ocean.storm_intensity + 0.3) * 1.2);
}

fn gerstner(pos: vec2<f32>, w: Wave) -> vec3<f32> {
	let d = w.direction;
	let phi = get_phase(pos, d, w);
	let t = get_time(w.speed);
	let a = w.amplitude * ocean.storm_intensity;

	var g = vec3(0.0);
	g.x = w.steepness * a * d.x * cos(w.frequency * phi + t);
	g.y = w.steepness * a * d.y * cos(w.frequency * phi + t);
	g.z = a * sin(w.frequency * phi + t);

	return g;
}

fn normalize_or_zero(v: vec2<f32>) -> vec2<f32> {
	let l = length(v);
	if l == 0.0 {
		return vec2(0.0);
	} else {
		return v / l;
	}
}

fn gerstner_tide(pos: vec2<f32>, w: Tide) -> vec3<f32> {
	let d = normalize_or_zero(pos);
	let phi = length(pos);
	let t = get_time(w.speed);
	// Get rid of stretchy flat disk in center
	let taper = min(1.0, pow(phi * 0.125, 4.0));
	let a = w.amplitude * ocean.storm_intensity * taper;

	var g = vec3(0.0);
	g.x = w.steepness * a * d.x * cos(w.frequency * phi + t);
	g.y = w.steepness * a * d.y * cos(w.frequency * phi + t);
	g.z = a * sin(w.frequency * phi + t);
	

	return g;
}

fn gerstner_normal(pos: vec2<f32>, w: Wave) -> vec3<f32> {
	let d = w.direction;
	let phi = get_phase(pos, d, w);
	let t = get_time(w.speed);

	var n = vec3(0.0);

	let wa = w.frequency * w.amplitude * ocean.storm_intensity;
	let s = sin(w.frequency * phi + t);
	let c = cos(w.frequency * phi + t);

	n.x = d.x * wa * c;
	n.y = d.y * wa * c;
	n.z = w.steepness * wa * s;

	return n;
}

fn gerstner_tide_normal(pos: vec2<f32>, w: Tide) -> vec3<f32> {
	let d = normalize_or_zero(pos);
	let phi = length(pos);
	let t = -get_time(w.speed);

	var n = vec3(0.0);

	let wa = w.frequency * w.amplitude * ocean.storm_intensity;
	let s = sin(w.frequency * phi + t);
	let c = cos(w.frequency * phi + t);

	n.x = d.x * wa * c;
	n.y = d.y * wa * c;
	n.z = w.steepness * wa * s;

	return n;
}

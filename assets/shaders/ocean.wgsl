#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_normal_local_to_world},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{globals, view},
    pbr_bindings::{depth_map_texture, depth_map_sampler},
    utils::rand_vec2f
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    prepass_utils::prepass_depth,
}
#endif


const MIN_BLUENESS: f32 = 0.08;
const MIN_BRIGHTNESS: f32 = 0.02;
const FINAL_BRIGHTNESS: f32 = 3.0;

const LACUNARITY: f32 = 1.7;
const GAIN: f32 = 0.75;
const STEEP_GAIN: f32 = 0.87;
const SPEED_GAIN: f32 = 1.25;

const INIT_WAVE: Wave = Wave(
	vec2<f32>(0.0, 0.0), // origin
	0.03, // frequency
	2.0, // amplitude
	4.0, // steepness
	1.0, // speed
);

const INIT_TIDE: WaveParams = WaveParams(
	0.029, // frequency
	2.0, // amplitude
	1.5, // steepness
	0.1, // speed
);

struct Wave {
	origin: vec2<f32>,
	frequency: f32,
	amplitude: f32,
	steepness: f32,
	speed: f32,
}

struct WaveParams {
	frequency: f32,
	amplitude: f32,
	steepness: f32,
	speed: f32,
}

struct OceanMaterial {
	seed: u32,
	vertex_wave_octaves: u32,
	vertex_tide_octaves: u32,
	fragment_wave_octaves: u32,
	fragment_tide_octaves: u32,
	size: f32,
    storm_intensity: f32,
	horizon_color: vec4<f32>,
}

@group(2) @binding(100) var<uniform> ocean: OceanMaterial;

@vertex
fn vertex(
	@location(0) local_pos: vec3<f32>,
	@builtin(instance_index) instance_index: u32,
) -> VertexOutput {
	let xf = get_world_from_local(instance_index);
	// Apply rotation only. `See crate::game::ocean::ocean_rotation_follow_cam_anchor`
	let position = mesh_position_local_to_world(
		xf,
		vec4<f32>(local_pos, 0.0),
	);
    let xy = position.xy;
	let len = length(xy) / ocean.size;
	// position is needed later if lighting is enabled
	var p = position.xyz;
	#ifdef LIGHTING
	#ifndef FRAGMENT_NORMALS
		var n = vec3(xy * 0.002, 0.0);
	#endif
	#endif
	var amp_sum = 0.0;

	var freq_mult = 1.0;
	var amp_mult = 1.0;
	var steep_mult = 1.0;
	var speed_mult = 1.0;
	var state = ocean.seed;
	for(var i: u32 = 0; i < ocean.vertex_wave_octaves; i++) {
		var wave = INIT_WAVE;
		wave.origin = (rand_vec2f(&state) * (ocean.size * 4.0)) - (ocean.size * 2.0);

		
		wave.frequency *= freq_mult;
		wave.amplitude *= amp_mult;
		wave.steepness *= steep_mult;
		wave.speed *= speed_mult;
		
		freq_mult *= LACUNARITY;
		amp_mult *= GAIN;
		steep_mult *= STEEP_GAIN;
		speed_mult *= SPEED_GAIN;

		p += gerstner(xy, wave);
		#ifdef LIGHTING
		#ifndef FRAGMENT_NORMALS
			n += gerstner_normal(xy, wave);
		#endif
		#endif
		amp_sum += wave.amplitude;
	}

//	// Every 7th tide is larger IRL
//	var swell: WaveParams;
//	swell.frequency = ocean.base_tide.frequency / 7.0;
//	swell.amplitude = ocean.base_tide.amplitude;
//	swell.steepness = ocean.base_tide.steepness * 7.0;
//	swell.speed = ocean.base_tide.speed;
//	let swell_amount = gerstner_tide(xy, swell);
//	p += swell_amount;
//	#ifdef LIGHTING
//	#ifndef FRAGMENT_NORMALS
//		n += gerstner_tide_normal(xy, swell);
//	#endif
//	#endif
//
//	// Swell shouldn't affect foam quite so much
//	// Otherwise low tide is too dark or hight tide is too white
//	amp_sum += swell_amount.z * 2.0;

	freq_mult = 1.0;
	amp_mult = 1.0;
	steep_mult = 1.0;
	speed_mult = 1.0;

	for(var i: u32 = 0; i < ocean.vertex_tide_octaves; i++) {
		var tide = INIT_TIDE;
		
		tide.frequency *= freq_mult;
		tide.amplitude *= amp_mult;
		tide.steepness *= steep_mult;
		tide.speed *= speed_mult;
		
		freq_mult *= LACUNARITY;
		amp_mult *= GAIN;
		steep_mult *= STEEP_GAIN;
		speed_mult *= SPEED_GAIN;
		
		p += gerstner_tide(xy, tide);
		#ifdef LIGHTING
		#ifndef FRAGMENT_NORMALS
			n += gerstner_tide_normal(xy, tide);
		#endif
		#endif
		amp_sum += tide.amplitude;
	}

	#ifdef LIGHTING
	#ifndef FRAGMENT_NORMALS
		let norm = normalize(vec3(-n.x, -n.y, 1.0 - n.z));
	#endif
	#endif

	var out: VertexOutput;
	// We already rotated for world-space gerstner calculations
	out.world_position = xf[3] + vec4<f32>(p, 1.0);
	out.position = position_world_to_clip(out.world_position.xyz);

	let z_norm = p.z / amp_sum;
	var peak = (z_norm * 0.5) + 0.7;
	out.color = vec4(
		clamp(pow(peak, 12.0), MIN_BRIGHTNESS, 1.0),
		clamp(pow(peak, 5.0), MIN_BRIGHTNESS, 1.0),
		clamp(pow(peak, 9.0) + MIN_BLUENESS, MIN_BLUENESS, 1.0),
		clamp((z_norm * z_norm) + MIN_BRIGHTNESS, MIN_BRIGHTNESS, 1.0),
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
		var p = in.world_normal;
		var n = vec3(vec2(xy * 0.0015), 0.0);
		var freq_mult = 1.0;
		var amp_mult = 1.0;
		var steep_mult = 1.0;
		var speed_mult = 1.0;
		for(var i: u32 = 0; i < ocean.fragment_wave_octaves; i++) {
			var wave = ocean.waves[0];
			wave.origin = ocean.waves[i].origin;
			wave.direction = ocean.waves[i].direction;
			let exp = f32(i);

			wave.frequency *= pow(LACUNARITY, exp);
			wave.amplitude *= pow(GAIN, exp);
			wave.steepness *= pow(STEEP_GAIN, exp);
			wave.speed *= pow(SPEED_GAIN, exp);

			freq_mult *= LACUNARITY;
			amp_mult *= GAIN;
			steep_mult *= STEEP_GAIN;
			speed_mult *= SPEED_GAIN;

			n += gerstner_normal(xy, wave) * 0.1;
		}

		freq_mult = 1.0;
		amp_mult = 1.0;
		steep_mult = 1.0;
		speed_mult = 1.0;
		for(var i: u32 = 0; i < ocean.fragment_tide_octaves; i++) {
			var tide = ocean.base_tide;
			let exp = f32(i);

			tide.frequency *= pow(LACUNARITY, exp);
			tide.amplitude *= pow(GAIN, exp);
			tide.steepness *= pow(STEEP_GAIN, exp);
			tide.speed *= pow(SPEED_GAIN, exp);

			freq_mult *= LACUNARITY;
			amp_mult *= GAIN;
			steep_mult *= STEEP_GAIN;
			speed_mult *= SPEED_GAIN;

			n += gerstner_tide_normal(xy, tide);
		}
		input.world_normal = normalize(vec3(-n.x, -n.y, 1.0 - n.z));
	#endif
	#endif
	//</editor-fold>

#ifndef PREPASS_PIPELINE
	//<editor-fold desc="Shallow water transparency">
	let prepass_depth = prepass_depth(in.position, 0u);
	let unscaled_pre_depth = prepass_depth / input.position.w;
	let ocean_z = input.position.z / input.position.w;
	let ocean_depth = (ocean_z - unscaled_pre_depth) / input.position.w;

	let a = ocean_depth * ocean_depth * 0.2;
	input.color.a = min(input.color.a + a + 0.5, 1.0);
	var t = max(0.15 - ocean_depth, 0.0) / 0.15;
	t = t * t * t;
	input.color = mix(input.color, vec4(2.0, 2.0, 2.0, 1.0), t);
	//</editor-fold>
#endif

	// Setting this before lighting results in a different tone, worth experimenting with later.
//	input.color = vec4(vec2(input.color.rg * (0.2 * ocean.storm_intensity + 0.2)), input.color.ba);


	//<editor-fold desc="Standard shading">
	var pbr_input = pbr_input_from_standard_material(input, is_front);

#ifdef PREPASS_PIPELINE
	let out = deferred_output(input, pbr_input);
#else
	var out: FragmentOutput;
	#ifdef LIGHTING
		out.color = apply_pbr_lighting(pbr_input);
	#else
		out.color = vec4(vec3(pbr_input.material.base_color.rgb * 0.1), pbr_input.material.base_color.a);
	#endif
	//</editor-fold>

	out.color = vec4(vec2(out.color.rg * (0.2 * ocean.storm_intensity + 0.2)), out.color.ba);

	out.color = vec4(out.color.rgb * FINAL_BRIGHTNESS, out.color.a);

	let dist = length(in.world_position.xy) / ocean.size;
	out.color = mix(out.color, ocean.horizon_color, dist * dist);

//	out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

	return out;
}

fn get_phase(pos: vec2<f32>, dir: vec2<f32>, w: Wave) -> f32 {
	return pos.x * dir.x + pos.y * dir.y;
}

fn get_time(speed: f32) -> f32 {
	return globals.time * speed * ((ocean.storm_intensity + 0.3) * 1.2);
}

fn gerstner(pos: vec2<f32>, w: Wave) -> vec3<f32> {
	let d = normalize_or_zero(w.origin - pos);
	let phi = get_phase(pos - w.origin, d, w);
	let t = get_time(w.speed);
	let a = w.amplitude * ocean.storm_intensity;

	var g = vec3(0.0);
	g.x = w.steepness * a * d.x * cos(w.frequency * phi + t);
	g.y = w.steepness * a * d.y * cos(w.frequency * phi + t);
	g.z = a * sin(w.frequency * phi + t);

	return g;
}

fn normalize_or_zero(v: vec2<f32>) -> vec2<f32> {
	let l2 = v.x * v.x + v.y * v.y;
	if l2 == 0.0 {
		return vec2(0.0);
	} else {
		return v / sqrt(l2);
	}
}

const TAPER_POWER: f32 = 0.5;
const MAX_TAPER_DIST: f32 = 20.0;

fn gerstner_tide(pos: vec2<f32>, w: WaveParams) -> vec3<f32> {
	let d = normalize_or_zero(pos);
	let phi = length(pos);
	let t = get_time(w.speed);
	// Reduce stretchy flat disk in center
	let taper = min(1.0, pow(phi / MAX_TAPER_DIST, TAPER_POWER));
	let a = w.amplitude * ocean.storm_intensity;

	var g = vec3(0.0);
	g.x = w.steepness * taper * a * d.x * cos(w.frequency * phi + t);
	g.y = w.steepness * taper * a * d.y * cos(w.frequency * phi + t);
	g.z = a * sin(w.frequency * phi + t);

	return g;
}

fn gerstner_normal(pos: vec2<f32>, w: Wave) -> vec3<f32> {
	let d = normalize_or_zero(pos - w.origin);
	let phi = get_phase(pos - w.origin, d, w);
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

fn gerstner_tide_normal(pos: vec2<f32>, w: WaveParams) -> vec3<f32> {
	let d = normalize_or_zero(pos);
	let phi = length(pos);
	let t = -get_time(w.speed);
	let taper = min(1.0, pow(phi / MAX_TAPER_DIST, TAPER_POWER));

	var n = vec3(0.0);

	let wa = w.frequency * w.amplitude * ocean.storm_intensity * taper;
	let s = sin(w.frequency * phi + t);
	let c = cos(w.frequency * phi + t);

	n.x = d.x * wa * c * taper;
	n.y = d.y * wa * c * taper;
	n.z = w.steepness * wa * s;

	return n;
}

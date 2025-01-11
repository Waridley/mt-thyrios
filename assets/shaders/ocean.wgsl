#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_normal_local_to_world},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
    mesh_bindings::mesh,
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
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
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

const WAVE_COUNT = 9;

struct OceanMaterial {
	waves: array<Wave, WAVE_COUNT>,
    size: f32,
    storm_intensity: f32,
}


@group(2) @binding(100) var<uniform> ocean: OceanMaterial;

@vertex
fn vertex(
	@location(0) position: vec3<f32>,
	@builtin(instance_index) instance_index: u32,
) -> VertexOutput {
	let xy = position.xy;
	let len = length(xy) / ocean.size;
	let dome = (len * len) * 100.0;
	var p = position;
	var n = vec3(xy * 0.002, 0.0);
	var max_amp = 0.0;
	for(var i = 0; i < WAVE_COUNT; i++) {
		p += gerstner(xy, ocean.waves[i]);
		n += gerstner_normal(xy, ocean.waves[i]);
		max_amp += (ocean.waves[i].amplitude + 1.0) * 0.5;
	}

	let pos = vec3(p.xy, p.z - dome);
	let norm = normalize(vec3(-n.x, -n.y, 1.0));

	var out: VertexOutput;
	let world_from_local = get_world_from_local(instance_index);
	out.world_position = mesh_position_local_to_world(
		world_from_local,
		vec4<f32>(pos, 1.0),
	);
	out.position = position_world_to_clip(out.world_position.xyz);

	var peak = ((p.z / max_amp) + 1.0) * 0.5;
//	peak = peak * pow(dot(norm, vec3(0.0, 0.0, 1.0)), 8.0);
	peak = pow(peak * 1.2, 4.0);
	out.color = vec4(
		min(0.05 + peak, 1.0),
		min(0.05 + peak, 1.0),
		1.0,
		min(0.1 + (peak * 0.9), 1.0),
	);
	out.world_normal = mesh_normal_local_to_world(norm, 0u);
	return out;
}

fn get_phase(pos: vec2<f32>, dir: vec2<f32>, w: Wave) -> f32 {
	return pos.x * dir.x + pos.y * dir.y;
}

fn get_time(w: Wave) -> f32 {
	return globals.time * w.speed * ocean.storm_intensity;
}

fn gerstner(pos: vec2<f32>, w: Wave) -> vec3<f32> {
	let d = w.direction;
	let phi = get_phase(pos, d, w);
	let t = get_time(w);
	let a = w.amplitude * ocean.storm_intensity;

	var g = vec3(0.0);
	g.x = w.steepness * a * d.x * cos(w.frequency * phi + t);
	g.y = w.steepness * a * d.y * cos(w.frequency * phi + t);
	g.z = a * sin(w.frequency * phi + t);

	return g;
}

fn gerstner_normal(pos: vec2<f32>, w: Wave) -> vec3<f32> {
	let d = w.direction;
	let phi = get_phase(pos, d, w);
	let t = get_time(w);

	var n = vec3(0.0);

	let wa = w.frequency * w.amplitude * ocean.storm_intensity;
	let s = sin(w.frequency * phi + t * t);
	let c = cos(w.frequency * phi + t);

	n.x = d.x * wa * c;
	n.y = d.y * wa * c;
	n.z = w.steepness * wa * s;

	return n;
}

@fragment
fn fragment(
	in: VertexOutput,
	@builtin(front_facing) is_front: bool,
) -> FragmentOutput {
// --- Modified from bevy extended_material example ---

	// generate a PbrInput struct from the StandardMaterial bindings
	var pbr_input = pbr_input_from_standard_material(in, is_front);

//	// we can optionally modify the input before lighting and alpha_discard is applied
//	pbr_input.material.base_color.b = pbr_input.material.base_color.r;

	// alpha discard
	pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
	// in deferred mode we can't modify anything after that, as lighting is run in a separate fullscreen shader.
	let out = deferred_output(in, pbr_input);
#else
	// in forward mode, we calculate the lit color immediately, and then apply some post-lighting effects here.
	// in deferred mode the lit color and these effects will be calculated in the deferred lighting shader
	var out: FragmentOutput;
	if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
		out.color = apply_pbr_lighting(pbr_input);
	} else {
		out.color = pbr_input.material.base_color;
	}

//	// we can optionally modify the lit color before post-processing is applied
//	out.color = vec4<f32>(vec4<u32>(out.color * f32(my_extended_material.quantize_steps))) / f32(my_extended_material.quantize_steps);

	// apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
	// note this does not include fullscreen postprocessing effects like bloom.
	out.color = main_pass_post_lighting_processing(pbr_input, out.color);

//	// we can optionally modify the final result here
//	out.color = out.color * 2.0;
#endif
	let prepass_depth = bevy_pbr::prepass_utils::prepass_depth(in.position, 0u) / in.position.w;
	let ocean_z = in.position.z / in.position.w;
	let ocean_depth = (ocean_z - prepass_depth) / in.position.w;
	let a = pow(ocean_depth, 3.0) * 0.2;
	out.color.a = min(out.color.a + a + 0.01, 1.0);
	return out;
}

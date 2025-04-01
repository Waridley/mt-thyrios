#import bevy_pbr::{
	pbr_fragment::pbr_input_from_standard_material,
	pbr_functions::{alpha_discard, calculate_view},
	pbr_bindings::{base_color_texture, base_color_sampler},
	mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_normal_local_to_world},
	view_transformations::position_world_to_clip,
	mesh_view_bindings::view,
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
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#endif

const NUM_KINDS: u32 = 6;

@group(2) @binding(101) var textures: texture_2d_array<f32>;
@group(2) @binding(102) var textures_sampler: sampler;


struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(8) terrain_weights_0_3: vec4<f32>,
    @location(9) terrain_weights_4_7: vec4<f32>,
};

struct TriplanarVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(6) @interpolate(flat) instance_index: u32,
    @location(8) terrain_weights_0_3: vec4<f32>,
    @location(9) terrain_weights_4_7: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> TriplanarVertexOutput {
    var out: TriplanarVertexOutput;
    let world_from_local = get_world_from_local(vertex.instance_index);
    out.world_position = mesh_position_local_to_world(
    	world_from_local,
    	vec4<f32>(vertex.position, 1.0),
    );
    out.clip_position = position_world_to_clip(out.world_position.xyz);
    out.world_normal = mesh_normal_local_to_world(vertex.normal, 0u);
    out.instance_index = vertex.instance_index;

    out.terrain_weights_0_3 = vertex.terrain_weights_0_3;
    out.terrain_weights_4_7 = vertex.terrain_weights_4_7;

    return out;
}

@fragment
fn fragment(
	in: VertexOutput,
    @location(8) terrain_weights_0_3: vec4<f32>,
    @location(9) terrain_weights_4_7: vec4<f32>,
	@builtin(front_facing) is_front: bool,
) -> FragmentOutput {
	var pbr_input = pbr_input_from_standard_material(in, is_front);

	// Texture splatting
	#ifdef TRIPLANAR
	let n = in.world_normal;
	let x = abs(dot(n, vec3(1.0, 0.0, 0.0)));
	let y = abs(dot(n, vec3(0.0, 1.0, 0.0)));
	let z = abs(dot(n, vec3(0.0, 0.0, 1.0)));
	#endif
	let width = f32(textureDimensions(textures).x);
	let coord = (in.world_position.xyz * 64.0) / width;

	let uv = (coord + vec3(1.0)) * 0.5;

	var color = vec4(0.0, 0.0, 0.0, 1.0);
	var weights: array<f32, NUM_KINDS>;
	for (var i = 0u; i < NUM_KINDS; i++) {
		var weight = 0.0;
		if i < 4 {
			weight = terrain_weights_0_3[i];
		} else if i < 8 {
			weight = terrain_weights_4_7[i - 4];
		}
		weights[i] = weight;
	}
	for (var i = 0u; i < NUM_KINDS; i++) {
		#ifdef TRIPLANAR
			let x_color = textureSample(textures, textures_sampler, vec2(uv.y, uv.z), i);
			let y_color = textureSample(textures, textures_sampler, vec2(uv.x, uv.z), i);
			let z_color = textureSample(textures, textures_sampler, vec2(uv.x, uv.y), i);
			color += ((x * x_color) + (y * y_color) + (z * z_color)) * weights[i];
		#else
			color += textureSample(textures, textures_sampler, vec2(uv.x, uv.y), i) * weights[i];
		#endif
	}
	pbr_input.material.base_color *= color;

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
	
	return out;
}

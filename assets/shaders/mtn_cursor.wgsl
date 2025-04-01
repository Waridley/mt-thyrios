#import bevy_pbr::{
    prepass_utils::prepass_depth,
}
#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
}
#endif

struct MtnCursorMaterial {
	base_color: vec4<f32>,
	intersection_depth_mul: f32,
}

@group(2) @binding(0) var<uniform> material: MtnCursorMaterial;
@group(2) @binding(1) var base_color_texture: texture_2d<f32>;
@group(2) @binding(2) var base_color_sampler: sampler;
@group(2) @binding(3) var intersection_depth_map: texture_1d<f32>;
@group(2) @binding(4) var intersection_depth_sampler: sampler;

@fragment
fn fragment(
	in: VertexOutput,
) -> FragmentOutput {
	var color = textureSample(base_color_texture, base_color_sampler, in.uv) * material.base_color;

	let prepass_depth = prepass_depth(in.position, 0u);
	let unscaled_pre_depth = prepass_depth / in.position.w;
	let cursor_z = in.position.z / in.position.w;
	let intersection_depth = (cursor_z - unscaled_pre_depth) / in.position.w;
	let intersection_color = textureSample(intersection_depth_map, intersection_depth_sampler, intersection_depth * material.intersection_depth_mul);
	color = vec4(color.rgb * color.a, 0.0);
	color = color + vec4(intersection_color.rgb * intersection_color.a, 0.0);

	var out: FragmentOutput;
	out.color = color;
	return out;
}

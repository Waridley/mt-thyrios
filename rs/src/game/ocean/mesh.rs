use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use std::f32::consts::TAU;

pub fn generate_ocean_mesh(radius: f32, rings: u32, wedges: u32) -> Mesh {
	let mut verts = Vec::new();
	let mut indices = Indices::U16(Vec::new());

	let center_bias = EasingCurve::new(0.0, radius, EaseFunction::CircularIn);

	verts.push(Vec3::ZERO);
	let r = center_bias.sample(1.0 / rings as f32).unwrap();
	for wedge in 0..wedges {
		let theta = TAU * (wedge as f32) / (wedges as f32);
		let x = theta.cos() * r;
		let y = theta.sin() * r;
		verts.push(Vec3::new(x, y, 0.0));
		indices.push(wedge + 1);
		indices.push(((wedge + 1) % wedges) + 1);
		indices.push(0);
	}
	for ring in 2..=rings {
		let r = center_bias.sample(ring as f32 / rings as f32).unwrap();
		for wedge in 0..wedges {
			let theta = TAU * (wedge as f32) / (wedges as f32);
			let x = theta.cos() * r;
			let y = theta.sin() * r;
			verts.push(Vec3::new(x, y, 0.0));
			let ia = ((ring - 1) * wedges) + wedge + 1;
			let ib = ((ring - 2) * wedges) + ((wedge + 1) % wedges) + 1;
			let ic = ((ring - 2) * wedges) + wedge + 1;
			indices.push(ia);
			indices.push(ib);
			indices.push(ic);
			let ic = ib;
			let ib = ((ring - 1) * wedges) + ((wedge + 1) % wedges) + 1;
			indices.push(ia);
			indices.push(ib);
			indices.push(ic);
		}
	}
	let len = verts.len();
	let mut mesh = Mesh::new(
		PrimitiveTopology::TriangleList,
		RenderAssetUsages::RENDER_WORLD,
	);
	mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verts);
	mesh.insert_indices(indices);
	mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![Vec3::Z; len]);
	mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Vec4::ONE; len]);
	mesh
}

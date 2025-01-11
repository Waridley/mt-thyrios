use bevy::render::mesh::Indices;
use bevy::{
	asset::RenderAssetUsages, math::primitives::Plane3d, prelude::*,
	render::mesh::PrimitiveTopology,
};
use std::cmp::Ordering;
use std::cmp::Ordering::{Equal, Greater, Less};

pub struct CircleGridMeshBuilder {
	pub circle: Circle,
	pub subdivisions: u32,
}

impl CircleGridMeshBuilder {
	pub fn subdivisions(mut self, subdivisions: u32) -> Self {
		self.subdivisions = subdivisions;
		self
	}
}

impl MeshBuilder for CircleGridMeshBuilder {
	fn build(&self) -> Mesh {
		let mut mesh = Mesh::new(
			PrimitiveTopology::TriangleList,
			RenderAssetUsages::default(),
		);
		let mut verts = Vec::with_capacity((self.subdivisions as usize + 1) * 4);
		let x_verts = self.subdivisions + 2;
		let y_verts = self.subdivisions + 2;
		let cap = ((x_verts as f64 * y_verts as f64) * std::f64::consts::FRAC_PI_4).ceil() as usize;
		let mut indices = if x_verts < 288 {
			Indices::U16(Vec::with_capacity(cap))
		} else {
			Indices::U32(Vec::with_capacity(cap))
		};
		let mut grid = vec![vec![None; x_verts as usize]; y_verts as usize];
		let slices = self.subdivisions + 1;
		let r = self.circle.radius;
		let d = r * 2.0;
		for y in 0..y_verts {
			for x in 0..x_verts {
				let mut cell: &mut Option<u32> = &mut grid[y as usize][x as usize];
				let y = ((y as f32 * d) / slices as f32) - r;
				let x = ((x as f32 * d) / slices as f32) - r;
				if Vec2::new(x, y).length_squared() <= r * r {
					*cell = Some(verts.len() as u32);
					verts.push([x, y, 0.0]);
				}
			}
		}
		use Ordering::*;
		enum Dir {
			Left,
			Right,
			Both,
		}
		for y in 0..slices {
			for x in 0..x_verts {
				if let Some(ia) = grid[y as usize][x as usize] {
					let dir = match (y.cmp(&(y_verts / 2)), x.cmp(&(x_verts / 2))) {
						// bottom-left | top-right
						(Less, Less) | (Greater, Greater) | (Equal, Greater) => Dir::Left,
						// bottom-right | top-left
						(Less, Greater) | (Greater, Less) | (Equal, Less) => Dir::Right,
						// x direction is outward for -y but inward for +y
						(Less, Equal) => Dir::Both,
						(Greater, Equal) | (Equal, Equal) => continue,
					};
					let (y, x) = (y as usize, x as usize);
					let tris: &[((isize, isize), (isize, isize))] = match dir {
						Dir::Left => {
							if x == 0 {
								continue;
							} else {
								&[((1, -1), (0, -1)), ((1, 0), (1, -1))]
							}
						}
						Dir::Right => {
							if x >= (x_verts - 1) as usize {
								continue;
							} else {
								&[((1, 1), (1, 0)), ((0, 1), (1, 1))]
							}
						}
						Dir::Both => &[
							((1, -1), (0, -1)),
							((1, 0), (1, -1)),
							((1, 1), (1, 0)),
							((0, 1), (1, 1)),
						],
					};
					for &((by, bx), (cy, cx)) in tris.iter() {
						let by = y.checked_add_signed(by).unwrap();
						let bx = x.checked_add_signed(bx).unwrap();
						let cy = y.checked_add_signed(cy).unwrap();
						let cx = x.checked_add_signed(cx).unwrap();
						if let Some((ib, ic)) = grid[by][bx].zip(grid[cy][cx]) {
							indices.push(ia);
							indices.push(ib);
							indices.push(ic);
						}
					}
				}
			}
		}
		mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verts);
		mesh.insert_indices(indices);
		mesh
	}
}

/// Trait for subdividing a mesh as a grid instead of the default `Meshable` implementation for a shape.
pub trait GridMesh {
	type Builder: MeshBuilder;
	fn grid_mesh(&self) -> Self::Builder;
}

impl GridMesh for Plane3d {
	type Builder = <Plane3d as Meshable>::Output;

	fn grid_mesh(&self) -> Self::Builder {
		self.mesh()
	}
}

impl GridMesh for Circle {
	type Builder = CircleGridMeshBuilder;

	fn grid_mesh(&self) -> Self::Builder {
		CircleGridMeshBuilder {
			circle: *self,
			subdivisions: 0,
		}
	}
}

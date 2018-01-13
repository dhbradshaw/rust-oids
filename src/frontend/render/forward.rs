use std::result;
use cgmath;
use gfx;
use gfx::traits::FactoryExt;
use frontend::render::Result;
use frontend::render::RenderFactoryExt;
use frontend::render::formats;
use core::resource;

gfx_vertex_struct!(VertexPosNormal {
	pos: [f32; 3] = "a_Pos",
	normal: [f32; 3] = "a_Normal",
	tangent: [f32; 3] = "a_Tangent",
	tex_coord: [f32; 2] = "a_TexCoord",
});

impl Default for VertexPosNormal {
	fn default() -> Self {
		VertexPosNormal {
			pos: [0.; 3],
			normal: [0., 0., 1.],
			tangent: [1., 0., 0.],
			tex_coord: [0.5, 0.5],
		}
	}
}

pub type Vertex = VertexPosNormal;
pub type M44 = cgmath::Matrix4<f32>;

const MAX_NUM_TOTAL_LIGHTS: usize = 16;

gfx_defines!(
    constant PointLight {
        propagation: [f32; 4] = "propagation",
        center: [f32; 4] = "center",
        color: [f32; 4] = "color",
    }

    constant CameraArgs {
        proj: [[f32; 4]; 4] = "u_Proj",
        view: [[f32; 4]; 4] = "u_View",
    }

    constant ModelArgs {
        model: [[f32; 4]; 4] = "u_Model",
    }

    constant FragmentArgs {
        light_count: i32 = "u_LightCount",
    }

	constant MaterialArgs {
		emissive: [f32; 4] = "u_Emissive",
		effect: [f32; 4] = "u_Effect",
	}

    pipeline shaded {
        vbuf: gfx::VertexBuffer<VertexPosNormal> = (),
        camera_args: gfx::ConstantBuffer<CameraArgs> = "cb_CameraArgs",
        model_args: gfx::ConstantBuffer<ModelArgs> = "cb_ModelArgs",
        fragment_args: gfx::ConstantBuffer<FragmentArgs> = "cb_FragmentArgs",
        material_args: gfx::ConstantBuffer<MaterialArgs> = "cb_MaterialArgs",
        lights: gfx::ConstantBuffer<PointLight> = "u_Lights",
        color_target: gfx::BlendTarget<formats::RenderColorFormat> = ("o_Color", gfx::state::MASK_ALL, gfx::preset::blend::ADD),
        depth_target: gfx::DepthTarget<formats::RenderDepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }

    pipeline blend {
        vbuf: gfx::VertexBuffer<VertexPosNormal> = (),
        camera_args: gfx::ConstantBuffer<CameraArgs> = "cb_CameraArgs",
        model_args: gfx::ConstantBuffer<ModelArgs> = "cb_ModelArgs",
        fragment_args: gfx::ConstantBuffer<FragmentArgs> = "cb_FragmentArgs",
        material_args: gfx::ConstantBuffer<MaterialArgs> = "cb_MaterialArgs",
        lights: gfx::ConstantBuffer<PointLight> = "u_Lights",
        color_target: gfx::BlendTarget<formats::RenderColorFormat> = ("o_Color", gfx::state::MASK_ALL, gfx::preset::blend::ALPHA),
        depth_target: gfx::DepthTarget<formats::RenderDepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }

);

pub type ShadedInit<'f> = shaded::Init<'f>;

use std::marker::PhantomData;

pub enum Shader {
	Ball = 0,
	Flat = 1,
	Wireframe = 2,
	Lines = 3,
	DebugLines = 4,
	Count = 5,
}

pub struct ForwardLighting<R: gfx::Resources, C: gfx::CommandBuffer<R>, D>
	where D: gfx::pso::PipelineInit {
	camera: gfx::handle::Buffer<R, CameraArgs>,
	model: gfx::handle::Buffer<R, ModelArgs>,
	fragment: gfx::handle::Buffer<R, FragmentArgs>,
	material: gfx::handle::Buffer<R, MaterialArgs>,
	lights: gfx::handle::Buffer<R, PointLight>,
	pso: [gfx::pso::PipelineState<R, D::Meta>; Shader::Count as usize],
	_buffer: PhantomData<C>,
}

impl<R: gfx::Resources, C: gfx::CommandBuffer<R>, D> ForwardLighting<R, C, D>
	where D: gfx::pso::PipelineInit + Clone {
	pub fn new<F>(factory: &mut F, res: &resource::ResourceLoader<u8>, init: D) -> Result<ForwardLighting<R, C, D>>
		where
			F: gfx::Factory<R>, {
		let lights = factory.create_constant_buffer(MAX_NUM_TOTAL_LIGHTS);
		let camera = factory.create_constant_buffer(1);
		let model = factory.create_constant_buffer(1);
		let fragment = factory.create_constant_buffer(1);
		let material = factory.create_constant_buffer(1);

		macro_rules! load_shaders {
			($v:expr, $f:expr) => { factory.create_shader_set(
					&res.load(concat!("shaders/forward/", $v, ".vert"))?,
					&res.load(concat!("shaders/forward/", $f, ".frag"))?) };

			($g:expr, $v:expr, $f:expr) => { factory.create_shader_set_with_geometry(
					&res.load(concat!("shaders/forward/", $g, ".geom"))?,
					&res.load(concat!("shaders/forward/", $v, ".vert"))?,
					&res.load(concat!("shaders/forward/", $f, ".frag"))?)
				 }
		};

		let flat_shaders = load_shaders!("lighting", "lighting_flat")?;
		let solid_shaders = load_shaders!("lighting", "lighting_poly")?;
		let ball_shaders = load_shaders!("point_ball", "lighting", "lighting_poly")?;

		let solid_rasterizer = gfx::state::Rasterizer {
			samples: Some(gfx::state::MultiSample),
			..gfx::state::Rasterizer::new_fill()
		};

		let line_rasterizer = gfx::state::Rasterizer {
			method: gfx::state::RasterMethod::Line(2),
			..solid_rasterizer
		};
		let debug_line_rasterizer = gfx::state::Rasterizer {
			method: gfx::state::RasterMethod::Line(1),
			..solid_rasterizer
		};

		let ball_pso = Self::new_pso(
			factory,
			&ball_shaders,
			gfx::Primitive::TriangleList,
			solid_rasterizer,
			init.clone(),
		)?;
		let poly_pso = Self::new_pso(
			factory,
			&solid_shaders,
			gfx::Primitive::TriangleList,
			solid_rasterizer,
			init.clone(),
		)?;
		let wireframe_pso = Self::new_pso(
			factory,
			&solid_shaders,
			gfx::Primitive::TriangleList,
			line_rasterizer,
			init.clone(),
		)?;
		let lines_pso = Self::new_pso(
			factory,
			&flat_shaders,
			gfx::Primitive::LineStrip,
			line_rasterizer,
			init.clone(),
		)?;
		let debug_lines_pso = Self::new_pso(
			factory,
			&flat_shaders,
			gfx::Primitive::LineStrip,
			debug_line_rasterizer,
			init.clone(),
		)?;
		Ok(ForwardLighting {
			camera,
			model,
			fragment,
			material,
			lights,
			pso: [
				ball_pso,
				poly_pso,
				wireframe_pso,
				lines_pso,
				debug_lines_pso,
			],
			_buffer: PhantomData,
		})
	}

	fn new_pso<F>(factory: &mut F, shaders: &gfx::ShaderSet<R>, primitive: gfx::Primitive, rasterizer: gfx::state::Rasterizer, init: D)
				  -> result::Result<gfx::pso::PipelineState<R, D::Meta>, gfx::PipelineStateError<String>>
		where
			F: gfx::Factory<R>, {
		factory.create_pipeline_state(&shaders, primitive, rasterizer, init)
	}

	pub fn setup(&self, encoder: &mut gfx::Encoder<R, C>, camera_projection: M44, camera_view: M44, lights: &Vec<PointLight>) {
		let mut lights_buf = lights.clone();

		let count = lights_buf.len();
		while lights_buf.len() < MAX_NUM_TOTAL_LIGHTS {
			lights_buf.push(PointLight {
				propagation: [0., 0., 0., 0.],
				color: [0., 0., 0., 0.],
				center: [0., 0., 0., 0.],
			})
		}

		if let Ok(_) = encoder.update_buffer(&self.lights, &lights_buf[..], 0) {
			encoder.update_constant_buffer(
				&self.camera,
				&CameraArgs {
					proj: camera_projection.into(),
					view: camera_view.into(),
				},
			);
			encoder.update_constant_buffer(&self.fragment, &FragmentArgs { light_count: count as i32 });
		}
	}
}

impl<R: gfx::Resources, C: gfx::CommandBuffer<R>> ForwardLighting<R, C, shaded::Init<'static>> {
	pub fn draw_primitives(
		&self, shader: Shader, encoder: &mut gfx::Encoder<R, C>, vertices: gfx::handle::Buffer<R, VertexPosNormal>,
		indices: &gfx::Slice<R>, transform: &M44, color: [f32; 4], effect: [f32; 4],
		color_buffer: &gfx::handle::RenderTargetView<R, formats::RenderColorFormat>,
		depth_buffer: &gfx::handle::DepthStencilView<R, formats::RenderDepthFormat>,
	) {
		encoder.update_constant_buffer(&self.model, &ModelArgs { model: (*transform).into() });
		encoder.update_constant_buffer(
			&self.material,
			&MaterialArgs {
				emissive: color,
				effect,
			},
		);
		encoder.draw(
			indices,
			&self.pso[shader as usize],
			&shaded::Data {
				vbuf: vertices,
				fragment_args: self.fragment.clone(),
				material_args: self.material.clone(),
				camera_args: self.camera.clone(),
				model_args: self.model.clone(),
				lights: self.lights.clone(),
				color_target: color_buffer.clone(),
				depth_target: depth_buffer.clone(),
			},
		);
	}
}

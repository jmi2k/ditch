use std::{mem::size_of, collections::HashMap, sync::Arc, time::Instant};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, IVec2, Vec2, IVec3, ivec3, ivec2, vec2};
use image::{RgbaImage, imageops::FilterType};
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    vertex_attr_array, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferBindingType, BufferDescriptor,
    BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, CompareFunction,
    DepthBiasState, DepthStencilState, Extent3d, Face, FragmentState, FrontFace, LoadOp,
    MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, StencilState,
    SurfaceError, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, VertexBufferLayout, VertexState, VertexStepMode, PushConstantRange, IndexFormat, RenderBundle, RenderBundleEncoder, RenderBundleEncoderDescriptor, RenderBundleDescriptor, RenderBundleDepthStencil, ImageCopyTexture, ImageDataLayout, SamplerDescriptor, AddressMode, FilterMode, TextureViewDimension, TextureSampleType, BindingResource,
};

use crate::{graphics::{Camera, GraphicsContext}, chunk, assets::N_MIPS};

const N_SAMPLES: usize = 1;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PushConstants {
    camera: Mat4,
    viewport: Vec2,
    time: f32,
    padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub xyz: Vec3,
    pub uv: Vec2,
    pub shadow: f32,
    pub light: u32,
}

impl Vertex {
    const BUFFER_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as _,
        step_mode: VertexStepMode::Vertex,
        attributes: &vertex_attr_array![
            0 => Float32x3,
            1 => Float32x2,
            2 => Float32,
            3 => Uint32,
        ],
    };
}

pub struct WorldRenderer {
    epoch: Instant,
    depth_texture: Texture,
    msaa_texture: Texture,
    atlas_bind_group: BindGroup,
    vertex_buffers: HashMap<IVec3, (u32, Buffer, Buffer)>,
    pipeline: RenderPipeline,
}

impl WorldRenderer {
    pub fn new(graphics_context: &GraphicsContext, atlases: &[RgbaImage; N_MIPS]) -> Self {
        let GraphicsContext { device, config, .. } = graphics_context;

        let depth_texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: N_SAMPLES as _,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let msaa_texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: N_SAMPLES as _,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let atlas_size = Extent3d {
            width: atlases[0].width(),
            height: atlases[0].height(),
            depth_or_array_layers: 1,
        };

        let atlas_texture = device.create_texture(
            &TextureDescriptor {
                // All textures are stored as 3D, we represent our 2D texture
                // by setting depth to 1.
                size: atlas_size,
                mip_level_count: N_MIPS as _, // We'll talk about this a little later
                sample_count: 1,
                dimension: TextureDimension::D2,
                // Most images are stored using sRGB so we need to reflect that here.
                format: TextureFormat::Rgba8UnormSrgb,
                // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
                // COPY_DST means that we want to copy data to this texture
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                label: None,
                // This is the same as with the SurfaceConfig. It
                // specifies what texture formats can be used to
                // create TextureViews for this texture. The base
                // texture format (Rgba8UnormSrgb in this case) is
                // always supported. Note that using a different
                // texture format is not supported on the WebGL2
                // backend.
                view_formats: &[],
            }
        );


        for mip_lvl in 0..N_MIPS {
            let atlas = &atlases[mip_lvl];

            let atlas_size = Extent3d {
                width: atlas.width(),
                height: atlas.height(),
                depth_or_array_layers: 1,
            };

            graphics_context.queue.write_texture(
                // Tells wgpu where to copy the pixel data
                ImageCopyTexture {
                    texture: &atlas_texture,
                    mip_level: mip_lvl as _,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                // The actual pixel data
                atlas,
                // The layout of the texture
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * atlas.width()),
                    rows_per_image: Some(atlas.height()),
                },
                atlas_size,
            );

        }

        let atlas_texture_view = atlas_texture.create_view(&TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Linear,
            ..SamplerDescriptor::default()
        });

        let texture_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
                    ty: BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: None,
        });

        let atlas_bind_group = {
            device.create_bind_group(
                &BindGroupDescriptor {
                    layout: &texture_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&atlas_texture_view),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&atlas_sampler),
                        }
                    ],
                    label: None,
                }
            )
        };

        let pipeline = {
            let shader = device.create_shader_module(include_wgsl!("../../shader.wgsl"));

            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::VERTEX,
                    range: 0..128,
                }],
            });

            let primitive = PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                cull_mode: Some(Face::Back),
                front_face: FrontFace::Ccw,
                polygon_mode: PolygonMode::Fill,
                ..Default::default()
            };

            let vertex = VertexState {
                module: &shader,
                entry_point: "vertex",
                buffers: &[Vertex::BUFFER_LAYOUT],
            };

            let fragment = FragmentState {
                module: &shader,
                entry_point: "fragment",
                targets: &[Some(ColorTargetState {
                    //format: config.format,
                    format: TextureFormat::Bgra8UnormSrgb,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            };

            let depth_stencil = DepthStencilState {
                format: depth_texture.format(),
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            };

            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: None,
                layout: Some(&layout),
                primitive,
                vertex,
                fragment: Some(fragment),
                depth_stencil: Some(depth_stencil),
                multisample: MultisampleState { count: N_SAMPLES as _, mask: !0, alpha_to_coverage_enabled: false },
                multiview: None,
            })
        };

        Self {
            epoch: Instant::now(),
            pipeline,
            depth_texture,
            msaa_texture,
            atlas_bind_group,
            vertex_buffers: HashMap::default(),
        }
    }

    pub fn add_vertices(&mut self, graphics_context: &GraphicsContext, location: IVec3, mesh: &Arc<(u32, Vec<Vertex>, Vec<u32>)>) {
        if let Some(entry) = self.vertex_buffers.get(&location) {
            if entry.0 == mesh.0 {
                return;
            }
        }

        let (_, ref vertices, ref indices) = **mesh;

        if vertices.is_empty() {
            return;
        }

        let vertex_buffer = graphics_context
            .device
            .create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(vertices),
                usage: BufferUsages::VERTEX,
            });

        let index_buffer = graphics_context
            .device
            .create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(indices),
                usage: BufferUsages::INDEX,
            });

        self.vertex_buffers.insert(location, (mesh.0, vertex_buffer, index_buffer));
    }

    pub fn remove_vertices(&mut self, location: IVec3, distance: i32) {
        let location: IVec3 = location >> 5;
        let to_be_removed = self.vertex_buffers.keys().filter(|loc| location.distance_squared(**loc) >= distance*distance).cloned().collect::<Vec<_>>();

        for chunk_loc in to_be_removed {
            self.vertex_buffers.remove(&chunk_loc);
        }
    }

    pub fn render(
        &mut self,
        graphics_context: &GraphicsContext,
        camera: Camera,
    ) -> Result<(), SurfaceError> {
        let output = graphics_context.surface.get_current_texture()?;

        if output.texture.size() != self.depth_texture.size() {
            self.depth_texture = graphics_context.device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: output.texture.size().width,
                    height: output.texture.size().height,
                    depth_or_array_layers: self.depth_texture.depth_or_array_layers(),
                },
                mip_level_count: self.depth_texture.mip_level_count(),
                sample_count: self.depth_texture.sample_count(),
                dimension: self.depth_texture.dimension(),
                format: self.depth_texture.format(),
                usage: self.depth_texture.usage(),
                view_formats: &[],
            });

            self.msaa_texture = graphics_context.device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: output.texture.size().width,
                    height: output.texture.size().height,
                    depth_or_array_layers: self.msaa_texture.depth_or_array_layers(),
                },
                mip_level_count: self.msaa_texture.mip_level_count(),
                sample_count: self.msaa_texture.sample_count(),
                dimension: self.msaa_texture.dimension(),
                format: self.msaa_texture.format(),
                usage: self.msaa_texture.usage(),
                view_formats: &[],
            });
        }

        let msaa_view = self
            .msaa_texture
            .create_view(&TextureViewDescriptor::default());

        let output_view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let depth_view = self
            .depth_texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = graphics_context
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        let mut render_pass = {
            let color_attachment = RenderPassColorAttachment {
                view: if N_SAMPLES > 1 { &msaa_view } else { &output_view },
                resolve_target: if N_SAMPLES > 1 { Some(&output_view) } else { None },
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.527,
                        g: 0.805,
                        b: 0.918,
                        a: 1.,
                    }),
                    store: true,
                },
            };

            let depth_attachment = RenderPassDepthStencilAttachment {
                view: &depth_view,
                stencil_ops: None,
                depth_ops: Some(wgpu::Operations {
                    load: LoadOp::Clear(1.),
                    store: true,
                }),
            };

            encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: Some(depth_attachment),
            })
        };

        let size = output.texture.size();
        let viewport = vec2(size.width as _, size.height as _);

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::cast_slice(&[PushConstants {
            camera: Mat4::from(camera),
            viewport,
            time: self.epoch.elapsed().as_secs_f32(),
            padding: 0,
        }]));
        render_pass.set_bind_group(0, &self.atlas_bind_group, &[]);

        for (_, vertex_buffer, index_buffer) in self.vertex_buffers.values() {
            let index_count = index_buffer.size() / size_of::<u32>() as u64;

            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint32);
            render_pass.draw_indexed(0..index_count as u32, 0, 0..1);
        }

        drop(render_pass);
        graphics_context.queue.submit([encoder.finish()]);
        output.present();

        Ok(())
    }
}

pub mod camera;
pub use camera::{Camera, Pov, Projection};

pub mod render;
pub use render::{Vertex, WorldRenderer};

use wgpu::{
    Backends, Device, DeviceDescriptor, Dx12Compiler, Instance, InstanceDescriptor,
    PowerPreference, PresentMode, Queue, RequestAdapterOptions, Surface, SurfaceCapabilities,
    SurfaceConfiguration, TextureFormat, TextureUsages, Features, Limits,
};
use winit::{dpi::PhysicalSize, window::Window};

pub struct GraphicsContext {
    pub surface: Surface,
    pub config: SurfaceConfiguration,
    pub device: Device,
    pub queue: Queue,
}

impl GraphicsContext {
    pub async fn new(window: &Window) -> Self {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            dx12_shader_compiler: Dx12Compiler::default(),
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                features: Features::PUSH_CONSTANTS | Features::POLYGON_MODE_LINE | Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                limits: Limits {
                    max_push_constant_size: 256,
                    ..Limits::default()
                },
                ..DeviceDescriptor::default()
            }, None)
            .await
            .unwrap();

        let config = {
            let SurfaceCapabilities {
                formats,
                alpha_modes,
                ..
            } = surface.get_capabilities(&adapter);

            let PhysicalSize { width, height } = window.inner_size();

            let format = formats
                .iter()
                .copied()
                .find(TextureFormat::is_srgb)
                .unwrap();

            SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                width,
                height,
                format,
                present_mode: PresentMode::AutoVsync,
                alpha_mode: alpha_modes[0],
                view_formats: vec![],
            }
        };

        surface.configure(&device, &config);

        Self {
            surface,
            config,
            device,
            queue,
        }
    }

    pub fn resize_viewport(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }
}

use crate::{
    collections::{
        structs::{BindGroups, Buffers, Params, Pipelines},
        vertices::VERTICES,
    },
    init::init_functions::{
        init_bind_groups, init_buffers, init_params, init_pipelines, init_shader_modules,
        init_textures,
    },
    updates::param_updates::{update_cpu_read_buffers, update_view_params_buffer},
};
use std::sync::Arc;

use super::controls::{update_controls, KeyboardState};

#[derive(Debug)]
pub(crate) struct State<'a> {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) surface: wgpu::Surface<'a>,
    pub(crate) surface_config: wgpu::SurfaceConfiguration,
    pub(crate) size: winit::dpi::PhysicalSize<u32>,
    pub(crate) params: Params,
    pub(crate) buffers: Buffers,
    pub(crate) bind_groups: BindGroups,
    pub(crate) pipelines: Pipelines,
    pub(crate) controls: KeyboardState,
    pub(crate) app_time: std::time::Instant,
    // Keep window at the bottom,
    // must be dropped after surface
    pub(crate) window: std::sync::Arc<winit::window::Window>,
}

impl<'a> State<'a> {
    pub(crate) async fn new(window: Arc<winit::window::Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let app_time = std::time::Instant::now();

        // SURFACE
        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("surface init should work");

        // ADAPTER
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("get_dev_storage_texture:: adapter should work");

        let limits = adapter.limits();

        // DEVICE/QUEUE
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("dev_storage_texture_capable Device"),
                    required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                        | wgpu::Features::FLOAT32_FILTERABLE,
                    required_limits: limits,
                },
                None,
            )
            .await
            .expect("get_dev_storage_texture:: device request should work");

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            desired_maximum_frame_latency: 1,
            view_formats: vec![wgpu::TextureFormat::Bgra8UnormSrgb],
            alpha_mode: surface_caps.alpha_modes[0],
        };

        surface.configure(&device, &surface_config);

        let shader_modules = init_shader_modules(&device);
        let params = init_params();
        let buffers = init_buffers(&device, &params);
        let textures = init_textures(&device, &queue);
        let bind_groups = init_bind_groups(&device, &buffers, &textures);
        let pipelines = init_pipelines(&device, &bind_groups, &shader_modules);
        let controls = KeyboardState::new();

        Self {
            device,
            queue,
            surface,
            surface_config,
            size,
            pipelines,
            params,
            buffers,
            bind_groups,
            controls,
            app_time,
            // Keep at bottom, must be dropped after surface
            // and declared after it
            window,
        }
    }

    pub(crate) fn update(&mut self) {
        update_controls(self);
        update_view_params_buffer(self);
        update_cpu_read_buffers(self);
    }

    pub(crate) fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&self.pipelines.render);

            render_pass.set_bind_group(0, &self.bind_groups.uniform_bg, &[]);
            render_pass.set_bind_group(1, &self.bind_groups.frag_bg, &[]);
            render_pass.set_bind_group(2, &self.bind_groups.sampled_texture_bg, &[]);
            render_pass.set_vertex_buffer(0, self.buffers.vertex.slice(..));

            let vertex_range = 0..VERTICES.len() as u32;
            let instance_range = 0..1;
            render_pass.draw(vertex_range, instance_range);
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub(crate) fn get_time(&self) -> f32 {
        self.app_time.elapsed().as_secs_f32()
    }
}

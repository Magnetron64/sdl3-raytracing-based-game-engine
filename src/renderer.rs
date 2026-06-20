use std::error::Error;

use crate::{
    data::*,
    shaders,
    state::{Camera, Player, Quant, Universe},
};
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, SquareMatrix, Vector3};
use glsl_compiler::glsl;
use sdl3::{
    event::{Event, WindowEvent},
    gpu::{
        ColorTargetDescription, ColorTargetInfo, CommandBuffer, CullMode, Device, GraphicsPipeline,
        GraphicsPipelineTargetInfo, RasterizerState, SampleCount, Sampler, SamplerCreateInfo,
        Texture, TextureCreateInfo, TextureFormat, TextureSamplerBinding,
    },
    keyboard::Keycode,
    pixels::Color,
    sys::{keycode::SDL_Keycode, render::SDL_RendererLogicalPresentation},
    video::Window,
};

pub struct Renderer {
    pub gpu: Device,
    pub gpu_texture: Texture<'static>,
    pub sampler: Sampler,
    pub swapchain_format: TextureFormat,
    pub pipeline: GraphicsPipeline,
    pub window_pipeline: GraphicsPipeline,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self, Box<dyn Error>> {
        let mut gpu =
            sdl3::gpu::Device::new(sdl3::gpu::ShaderFormat::SpirV, true)?.with_window(&window)?;
        let (h, w) = window.size();

        let mut gpu_texture = gpu.create_texture(
            TextureCreateInfo::default()
                .with_width(w / 4)
                .with_height(h / 4)
                .with_format(sdl3::gpu::TextureFormat::R8g8b8a8Unorm)
                .with_usage(sdl3::gpu::TextureUsage::Sampler | sdl3::gpu::TextureUsage::ColorTarget)
                .with_type(sdl3::gpu::TextureType::_2D)
                .with_layer_count_or_depth(1)
                .with_num_levels(1)
                .with_sample_count(SampleCount::NoMultiSampling),
        )?;

        let sampler = gpu.create_sampler(
            SamplerCreateInfo::default()
                .with_address_mode_u(sdl3::gpu::SamplerAddressMode::ClampToEdge)
                .with_address_mode_v(sdl3::gpu::SamplerAddressMode::ClampToEdge)
                .with_address_mode_w(sdl3::gpu::SamplerAddressMode::ClampToEdge)
                .with_min_filter(sdl3::gpu::Filter::Nearest)
                .with_mag_filter(sdl3::gpu::Filter::Nearest)
                .with_mipmap_mode(sdl3::gpu::SamplerMipmapMode::Nearest),
        )?;

        let fs_shader = gpu
            .create_shader()
            .with_code(
                sdl3::gpu::ShaderFormat::SpirV,
                shaders::fs(),
                sdl3::gpu::ShaderStage::Fragment,
            )
            .with_storage_buffers(1)
            .with_uniform_buffers(1)
            .build()?;

        let vs_shader = gpu
            .create_shader()
            .with_code(
                sdl3::gpu::ShaderFormat::SpirV,
                shaders::vert(),
                sdl3::gpu::ShaderStage::Vertex,
            )
            .build()?;
        let window_vs_shader = gpu
            .create_shader()
            .with_code(
                sdl3::gpu::ShaderFormat::SpirV,
                shaders::win_vert(),
                sdl3::gpu::ShaderStage::Vertex,
            )
            .build()?;

        let window_fs_shader = gpu
            .create_shader()
            .with_code(
                sdl3::gpu::ShaderFormat::SpirV,
                shaders::win_fs(),
                sdl3::gpu::ShaderStage::Fragment,
            )
            .with_samplers(1)
            .with_uniform_buffers(1)
            .build()?;

        let swapchain_format = gpu.get_swapchain_texture_format(&window);

        let pipeline = gpu
            .create_graphics_pipeline()
            .with_fragment_shader(&fs_shader)
            .with_vertex_shader(&vs_shader)
            .with_primitive_type(sdl3::gpu::PrimitiveType::TriangleList)
            .with_fill_mode(sdl3::gpu::FillMode::Fill)
            .with_rasterizer_state(
                RasterizerState::new()
                    .with_fill_mode(sdl3::gpu::FillMode::Fill)
                    .with_cull_mode(CullMode::Back),
            )
            .with_target_info(
                GraphicsPipelineTargetInfo::new().with_color_target_descriptions(&[
                    ColorTargetDescription::new().with_format(swapchain_format),
                ]),
            )
            .build()?;

        let window_pipeline = gpu
            .create_graphics_pipeline()
            .with_fragment_shader(&window_fs_shader)
            .with_vertex_shader(&window_vs_shader)
            .with_primitive_type(sdl3::gpu::PrimitiveType::TriangleList)
            .with_fill_mode(sdl3::gpu::FillMode::Fill)
            .with_rasterizer_state(
                RasterizerState::new()
                    .with_fill_mode(sdl3::gpu::FillMode::Fill)
                    .with_cull_mode(CullMode::Back),
            )
            .with_target_info(
                GraphicsPipelineTargetInfo::new().with_color_target_descriptions(&[
                    ColorTargetDescription::new().with_format(swapchain_format),
                ]),
            )
            .build()?;

        println!("{:?}", swapchain_format);

        drop(vs_shader);
        drop(fs_shader);

        Ok(Self {
            gpu,
            gpu_texture,
            swapchain_format,
            sampler,
            pipeline,
            window_pipeline,
        })
    }

    pub fn render(
        &mut self,
        aspect: f32,
        window: &Window,
        player: &Player,
        planet_quants: Vec<Quant>,
    ) -> Result<(), Box<dyn Error>> {
        let mut command_buffer = self.gpu.acquire_command_buffer()?;

        let storage_buffer = upload_to_storage(
            &mut self.gpu,
            &mut command_buffer,
            Quants::new(planet_quants),
        )?;

        let color_targets = [ColorTargetInfo::default()
            .with_texture(&self.gpu_texture)
            .with_load_op(sdl3::gpu::LoadOp::Load)
            .with_store_op(sdl3::gpu::StoreOp::Store)
            .with_clear_color(Color::RGB(5, 5, 5))];

        let mut render_pass = self
            .gpu
            .begin_render_pass(&command_buffer, &color_targets, None)?;

        render_pass.bind_graphics_pipeline(&self.pipeline);

        render_pass.bind_fragment_storage_buffers(0, &[storage_buffer]);

        command_buffer.push_fragment_uniform_data(
            0,
            &Constants::new(
                aspect,
                0.1,
                cgmath::num_traits::Pow::pow(0.1, -1) as f32,
                3.4,
                6.0,
                player.cam.pos,
                player.cam.view.invert().unwrap(),
            ),
        );

        render_pass.draw_primitives(6, 1, 0, 0);

        self.gpu.end_render_pass(render_pass);

        if let Ok(swapchain) = command_buffer.wait_and_acquire_swapchain_texture(window) {
            let final_color_targets = [ColorTargetInfo::default()
                .with_texture(&swapchain)
                .with_load_op(sdl3::gpu::LoadOp::Load)
                .with_store_op(sdl3::gpu::StoreOp::Store)
                .with_clear_color(Color::RGB(5, 5, 5))];

            render_pass =
                self.gpu
                    .begin_render_pass(&command_buffer, &final_color_targets, None)?;
            render_pass.bind_graphics_pipeline(&self.window_pipeline);
            let sampler = TextureSamplerBinding::default()
                .with_texture(&self.gpu_texture.clone())
                .with_sampler(&self.sampler.clone());

            command_buffer.push_fragment_uniform_data(0, &sampler);
            render_pass.bind_fragment_samplers(0, &[sampler]);
            render_pass.draw_primitives(6, 1, 0, 0);

            self.gpu.end_render_pass(render_pass);
            command_buffer.submit()?;
        } else {
            println!("failed");
            command_buffer.cancel();
        }

        Ok(())
    }
}

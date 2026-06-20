use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

// use bytemuck::{Pod, Zeroable};
use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, SquareMatrix, Vector3};
use data::{convert_mat4_to_16, upload_to_storage, vec3_to_3, Constants, Quants};
use glsl_compiler::glsl;
use renderer::Renderer;
use sdl3::{
    event::{Event, WindowEvent},
    gpu::{
        ColorTargetDescription, ColorTargetInfo, CommandBuffer, CullMode,
        GraphicsPipelineTargetInfo, RasterizerState, SampleCount, Sampler, SamplerCreateInfo,
        TextureCreateInfo, TextureSamplerBinding,
    },
    keyboard::Keycode,
    pixels::Color,
    sys::{keycode::SDL_Keycode, render::SDL_RendererLogicalPresentation},
};
use state::{Camera, Player, Quant, Universe};

mod data;
mod renderer;
mod shaders;
mod state;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn Error>> {
    let sdl_context = sdl3::init()?;
    let video_subsystem = sdl_context.video()?;
    let mouse_subsystem = sdl_context.mouse();
    let (mut w, mut h) = (800, 600);
    let mut aspect = w as f32 / h as f32;
    let frame_rate = 165;
    let mut physics_delta = Duration::from_millis(33);
    let mut now = Instant::now();
    println!("{:?}", aspect);
    let mut window = video_subsystem
        .window("test", w, h)
        .vulkan()
        .input_grabbed()
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    mouse_subsystem.show_cursor(false);

    let mut gpu_renderer = Renderer::new(&window)?;

    let mut universe = Universe::new();
    let planet = &universe.planets[0];
    let mut camera = Camera::new(planet.pos, 0.1, aspect, 0.3, 0.1);
    let mut player = Player::new(camera, 1.0);

    let mut event_pump = sdl_context.event_pump()?;
    #[allow(clippy::all)]
    'main: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main,
                Event::KeyDown {
                    keycode: Some(Keycode::W),
                    ..
                } => {
                    player.step_forward();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    ..
                } => {
                    player.step_backward();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    ..
                } => {
                    player.step_right();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::D),
                    ..
                } => {
                    player.step_left();
                }
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::Resized(width, height) => {
                        w = width as u32;
                        h = height as u32;
                        aspect = w as f32 / h as f32;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        let relative_state = event_pump.relative_mouse_state();

        mouse_subsystem.warp_mouse_in_window(&window, (w / 2) as f32, (h / 2) as f32);

        let mut planet_quants = vec![];
        if now.elapsed() > physics_delta {
            universe.update(physics_delta);
            now = Instant::now();
        }

        planet_quants = player.get_visible_terrain(&mut universe);

        let dx = relative_state.x();
        let dy = relative_state.y();
        player.cam.rotate(dy, dx);
        player.update(&mut universe);

        gpu_renderer.render(aspect, &window, &player, planet_quants)?;

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / frame_rate));
    }

    Ok(())
}

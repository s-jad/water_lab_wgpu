mod app;
mod init;
mod updates;
use app::state::State;
mod collections;
use collections::consts::{SCREEN_HEIGHT, SCREEN_WIDTH};

use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().expect("event loop should init");
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = WindowBuilder::new()
        .with_title("winit window")
        .with_inner_size(PhysicalSize::new(SCREEN_WIDTH, SCREEN_HEIGHT))
        .build(&event_loop)
        .expect("window should open");

    let mut state = futures::executor::block_on(State::new(window.into()));

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::RedrawRequested => {
                    let elapsed_time = state.get_time();
                    let time_bytes = elapsed_time.to_ne_bytes();
                    state.queue.write_buffer(
                        &state.buffers.time_uniform,
                        0,
                        bytemuck::cast_slice(&[time_bytes]),
                    );

                    state.update();

                    match state.render() {
                        Ok(_) => {}
                        // Reconfigure the surface if lost
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        // The system is out of memory, quit
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            elwt.exit();
                        }
                        // All other errors (Outdated, Timeout) -> resolve by the next frame
                        Err(e) => eprintln!("{:?}", e),
                    };

                    state.window.request_redraw();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    state.controls.handle_keyboard_input(event);
                }
                WindowEvent::Focused(focused) => {
                    if !focused {
                        // Clear the keys HashSet when the window loses focus
                        state.controls.clear_keys();
                        println!("Window lost focus, cleared keys.");
                    }
                }
                _ => {}
            },
            _ => {}
        })
        .expect("event loop should run");
}

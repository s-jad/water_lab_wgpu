use std::collections::HashSet;
use std::thread;
use std::time;

use winit::keyboard::{KeyCode, PhysicalKey};

use crate::updates::param_updates::update_ray_params_buffer;
use crate::updates::param_updates::update_view_params_buffer;

use super::state::State;

#[derive(Debug, Copy, Clone)]
pub(crate) enum KeyboardMode {
    DEBUG,
    VIEW,
    TERRAIN,
    RAY,
    PRINT,
}

#[derive(Debug, Clone)]
pub(crate) struct KeyboardState {
    keys: HashSet<winit::keyboard::PhysicalKey>,
    mode: KeyboardMode,
}

impl KeyboardState {
    pub(crate) fn new() -> Self {
        Self {
            keys: HashSet::new(),
            mode: KeyboardMode::PRINT,
        }
    }

    pub(crate) fn key_pressed(&self, key: winit::keyboard::PhysicalKey) -> bool {
        self.keys.contains(&key)
    }

    pub(crate) fn handle_keyboard_input(&mut self, input: &winit::event::KeyEvent) {
        let key = input.physical_key;
        if input.state == winit::event::ElementState::Pressed {
            self.keys.insert(key);
        } else {
            self.keys.remove(&key);
        }
    }

    pub(crate) fn clear_keys(&mut self) {
        self.keys.clear();
    }

    pub(crate) fn get_keys(&self) -> &HashSet<winit::keyboard::PhysicalKey> {
        &self.keys
    }

    pub(crate) fn get_mode(&self) -> &KeyboardMode {
        &self.mode
    }

    pub(crate) fn set_mode(&mut self, new_mode: KeyboardMode) {
        self.mode = new_mode;
    }
}

pub(crate) fn print_gpu_data<T: bytemuck::Pod + std::fmt::Debug>(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    obj_label: &str,
) {
    // Map the buffer for reading
    let buffer_slice = buffer.slice(..);
    let (tx, rx) = futures::channel::oneshot::channel();

    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });

    println!("buffer size: {:?}", buffer.size());
    // Wait for the GPU to finish executing the commands
    device.poll(wgpu::Maintain::Wait);
    // Wait for the buffer to be mapped
    let result = futures::executor::block_on(rx);

    match result {
        Ok(_) => {
            let buf_view = buffer_slice.get_mapped_range();
            let data: &[T] = bytemuck::cast_slice(&buf_view);

            // Print the boids current properties
            for (i, obj) in data.iter().enumerate() {
                println!("{} {}:\n{:?}", obj_label, i, obj);
            }

            drop(buf_view);
            buffer.unmap();
        }
        Err(e) => eprintln!("Error retrieving gpu data: {:?}", e),
    }
}

pub(crate) fn print_gpu_interleave_two_buffers<
    T: bytemuck::Pod + std::fmt::Debug + std::iter::IntoIterator,
>(
    device: &wgpu::Device,
    buffer1: &wgpu::Buffer,
    buffer2: &wgpu::Buffer,
) where
    <T as IntoIterator>::Item: std::fmt::Debug,
{
    // Map the buffer for reading
    let buffer_slice1 = buffer1.slice(..);
    let (tx1, rx1) = futures::channel::oneshot::channel();

    buffer_slice1.map_async(wgpu::MapMode::Read, move |result| {
        tx1.send(result).unwrap();
    });

    // Wait for the GPU to finish executing the commands
    device.poll(wgpu::Maintain::Wait);
    // Wait for the buffer to be mapped
    let result1 = futures::executor::block_on(rx1);

    // Map the buffer for reading
    let buffer_slice2 = buffer2.slice(..);
    let (tx2, rx2) = futures::channel::oneshot::channel();

    buffer_slice2.map_async(wgpu::MapMode::Read, move |result| {
        tx2.send(result).unwrap();
    });

    // Wait for the GPU to finish executing the commands
    device.poll(wgpu::Maintain::Wait);
    // Wait for the buffer to be mapped
    let result2 = futures::executor::block_on(rx2);

    match (result1, result2) {
        (Ok(_), Ok(_)) => {
            let buf_view1 = buffer_slice1.get_mapped_range();
            let data1: &[T] = bytemuck::cast_slice(&buf_view1);
            let buf_view2 = buffer_slice2.get_mapped_range();
            let data2: &[T] = bytemuck::cast_slice(&buf_view2);

            let mut flattened_data1 = Vec::new();
            let mut flattened_data2 = Vec::new();

            for i in data1.into_iter() {
                flattened_data1.extend(i.to_owned());
            }

            for i in data2.into_iter() {
                flattened_data2.extend(i.to_owned());
            }

            for (idx, item) in flattened_data1
                .iter()
                .zip(flattened_data2.iter())
                .enumerate()
            {
                println!("\n{idx}:\n{:?}", item.0);
                println!("{:?}", item.1);
            }

            drop(buf_view1);
            drop(buf_view2);
            buffer1.unmap();
            buffer2.unmap();
        }
        (Err(e), Ok(_)) => eprintln!("Error retrieving gpu data from buffer1: {:?}", e),
        (Ok(_), Err(e)) => eprintln!("Error retrieving gpu data from buffer2: {:?}", e),
        (Err(e1), Err(e2)) => {
            eprintln!("Error retrieving gpu data from buffer1: {:?}", e1);
            eprintln!("Error retrieving gpu data from buffer2: {:?}", e2);
        }
    }
}

pub(crate) fn update_controls(state: &mut State) {
    if state.controls.key_pressed(PhysicalKey::Code(KeyCode::KeyD)) {
        state.controls.set_mode(KeyboardMode::DEBUG);
    } else if state
        .controls
        .key_pressed(PhysicalKey::Code(KeyCode::Digit1))
    {
        state.controls.set_mode(KeyboardMode::TERRAIN);
    } else if state
        .controls
        .key_pressed(PhysicalKey::Code(KeyCode::Digit2))
    {
        state.controls.set_mode(KeyboardMode::VIEW);
    } else if state
        .controls
        .key_pressed(PhysicalKey::Code(KeyCode::Digit3))
    {
        state.controls.set_mode(KeyboardMode::RAY);
    } else if state.controls.key_pressed(PhysicalKey::Code(KeyCode::KeyP)) {
        state.controls.set_mode(KeyboardMode::PRINT);
    }

    match state.controls.get_mode() {
        KeyboardMode::DEBUG => debug_controls(state),
        KeyboardMode::VIEW => view_controls(state),
        KeyboardMode::TERRAIN => terrain_controls(state),
        KeyboardMode::RAY => ray_controls(state),
        KeyboardMode::PRINT => print_controls(state),
    }
}

fn debug_controls(state: &mut State) {
    let pressed = state.controls.get_keys();

    if pressed.contains(&PhysicalKey::Code(KeyCode::KeyS)) {
        print_gpu_data::<[f32; 4]>(
            &state.device,
            &state.buffers.cpu_read_generic_debug,
            "Debug",
        );
        thread::sleep(time::Duration::from_millis(50));
        state.controls.set_mode(KeyboardMode::VIEW);
    } else if pressed.contains(&PhysicalKey::Code(KeyCode::Digit1)) {
        print_gpu_data::<[[f32; 4]; 512]>(
            &state.device,
            &state.buffers.cpu_read_debug_array1,
            "Debug",
        );
        thread::sleep(time::Duration::from_millis(50));
        state.controls.set_mode(KeyboardMode::VIEW);
    } else if pressed.contains(&PhysicalKey::Code(KeyCode::Digit2)) {
        print_gpu_data::<[[f32; 4]; 512]>(
            &state.device,
            &state.buffers.cpu_read_debug_array2,
            "Debug",
        );
        thread::sleep(time::Duration::from_millis(50));
        state.controls.set_mode(KeyboardMode::VIEW);
    } else if pressed.contains(&PhysicalKey::Code(KeyCode::Digit3)) {
        print_gpu_interleave_two_buffers::<[[f32; 4]; 512]>(
            &state.device,
            &state.buffers.cpu_read_debug_array1,
            &state.buffers.cpu_read_debug_array2,
        );
        thread::sleep(time::Duration::from_millis(50));
        state.controls.set_mode(KeyboardMode::VIEW);
    }
}

fn ray_controls(state: &mut State) {
    let pressed = state.controls.get_keys();
    let mut dval_f = 0.0f32;

    if pressed.contains(&PhysicalKey::Code(KeyCode::ArrowUp)) {
        dval_f = 1.0f32;
    } else if pressed.contains(&PhysicalKey::Code(KeyCode::ArrowDown)) {
        dval_f = -1.0f32;
    }

    if pressed.contains(&PhysicalKey::Code(KeyCode::KeyE)) {
        let maxv = &mut state.params.ray_params.epsilon;
        *maxv = f32::max(0f32, *maxv + (1.0 * dval_f));
        update_ray_params_buffer(state);
    } else if pressed.contains(&PhysicalKey::Code(KeyCode::KeyS)) {
        let maxv = &mut state.params.ray_params.max_steps;
        *maxv = f32::max(0f32, *maxv + (1.0 * dval_f));
        update_ray_params_buffer(state);
    } else if pressed.contains(&PhysicalKey::Code(KeyCode::KeyW)) {
        let maxv = &mut state.params.ray_params.max_dist;
        *maxv = f32::max(0f32, *maxv + (1.0 * dval_f));
        update_ray_params_buffer(state);
    }
}

fn terrain_controls(state: &mut State) {
    let pressed = state.controls.get_keys();
    let mut dval_f = 0.0f32;

    if pressed.contains(&PhysicalKey::Code(KeyCode::ArrowUp)) {
        dval_f = 1.0f32;
    } else if pressed.contains(&PhysicalKey::Code(KeyCode::ArrowDown)) {
        dval_f = -1.0f32;
    }

    println!("terrain controls not done yet");
}

fn view_controls(state: &mut State) {
    let pressed = state.controls.get_keys();
    let mz = state.params.view_params.zoom;

    if pressed.contains(&PhysicalKey::Code(KeyCode::ArrowLeft)) {
        if pressed.contains(&PhysicalKey::Code(KeyCode::ShiftLeft)) {
            state.params.view_params.x_rot = f32::max(0.0, state.params.view_params.x_rot + 0.1);
            update_view_params_buffer(state);
        } else {
            state.params.view_params.x_shift -= 0.01 / mz;
            update_view_params_buffer(state);
        }
    } else if pressed.contains(&PhysicalKey::Code(KeyCode::ArrowRight)) {
        if pressed.contains(&PhysicalKey::Code(KeyCode::ShiftLeft)) {
            state.params.view_params.x_rot -= 0.1;
            update_view_params_buffer(state);
        } else {
            state.params.view_params.x_shift += 0.01 / mz;
            update_view_params_buffer(state);
        }
    } else if pressed.contains(&PhysicalKey::Code(KeyCode::ArrowUp)) {
        if pressed.contains(&PhysicalKey::Code(KeyCode::ShiftLeft)) {
            state.params.view_params.y_rot = f32::max(0.0, state.params.view_params.y_rot + 0.1);
            update_view_params_buffer(state);
        } else {
            state.params.view_params.y_shift -= 0.01 / mz;
            update_view_params_buffer(state);
        }
    } else if pressed.contains(&PhysicalKey::Code(KeyCode::ArrowDown)) {
        if pressed.contains(&PhysicalKey::Code(KeyCode::ShiftLeft)) {
            state.params.view_params.y_rot -= 0.1;
            update_view_params_buffer(state);
        } else {
            state.params.view_params.y_shift += 0.01 / mz;
            update_view_params_buffer(state);
        }
    } else if pressed.contains(&PhysicalKey::Code(KeyCode::KeyX)) {
        state.params.view_params.zoom -= 0.1 * mz;
        update_view_params_buffer(state);
    } else if pressed.contains(&PhysicalKey::Code(KeyCode::KeyZ)) {
        state.params.view_params.zoom += 0.1 * mz;
        update_view_params_buffer(state);
    }
}

fn print_controls(state: &mut State) {
    // PRINT CURRENT PARAMETER VALUES ----------------------------------------------
    println!("\n------------------------------------------------------");
    println!("\n{:#?}", state.params.terrain_params);
    println!("\n{:#?}", state.params.view_params);
    println!("\n{:#?}", state.params.ray_params);
    println!("------------------------------------------------------\n");
    state.controls.mode = KeyboardMode::VIEW;
}

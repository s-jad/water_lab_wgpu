use crate::{
    app::state::State,
    collections::structs::{RayParams, ViewParams},
};

pub(crate) fn update_view_params_buffer(state: &mut State) {
    let new_view_params = ViewParams {
        x_shift: state.params.view_params.x_shift,
        y_shift: state.params.view_params.y_shift,
        zoom: state.params.view_params.zoom,
        x_rot: state.params.view_params.x_rot,
        y_rot: state.params.view_params.y_rot,
        time_modifier: state.params.view_params.time_modifier,
        fov_degrees: state.params.view_params.fov_degrees,
    };

    state.queue.write_buffer(
        &state.buffers.view_params,
        0,
        bytemuck::cast_slice(&[new_view_params]),
    );
}

pub(crate) fn update_ray_params_buffer(state: &mut State) {
    let new_ray_params = RayParams {
        epsilon: state.params.ray_params.epsilon,
        max_dist: state.params.ray_params.max_dist,
        max_steps: state.params.ray_params.max_steps,
    };

    state.queue.write_buffer(
        &state.buffers.ray_params,
        0,
        bytemuck::cast_slice(&[new_ray_params]),
    );
}

pub(crate) fn update_cpu_read_buffers(state: &mut State) {
    let mut encoder = state
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("update_cpu_read_buffers encoder"),
        });

    encoder.copy_buffer_to_buffer(
        &state.buffers.generic_debug,
        0,
        &state.buffers.cpu_read_generic_debug,
        0,
        (std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
    );

    encoder.copy_buffer_to_buffer(
        &state.buffers.debug_array1,
        0,
        &state.buffers.cpu_read_debug_array1,
        0,
        (std::mem::size_of::<[[f32; 4]; 512]>()) as wgpu::BufferAddress,
    );

    encoder.copy_buffer_to_buffer(
        &state.buffers.debug_array2,
        0,
        &state.buffers.cpu_read_debug_array2,
        0,
        (std::mem::size_of::<[[f32; 4]; 512]>()) as wgpu::BufferAddress,
    );

    state.queue.submit(Some(encoder.finish()));
}

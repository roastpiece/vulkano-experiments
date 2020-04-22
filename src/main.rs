#![windows_subsystem = "windows"]

use std::sync::Arc;

mod examples;
use crate::examples::{
    compute_mandel_and_save, compute_shader_multiply, copy_buffers, graphics_pipeline,
    graphics_window, image_clear_and_save, vulkano_particles,
};
use vulkano::device::{Device, DeviceExtensions, Features, Queue};
use vulkano::instance::{Instance, PhysicalDevice};

fn main() {
    let (device, queue, instance) = init_vulkan();

    // copy_buffers(device.clone(), queue.clone());
    //
    // compute_shader_multiply(device.clone(), queue.clone());
    //
    // image_clear_and_save(device.clone(), queue.clone());
    //
    // compute_mandel_and_save(device.clone(), queue.clone());
    //
    // graphics_pipeline(device.clone(), queue.clone());

    // graphics_window(device.clone(), queue.clone(), instance.clone());

    vulkano_particles(device.clone(), queue.clone(), instance.clone());
}

fn init_vulkan() -> (Arc<Device>, Arc<Queue>, Arc<Instance>) {
    println!("INIT VULKAN");

    let instance = Instance::new(None, &vulkano_win::required_extensions(), None)
        .expect("failed to create instance");

    let physical_dev = {
        PhysicalDevice::enumerate(&instance)
            .next()
            .expect("no device available")
    };

    println!("Physical Device: {}", physical_dev.name());

    let queue_family = physical_dev
        .queue_families()
        .find(|&q| q.supports_graphics() && q.supports_compute())
        .expect("couldn't find a queue with graphical and compute capabilities");

    let (device, mut queues) = {
        Device::new(
            physical_dev,
            &Features {
                fill_mode_non_solid: true,
                ..Features::none()
            },
            &DeviceExtensions {
                khr_storage_buffer_storage_class: true,
                khr_swapchain: true,
                ..DeviceExtensions::none()
            },
            [(queue_family, 0.5)].iter().cloned(),
        )
        .expect("failed to create device")
    };

    let queue = queues.next().unwrap();

    (device.clone(), queue.clone(), instance.clone())
}

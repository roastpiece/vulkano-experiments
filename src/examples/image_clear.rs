use image::{ImageBuffer, Rgba};
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::device::{Device, Queue};
use vulkano::format::ClearValue;
use vulkano::format::Format;
use vulkano::image::{Dimensions, StorageImage};
use vulkano::sync::GpuFuture;

pub fn image_clear_and_save(device: Arc<Device>, queue: Arc<Queue>) {
    let image = StorageImage::new(
        device.clone(),
        Dimensions::Dim2d {
            width: 1024,
            height: 1024,
        },
        Format::R8G8B8A8Unorm,
        Some(queue.family()),
    )
    .unwrap();

    let image_dest_buffer = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        (0..1024 * 1024 * 4).map(|_| 0u8),
    )
    .expect("failed to create image_dest_buffer");

    let image_clear_cmd_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family())
        .unwrap()
        .clear_color_image(image.clone(), ClearValue::Float([0.0, 1.0, 1.0, 1.0]))
        .unwrap()
        .copy_image_to_buffer(image.clone(), image_dest_buffer.clone())
        .unwrap()
        .build()
        .unwrap();

    let image_finished = image_clear_cmd_buffer.execute(queue.clone()).unwrap();
    image_finished
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    let image_buffer_data = image_dest_buffer.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &image_buffer_data[..]).unwrap();

    image.save("image.png").unwrap();
}

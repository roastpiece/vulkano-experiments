use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::device::{Device, Queue};
use vulkano::sync::GpuFuture;

pub fn copy_buffers(device: Arc<Device>, queue: Arc<Queue>) {
    let source_data = 0..64;
    let source_buffer =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, source_data)
            .expect("failed to create source_buffer");

    let dest_data = (0..64).map(|_| 0);
    let dest_buffer =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, dest_data)
            .expect("failed to creat dest_buffer");

    let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family())
        .unwrap()
        .copy_buffer(source_buffer.clone(), dest_buffer.clone())
        .unwrap()
        .build()
        .unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();

    finished
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    let source_data = source_buffer.read().unwrap();
    let dest_data = dest_buffer.read().unwrap();
    assert_eq!(&*source_data, &*dest_data);
    println!("BUFFER YAY OKAY!");
}

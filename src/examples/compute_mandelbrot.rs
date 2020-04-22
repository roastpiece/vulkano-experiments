use image::{ImageBuffer, Rgba};
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::{Dimensions, StorageImage};
use vulkano::pipeline::ComputePipeline;
use vulkano::sync::GpuFuture;

pub fn compute_mandel_and_save(device: Arc<Device>, queue: Arc<Queue>) {
    let img_mandel = StorageImage::new(
        device.clone(),
        Dimensions::Dim2d {
            width: 1024,
            height: 1024,
        },
        Format::R8G8B8A8Unorm,
        Some(queue.family()),
    )
    .unwrap();

    let mandel_buff = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        (0..1024 * 1024 * 4).map(|_| 0u8),
    )
    .expect("failed to create mandel_buff");

    let shader_mandel =
        cs_mandel::Shader::load(device.clone()).expect("failed to create shader module");

    let mandel_pipeline = Arc::new(
        ComputePipeline::new(device.clone(), &shader_mandel.main_entry_point(), &())
            .expect("failed to create multiplypline"),
    );

    let mandel_layout = mandel_pipeline.layout().descriptor_set_layout(0).unwrap();
    let mandel_set = Arc::new(
        PersistentDescriptorSet::start(mandel_layout.clone())
            .add_image(img_mandel.clone())
            .unwrap()
            .build()
            .unwrap(),
    );

    let mandel_cmd_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family())
        .unwrap()
        .dispatch(
            [1024 / 8, 1024 / 8, 1],
            mandel_pipeline.clone(),
            mandel_set.clone(),
            (),
        )
        .unwrap()
        .copy_image_to_buffer(img_mandel.clone(), mandel_buff.clone())
        .unwrap()
        .build()
        .unwrap();

    let mandel_finished = mandel_cmd_buffer.execute(queue.clone()).unwrap();
    mandel_finished
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    let mandel_data = mandel_buff.read().unwrap();
    let img_mandel = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &mandel_data[..]).unwrap();

    img_mandel.save("mandel.png").unwrap();
}

mod cs_mandel {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/mandelbrot.comp.glsl"
    }
}

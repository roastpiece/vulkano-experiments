use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::device::{Device, Queue};
use vulkano::pipeline::ComputePipeline;
use vulkano::sync::GpuFuture;

pub fn compute_shader_multiply(device: Arc<Device>, queue: Arc<Queue>) {
    let multi_data = 0..65536;
    let multi_buffer =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, multi_data)
            .expect("failed to create par_buffer");

    let shader = cs_multiply::Shader::load(device.clone()).expect("failed to create shader module");

    let multiplypline = Arc::new(
        ComputePipeline::new(device.clone(), &shader.main_entry_point(), &())
            .expect("failed to create multiplypline"),
    );

    let multi_layout = multiplypline.layout().descriptor_set_layout(0).unwrap();
    let multi_set = Arc::new(
        PersistentDescriptorSet::start(multi_layout.clone())
            .add_buffer(multi_buffer.clone())
            .unwrap()
            .build()
            .unwrap(),
    );

    let multi_cmd_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family())
        .unwrap()
        .dispatch([1024, 1, 1], multiplypline.clone(), multi_set.clone(), ())
        .unwrap()
        .build()
        .unwrap();

    let multi_finished = multi_cmd_buffer.execute(queue.clone()).unwrap();
    multi_finished
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    let multi_data = multi_buffer.read().unwrap();
    for (n, val) in multi_data.iter().enumerate() {
        assert_eq!(*val, n as u32 * 12);
    }

    println!("MULTI YAY OKAY!");
}

mod cs_multiply {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/multiply.comp.glsl",
    }
}

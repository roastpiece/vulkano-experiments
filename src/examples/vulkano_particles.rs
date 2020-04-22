use rand::Rng;
use std::sync::Arc;
use std::time::Instant;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer, DynamicState};
use vulkano::descriptor::descriptor_set::{
    FixedSizeDescriptorSetBuilder, FixedSizeDescriptorSetsPool, PersistentDescriptorSet,
};
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract, Subpass};
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::{ComputePipeline, GraphicsPipeline};
use vulkano::swapchain::{
    self, AcquireError, ColorSpace, FullscreenExclusive, PresentMode, SurfaceTransform, Swapchain,
    SwapchainCreationError,
};
use vulkano::sync;
use vulkano::sync::{FlushError, GpuFuture};
use vulkano_win::VkSurfaceBuild;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

pub fn graphics_window(device: Arc<Device>, queue: Arc<Queue>, instance: Arc<Instance>) {
    let events_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&events_loop, instance.clone())
        .unwrap();

    let caps = surface
        .capabilities(device.physical_device())
        .expect("failed to get surface capabilities");

    let dimensions = caps.current_extent.unwrap_or([1280, 1024]);
    let alpha = caps.supported_composite_alpha.iter().next().unwrap();
    let format = caps.supported_formats[0].0;

    let (mut swapchain, images) = Swapchain::new(
        device.clone(),
        surface.clone(),
        caps.min_image_count,
        format,
        dimensions,
        1,
        caps.supported_usage_flags,
        &queue,
        SurfaceTransform::Identity,
        alpha,
        PresentMode::Fifo,
        FullscreenExclusive::Default,
        true,
        ColorSpace::SrgbNonLinear,
    )
    .expect("failed to create swapchain");

    let vertex1 = Vertex::new(-0.5, -0.5);
    let vertex2 = Vertex::new(0.0, 0.5);
    let vertex3 = Vertex::new(0.5, -0.25);

    let vertex_buffer = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        vec![vertex1, vertex2, vertex3].into_iter(),
    )
    .unwrap();

    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap(),
    );

    let mut dynamic_state = DynamicState::none();

    let mut framebuffers =
        window_size_dependent_setup(&images, render_pass.clone(), &mut dynamic_state);

    let vert_shader =
        vs_graphics::Shader::load(device.clone()).expect("failed to create vert_shader");
    let frag_shader =
        fs_graphics::Shader::load(device.clone()).expect("failed to create frag_shader");

    let pipeline = Arc::new(
        GraphicsPipeline::start()
            .polygon_mode_point()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vert_shader.main_entry_point(), ())
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(frag_shader.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );

    let particle_shader =
        cs_particle_physics::Shader::load(device.clone()).expect("failed to load particle_shader");

    let particle_compute_pipeline = Arc::new(
        ComputePipeline::new(device.clone(), &particle_shader.main_entry_point(), &())
            .expect("failed to create particle_compute_pipeline"),
    );

    let vertex_layout = particle_compute_pipeline
        .layout()
        .descriptor_set_layout(0)
        .unwrap();

    let uniform_layout = particle_compute_pipeline
        .layout()
        .descriptor_set_layout(1)
        .unwrap();

    let particle_set = Arc::new(
        PersistentDescriptorSet::start(vertex_layout.clone())
            .add_buffer(vertex_buffer.clone())
            .unwrap()
            .build()
            .unwrap(),
    );

    let mut uniform_pool = FixedSizeDescriptorSetsPool::new(uniform_layout.clone());

    let mut previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);
    let mut recreate_swapchain = false;

    let mut mouse_position: [f32; 2] = [0.0, 0.0];

    let mut last_time = std::time::Instant::now();
    let mut delta_time: f32 = 0.0;

    events_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            winit::event::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                // winit::event::WindowEvent::KeyboardInput { input, .. } => match input {
                //     winit::event::KeyboardInput {
                //         state: winit::event::ElementState::Pressed,
                //         virtual_keycode: Some(winit::event::VirtualKeyCode::M),
                //         ..
                //     } => {}
                //     _ => (),
                // },
                winit::event::WindowEvent::CursorMoved { position, .. } => {
                    let dimensions = surface.window().inner_size();
                    mouse_position = [
                        (((position.x / dimensions.width as f64) - 0.5) * 2.0) as f32,
                        (((position.y / dimensions.height as f64) - 0.5) * 2.0) as f32,
                    ];
                }
                _ => (),
            },
            winit::event::Event::MainEventsCleared => {
                let uniform_data = ParticleUBO {
                    target: mouse_position.clone(),
                    delta_time,
                };

                let particle_uniform_buffer = CpuAccessibleBuffer::from_data(
                    device.clone(),
                    BufferUsage::all(),
                    false,
                    uniform_data,
                )
                .expect("failed to create particle_uniform_buffer");
                let uniform_set = uniform_pool
                    .next()
                    .add_buffer(particle_uniform_buffer.clone())
                    .unwrap()
                    .build()
                    .unwrap();

                let particle_cmd_buffer =
                    AutoCommandBufferBuilder::new(device.clone(), queue.family())
                        .unwrap()
                        .dispatch(
                            [3, 1, 1],
                            particle_compute_pipeline.clone(),
                            (particle_set.clone(), uniform_set),
                            (),
                        )
                        .unwrap()
                        .build()
                        .unwrap();

                let particle_cmd_finished = particle_cmd_buffer.execute(queue.clone()).unwrap();
                particle_cmd_finished
                    .then_signal_fence_and_flush()
                    .unwrap()
                    .wait(None)
                    .unwrap();

                surface
                    .window()
                    .set_title(format!("FPS: {:.2}", 1.0 / delta_time).as_str());

                surface.window().request_redraw();
            }
            winit::event::Event::RedrawRequested(_) => {
                let time = std::time::Instant::now();
                let delta_time_instant = time - last_time;

                delta_time = delta_time_instant.as_secs_f32();
                last_time = time;

                previous_frame_end.as_mut().unwrap().cleanup_finished();

                if recreate_swapchain {
                    let dimensions: [u32; 2] = surface.window().inner_size().into();
                    let (new_swapchain, new_images) =
                        match swapchain.recreate_with_dimensions(dimensions) {
                            Ok(r) => r,
                            Err(SwapchainCreationError::UnsupportedDimensions) => return,
                            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                        };

                    swapchain = new_swapchain;
                    framebuffers = window_size_dependent_setup(
                        &new_images,
                        render_pass.clone(),
                        &mut dynamic_state,
                    );
                    recreate_swapchain = false;
                }

                let (image_num, suboptimal, acquire_future) =
                    match swapchain::acquire_next_image(swapchain.clone(), None) {
                        Ok(r) => r,
                        Err(AcquireError::OutOfDate) => {
                            recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("Failed to acquire next image: {:?}", e),
                    };

                if suboptimal {
                    recreate_swapchain = true;
                }

                let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(
                    device.clone(),
                    queue.family(),
                )
                .unwrap()
                .begin_render_pass(
                    framebuffers[image_num].clone(),
                    false,
                    vec![[0.0, 0.0, 0.0, 1.0].into()],
                )
                .unwrap()
                .draw(
                    pipeline.clone(),
                    &dynamic_state,
                    vertex_buffer.clone(),
                    (),
                    (),
                )
                .unwrap()
                .end_render_pass()
                .unwrap()
                .build()
                .unwrap();

                let future = previous_frame_end
                    .take()
                    .unwrap()
                    .join(acquire_future)
                    .then_execute(queue.clone(), command_buffer)
                    .unwrap()
                    .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                    .then_signal_fence_and_flush();

                match future {
                    Ok(future) => {
                        previous_frame_end = Some(Box::new(future) as Box<_>);
                    }
                    Err(FlushError::OutOfDate) => {
                        recreate_swapchain = true;
                        previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
                    }
                    Err(e) => {
                        println!("Failed to flush future {:?}", e);
                        previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
                    }
                }
            }
            _ => (),
        }
    });
}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };

    dynamic_state.viewports = Some(vec![viewport]);

    images
        .iter()
        .map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>()
}

#[derive(Default, Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
    velocity: [f32; 2],
}

impl Vertex {
    fn new(x: f32, y: f32) -> Vertex {
        Vertex {
            position: [x, y],
            velocity: [0.0, 0.0],
        }
    }
}
vulkano::impl_vertex!(Vertex, position, velocity);

struct ParticleUBO {
    target: [f32; 2],
    delta_time: f32,
}

mod vs_graphics {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/particles.vert.glsl"
    }
}

mod fs_graphics {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/graphics.frag.glsl"
    }
}

mod cs_particle_physics {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/particle_physics.comp.glsl"
    }
}

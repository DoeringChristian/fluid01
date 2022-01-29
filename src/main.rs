use wgpu::RenderPipeline;
use wgpu_utils::{framework::{State, Framework}, mesh::{Mesh, Drawable}, vert::Vert2, pipeline::{self, RenderPipelineBuilder, shader_with_shaderc, VertexStateBuilder, FragmentStateBuilder, PipelineLayoutBuilder, RenderPass, RenderPassBuilder}, render_target::ColorAttachment};

#[macro_use]
extern crate more_asserts;

extern crate nalgebra_glm as glm;
extern crate naga;

mod wgpu_utils;

struct WinState{
    mesh: Mesh<Vert2>,
    render_pipeline: pipeline::RenderPipeline,
}

impl State for WinState{
    fn new(app: &mut wgpu_utils::framework::AppState) -> Self {
        let mesh = Mesh::new(&app.device, &Vert2::QUAD_VERTS, &Vert2::QUAD_IDXS).unwrap();

        let vert_shader = shader_with_shaderc(&app.device, include_str!("shaders/vert_test01.glsl"), shaderc::ShaderKind::Vertex, "main", None).unwrap();
        let frag_shader = shader_with_shaderc(&app.device, include_str!("shaders/frag_test01.glsl"), shaderc::ShaderKind::Fragment, "main", None).unwrap();

        let vert_state = VertexStateBuilder::new(&vert_shader)
            .push_named("model", mesh.vert_buffer_layout())
            .build();

        let frag_state = FragmentStateBuilder::new(&frag_shader)
            .push_target_replace(app.config.format)
            .build();

        let render_pipeline_layout = PipelineLayoutBuilder::new()
            .create(&app.device, None);

        let render_pipeline = RenderPipelineBuilder::new(vert_state, frag_state)
            .set_layout(&render_pipeline_layout)
            .build(&app.device);

        Self{
            mesh,
            render_pipeline,
        }
    }

    fn render(&mut self, app: &mut wgpu_utils::framework::AppState, control_flow: &mut winit::event_loop::ControlFlow) -> Result<(), wgpu::SurfaceError> {
        let output = app.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = app.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = RenderPassBuilder::new()
                .push_color_attachment(view.color_attachment_clear())
                .begin(&mut encoder, None);

            let mut render_pass_pipeline = render_pass.set_pipeline(&self.render_pipeline);

            self.mesh.draw(&mut render_pass_pipeline);

        }


        app.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn main() {
    let framework = Framework::<WinState>::new([800, 600]).run();
}

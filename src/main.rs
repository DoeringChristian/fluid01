use wgpu::RenderPipeline;
use wgpu_utils::{framework::{State, Framework}, mesh::{Mesh, Drawable}, vert::Vert2, pipeline::{self, RenderPipelineBuilder, shader_with_shaderc, VertexStateBuilder, FragmentStateBuilder, PipelineLayoutBuilder, RenderPass, RenderPassBuilder}, render_target::ColorAttachment, buffer::UniformBindGroup, binding::{GetBindGroupLayout, GetBindGroup}, texture::Texture};

#[macro_use]
extern crate more_asserts;

extern crate nalgebra_glm as glm;
extern crate naga;

mod wgpu_utils;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GlobalShaderData{
    size: [f32; 2],
    time: f32,
    _pad0: f32,
}

struct WinState{
    mesh: Mesh<Vert2>,
    fluid_render_pipeline: pipeline::RenderPipeline,
    display_rp: pipeline::RenderPipeline,
    global_uniform: UniformBindGroup<GlobalShaderData>,

    tex0: Texture,
    tex1: Texture,

    fc: usize,
}

impl State for WinState{
    fn new(app: &mut wgpu_utils::framework::AppState) -> Self {
        let mesh = Mesh::new(&app.device, &Vert2::QUAD_VERTS, &Vert2::QUAD_IDXS).unwrap();

        let global_uniform = UniformBindGroup::<GlobalShaderData>::new_with_data(&app.device, &GlobalShaderData{
            size: [app.size.width as f32, app.size.height as f32],
            time: 0.0,
            _pad0: 0.0,
        });

        //let tex0 = Texture::load_from_path(&app.device, &app.queue, "assets/test01.png", None, app.config.format).unwrap();
        let tex0 = Texture::new_black([app.size.width as u32, app.size.height as u32], &app.device, &app.queue, None, wgpu::TextureFormat::Rgba32Float).unwrap();
        let tex1 = Texture::new_black(tex0.size, &app.device, &app.queue, None, wgpu::TextureFormat::Rgba32Float).unwrap();


        // Initialize Pipeline
        //
        let fluid_vert_shader = shader_with_shaderc(&app.device, include_str!("shaders/vert_test01.glsl"), shaderc::ShaderKind::Vertex, "main", None).unwrap();
        let fluid_frag_shader = shader_with_shaderc(&app.device, include_str!("shaders/frag_fluid01.glsl"), shaderc::ShaderKind::Fragment, "main", None).unwrap();

        let fluid_vert_state = VertexStateBuilder::new(&fluid_vert_shader)
            .push_named("model", mesh.vert_buffer_layout())
            .build();

        let fluid_frag_state = FragmentStateBuilder::new(&fluid_frag_shader)
            .push_target_replace(wgpu::TextureFormat::Rgba32Float)
            .build();

        let fluid_render_pipeline_layout = PipelineLayoutBuilder::new()
            .push_named("global", global_uniform.get_bind_group_layout())
            .push_named("tex", tex0.get_bind_group_layout())
            .create(&app.device, None);

        let fluid_render_pipeline = RenderPipelineBuilder::new(fluid_vert_state, fluid_frag_state)
            .set_layout(&fluid_render_pipeline_layout)
            .build(&app.device);


        let display_fsh = shader_with_shaderc(&app.device, include_str!("shaders/frag_test01.glsl"), shaderc::ShaderKind::Fragment, "main", None).unwrap();
        
        let display_vst = VertexStateBuilder::new(&fluid_vert_shader)
            .push_named("model", mesh.vert_buffer_layout())
            .build();

        let display_fst = FragmentStateBuilder::new(&display_fsh)
            .push_target_replace(app.config.format)
            .build();

        let display_rpl = PipelineLayoutBuilder::new()
            .push_named("global", global_uniform.get_bind_group_layout())
            .push_named("tex", tex0.get_bind_group_layout())
            .create(&app.device, None);

        let display_rp = RenderPipelineBuilder::new(display_vst, display_fst)
            .set_layout(&display_rpl)
            .build(&app.device);
        

        Self{
            mesh,
            fluid_render_pipeline,
            display_rp,
            global_uniform,
            tex0,
            tex1,
            fc: 0,
        }
    }

    fn render(&mut self, app: &mut wgpu_utils::framework::AppState, control_flow: &mut winit::event_loop::ControlFlow) -> Result<(), wgpu::SurfaceError> {
        let output = app.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = app.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
            label: Some("Render Encoder"),
        });

        // fluid sim
            {
                let mut render_pass = RenderPassBuilder::new()
                    .push_color_attachment(self.tex1.view.color_attachment_clear())
                    .begin(&mut encoder, None);

                let mut render_pass_pipeline = render_pass.set_pipeline(&self.fluid_render_pipeline);
                render_pass_pipeline.set_bind_group("global", self.global_uniform.get_bind_group(), &[]);
                render_pass_pipeline.set_bind_group("tex", self.tex0.get_bind_group(), &[]);

                self.mesh.draw(&mut render_pass_pipeline);
            }
            // copy texture back
        if self.fc < 10000{
            {
                self.tex1.copy_all_to(&mut self.tex0, &mut encoder);
            }
        }

        // render result to view.
        {
            let mut render_pass = RenderPassBuilder::new()
                .push_color_attachment(view.color_attachment_clear())
                .begin(&mut encoder, None);

            let mut render_pass_pipeline = render_pass.set_pipeline(&self.display_rp);
            render_pass_pipeline.set_bind_group("global", self.global_uniform.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group("tex", self.tex1.get_bind_group(), &[]);

            self.mesh.draw(&mut render_pass_pipeline);
        }

        self.fc += 1;
        self.global_uniform.get_content().time = self.fc as f32 / 60.0;
        self.global_uniform.update_int(&app.queue);

        app.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn resize(&mut self, app: &mut wgpu_utils::framework::AppState, new_size: winit::dpi::PhysicalSize<u32>) {
        self.global_uniform.get_content().size = [new_size.width as f32, new_size.height as f32];
        self.global_uniform.update_int(&app.queue);
    }
}

fn main() {
    let framework = Framework::<WinState>::new([800, 600]).run();
}

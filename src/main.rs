use bytemuck::Zeroable;
use wgpu::RenderPipeline;
use wgpu_utils::{framework::{State, Framework}, mesh::{Mesh, Drawable}, vert::Vert2, pipeline::{self, RenderPipelineBuilder, shader_with_shaderc, VertexStateBuilder, FragmentStateBuilder, PipelineLayoutBuilder, RenderPass, RenderPassBuilder}, render_target::ColorAttachment, uniform::{UniformBindGroup, Uniform}, binding::{GetBindGroupLayout, GetBindGroup, CreateBindGroupLayout, BindGroup}, texture::Texture};

#[macro_use]
extern crate more_asserts;

extern crate nalgebra_glm as glm;
extern crate naga;

mod wgpu_utils;
mod paintsim;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GlobalShaderData{
    size: [f32; 2],
    time: f32,
    _pad0: f32,
}

#[derive(Default)]
struct DisplayData{

}


struct WinState{
    mesh: Mesh<Vert2>,
    display_rp: pipeline::RenderPipeline,
    //global_uniform: UniformBindGroup<GlobalShaderData>,
    global_uniform: UniformBindGroup<GlobalShaderData>,

    paintsim: paintsim::PaintSim,

    fc: usize,
}

impl State for WinState{
    fn new(app: &mut wgpu_utils::framework::AppState) -> Self {
        let mesh = Mesh::new(&app.device, &Vert2::QUAD_VERTS, &Vert2::QUAD_IDXS).unwrap();

        let global_uniform = UniformBindGroup::<GlobalShaderData>::new(&app.device, GlobalShaderData{
            size: [app.size.width as f32, app.size.height as f32],
            time: 0.0,
            _pad0: 0.0,
        });

        let paintsim = paintsim::PaintSim::new(&app.device, &app.queue, "assets/test03.jpg").unwrap();

        // Init display pipeline.
        let display_vsh = shader_with_shaderc(&app.device, include_str!("shaders/vf_display.glsl"), shaderc::ShaderKind::Vertex, "main", None).unwrap();
        let display_fsh = shader_with_shaderc(&app.device, include_str!("shaders/vf_display.glsl"), shaderc::ShaderKind::Fragment, "main", None).unwrap();

        let display_vst = VertexStateBuilder::new(&display_vsh)
            .push_vert_layout(mesh.vert_buffer_layout())
            .build();

        let display_fst = FragmentStateBuilder::new(&display_fsh)
            .push_target_replace(app.config.format)
            .build();

        // TODO: put all textures together into one bindgroup.
        let display_rpl = PipelineLayoutBuilder::new()
            .push(global_uniform.get_bind_group_layout())
            .push(&BindGroup::<Texture>::create_bind_group_layout(&app.device, None))
            .push(&BindGroup::<Texture>::create_bind_group_layout(&app.device, None))
            .push(&BindGroup::<Texture>::create_bind_group_layout(&app.device, None))
            .create(&app.device, None);

        let display_rp = RenderPipelineBuilder::new(display_vst, display_fst)
            .set_layout(&display_rpl)
            .build(&app.device);

        Self{
            mesh,
            display_rp,
            global_uniform,
            paintsim,
            fc: 0,
        }
    }

    fn render(&mut self, app: &mut wgpu_utils::framework::AppState, control_flow: &mut winit::event_loop::ControlFlow) -> Result<(), wgpu::SurfaceError> {
        let output = app.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = app.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
            label: Some("Render Encoder"),
        });

        for i in 0..6{
            self.paintsim.step(&mut app.queue, &mut encoder);
        }

        // render result to view.
        {
            let mut render_pass = RenderPassBuilder::new()
                .push_color_attachment(view.color_attachment_clear())
                .begin(&mut encoder, None);

            let mut render_pass_pipeline = render_pass.set_pipeline(&self.display_rp);
            render_pass_pipeline.set_bind_group(0, self.global_uniform.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group(1, self.paintsim.tex_vpf.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group(2, self.paintsim.tex_color.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group(3, self.paintsim.tex_float.get_bind_group(), &[]);

            self.mesh.draw(&mut render_pass_pipeline);
        }

        self.fc += 1;
        println!("time: {}", self.fc as f32/60.0);
        self.global_uniform.content.borrow_ref(&mut app.queue).time = self.fc as f32 / 60.;
        //self.global_uniform.content.get_content().time = self.fc as f32 / 60.0;
        //self.global_uniform.content.update_int(&app.queue);

        app.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn pre_render(&mut self, app: &mut wgpu_utils::framework::AppState, control_flow: &mut winit::event_loop::ControlFlow) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = app.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
            label: Some("PreRenderEncoder"),
        });

        self.paintsim.prepare(&app.queue, &mut encoder);

        app.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    fn resize(&mut self, app: &mut wgpu_utils::framework::AppState, new_size: winit::dpi::PhysicalSize<u32>) {
        self.global_uniform.borrow_ref(&mut app.queue).size = [new_size.width as f32, new_size.height as f32];
    }
}

fn main() {
    let framework = Framework::<WinState>::new([800, 600]).run();
}

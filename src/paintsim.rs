use crate::wgpu_utils::binding::{GetBindGroupLayout, GetBindGroup};
use crate::wgpu_utils::buffer::UniformBindGroup;
use crate::wgpu_utils::mesh::Drawable;
use crate::wgpu_utils::pipeline::{shader_with_shaderc, VertexStateBuilder, FragmentStateBuilder, PipelineLayoutBuilder, RenderPipelineBuilder, RenderPassBuilder};
use crate::wgpu_utils::render_target::ColorAttachment;
use crate::wgpu_utils::{texture::Texture, mesh::Mesh, vert::Vert2, pipeline, buffer};
use crate::GlobalShaderData;
use anyhow::*;


pub struct PaintSim{
    // texture storing the velocity, preasure and fluidity.
    pub tex_vpf: Texture,
    tex_vpf_tmp: Texture,

    // texture storing the color for smearing.
    pub tex_color: Texture,
    tex_color_tmp: Texture,

    // texture storing the initial image.
    tex_src: Texture,

    pipeline: pipeline::RenderPipeline,

    global_uniform: buffer::UniformBindGroup<GlobalShaderData>,
    
    mesh: Mesh<Vert2>,

    fc: usize,
}

impl PaintSim{
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, path: &str) -> Result<Self>{
        let mesh = Mesh::new(device, &Vert2::QUAD_VERTS, &Vert2::QUAD_IDXS)?;

        let tex_src = Texture::load_from_path(device, queue, path, None, wgpu::TextureFormat::Rgba8Unorm)?;

        let tex_vpf = Texture::new_black(tex_src.size, device, queue, None, wgpu::TextureFormat::Rgba32Float)?;
        let tex_vpf_tmp = Texture::new_black(tex_src.size, device, queue, None, wgpu::TextureFormat::Rgba32Float)?;

        let tex_color = Texture::new_black(tex_src.size, device, queue, None, wgpu::TextureFormat::Rgba32Float)?; 
        let tex_color_tmp = Texture::new_black(tex_src.size, device, queue, None, wgpu::TextureFormat::Rgba32Float)?; 

        let global_uniform = UniformBindGroup::<GlobalShaderData>::new_with_data(device, &GlobalShaderData{
            size: [tex_src.size[0] as f32, tex_src.size[1] as f32],
            time: 0.0,
            _pad0: 0.0,
        });

        let vert_shader = shader_with_shaderc(device, include_str!("shaders/vf_paint03.glsl"), shaderc::ShaderKind::Vertex, "main", None)?;
        let frag_shader = shader_with_shaderc(device, include_str!("shaders/vf_paint03.glsl"), shaderc::ShaderKind::Fragment, "main", None)?;

        let vert_state = VertexStateBuilder::new(&vert_shader)
            .push_named("model", mesh.vert_buffer_layout())
            .build();

        let frag_state = FragmentStateBuilder::new(&frag_shader)
            .push_target_replace(wgpu::TextureFormat::Rgba32Float)
            .push_target_replace(wgpu::TextureFormat::Rgba32Float)
            .build();
        
        let pipeline_layout = PipelineLayoutBuilder::new()
            .push_named("global", global_uniform.get_bind_group_layout())
            .push_named("tex_vpf", tex_vpf.get_bind_group_layout())
            .push_named("tex_color", tex_color.get_bind_group_layout())
            .create(device, None);

        let pipeline = RenderPipelineBuilder::new(vert_state, frag_state)
            .set_layout(&pipeline_layout)
            .build(device);
        

        Ok(Self{
            mesh,
            tex_src,
            tex_vpf,
            tex_vpf_tmp,
            tex_color,
            tex_color_tmp,
            global_uniform,
            pipeline,
            fc: 0,
        })
    }

    pub fn update(&mut self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder){
        self.fc += 1;
        self.global_uniform.get_content().time = self.fc as f32 / 60.;
        self.global_uniform.update_int(queue);
        // Simulation step:
        {
            let mut render_pass = RenderPassBuilder::new()
                .push_color_attachment(self.tex_vpf_tmp.view.color_attachment_clear())
                .push_color_attachment(self.tex_color_tmp.view.color_attachment_clear())
                .begin(encoder, None);

            let mut render_pass_pipeline = render_pass.set_pipeline(&self.pipeline);

            render_pass_pipeline.set_bind_group("global", self.global_uniform.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group("tex_vpf", self.tex_vpf.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group("tex_color", self.tex_color.get_bind_group(), &[]);

            self.mesh.draw(&mut render_pass_pipeline);
        }

        // Copy back step:
        {
            self.tex_vpf_tmp.copy_all_to(&mut self.tex_vpf, encoder);
            self.tex_color_tmp.copy_all_to(&mut self.tex_color, encoder);
        }
    }
}

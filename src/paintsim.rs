use crate::wgpu_utils::binding::{GetBindGroupLayout, GetBindGroup, BindGroup};
use crate::wgpu_utils::buffer::UniformBindGroup;
use crate::wgpu_utils::mesh::Drawable;
use crate::wgpu_utils::pipeline::{shader_with_shaderc, VertexStateBuilder, FragmentStateBuilder, PipelineLayoutBuilder, RenderPipelineBuilder, RenderPassBuilder};
use crate::wgpu_utils::render_target::ColorAttachment;
use crate::wgpu_utils::{texture::Texture, mesh::Mesh, vert::Vert2, pipeline, buffer};
use crate::GlobalShaderData;
use anyhow::*;


pub struct PaintSim{
    // texture storing the velocity, preasure and fluidity.
    pub tex_vpf: BindGroup<Texture>,
    tex_vpf_tmp: Texture,

    // texture storing the color of the base layer.
    pub tex_color: BindGroup<Texture>,
    tex_color_tmp: Texture,

    // texture stiring the floating particulate.
    pub tex_float: BindGroup<Texture>,
    tex_float_tmp: Texture,

    // texture storing the initial image.
    pub tex_src: BindGroup<Texture>,

    pipeline: pipeline::RenderPipeline,

    pipeline_src_to_color: pipeline::RenderPipeline,

    global_uniform: buffer::UniformBindGroup<GlobalShaderData>,
    
    mesh: Mesh<Vert2>,

    sc: usize,
}

impl PaintSim{
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, path: &str) -> Result<Self>{
        let mesh = Mesh::new(device, &Vert2::QUAD_VERTS, &Vert2::QUAD_IDXS)?;

        let tex_src = BindGroup::new(Texture::load_from_path(device, queue, path, None, wgpu::TextureFormat::Rgba8Unorm)?, device);

        let tex_vpf = BindGroup::new(Texture::new_black(tex_src.size, device, queue, None, wgpu::TextureFormat::Rgba32Float)?, device);
        let tex_vpf_tmp = Texture::new_black(tex_src.size, device, queue, None, wgpu::TextureFormat::Rgba32Float)?;

        let tex_color = BindGroup::new(Texture::new_black(tex_src.size, device, queue, None, wgpu::TextureFormat::Rgba32Float)?, device); 
        let tex_color_tmp = Texture::new_black(tex_src.size, device, queue, None, wgpu::TextureFormat::Rgba32Float)?; 

        let tex_float = BindGroup::new(Texture::new_black(tex_src.size, device, queue, None, wgpu::TextureFormat::Rgba32Float)?, device); 
        let tex_float_tmp = Texture::new_black(tex_src.size, device, queue, None, wgpu::TextureFormat::Rgba32Float)?; 

        let global_uniform = UniformBindGroup::<GlobalShaderData>::new_with_data(device, GlobalShaderData{
            size: [tex_src.size[0] as f32, tex_src.size[1] as f32],
            time: 0.0,
            _pad0: 0.0,
        });

        let vert_shader = shader_with_shaderc(device, include_str!("shaders/vf_paint04.glsl"), shaderc::ShaderKind::Vertex, "main", None)?;
        let frag_shader = shader_with_shaderc(device, include_str!("shaders/vf_paint04.glsl"), shaderc::ShaderKind::Fragment, "main", None)?;

        let vert_state = VertexStateBuilder::new(&vert_shader)
            .push_named("model", mesh.vert_buffer_layout())
            .build();

        let frag_state = FragmentStateBuilder::new(&frag_shader)
            .push_target_replace(wgpu::TextureFormat::Rgba32Float)
            .push_target_replace(wgpu::TextureFormat::Rgba32Float)
            .push_target_replace(wgpu::TextureFormat::Rgba32Float)
            .build();
        
        let pipeline_layout = PipelineLayoutBuilder::new()
            .push_named("global", global_uniform.get_bind_group_layout())
            .push_named("tex_vpf", tex_vpf.get_bind_group_layout())
            .push_named("tex_color", tex_color.get_bind_group_layout())
            .push_named("tex_float", tex_float.get_bind_group_layout())
            .create(device, None);

        let pipeline = RenderPipelineBuilder::new(vert_state, frag_state)
            .set_layout(&pipeline_layout)
            .build(device);

        let vert_shader = shader_with_shaderc(device, include_str!("shaders/vf_src_to_color.glsl"), shaderc::ShaderKind::Vertex, "main", None)?;
        let frag_shader = shader_with_shaderc(device, include_str!("shaders/vf_src_to_color.glsl"), shaderc::ShaderKind::Fragment, "main", None)?;

        let vert_state = VertexStateBuilder::new(&vert_shader)
            .push_named("model", mesh.vert_buffer_layout())
            .build();

        let frag_state = FragmentStateBuilder::new(&frag_shader)
            .push_target_replace(wgpu::TextureFormat::Rgba32Float)
            .build();
        
        let pipeline_layout = PipelineLayoutBuilder::new()
            .push_named("tex_src", tex_src.get_bind_group_layout())
            .create(device, None);

        let pipeline_src_to_color = RenderPipelineBuilder::new(vert_state, frag_state)
            .set_layout(&pipeline_layout)
            .build(device);

        

        Ok(Self{
            mesh,
            tex_src,
            tex_vpf,
            tex_vpf_tmp,
            tex_color,
            tex_color_tmp,
            tex_float,
            tex_float_tmp,
            global_uniform,
            pipeline,
            pipeline_src_to_color,
            sc: 0,
        })
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder){
        {
            let mut render_pass = RenderPassBuilder::new()
                .push_color_attachment(self.tex_color.view.color_attachment_clear())
                .begin(encoder, None);

            let mut render_pass_pipeline = render_pass.set_pipeline(&self.pipeline_src_to_color);

            render_pass_pipeline.set_bind_group("tex_src", self.tex_src.get_bind_group(), &[]);
            
            self.mesh.draw(&mut render_pass_pipeline);
        }
    }

    pub fn step(&mut self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder){
        self.global_uniform.borrow_ref(queue).time = self.sc as f32 /60.;
        // Simulation step:
        {
            let mut render_pass = RenderPassBuilder::new()
                .push_color_attachment(self.tex_vpf_tmp.view.color_attachment_clear())
                .push_color_attachment(self.tex_color_tmp.view.color_attachment_clear())
                .push_color_attachment(self.tex_float_tmp.view.color_attachment_clear())
                .begin(encoder, None);

            let mut render_pass_pipeline = render_pass.set_pipeline(&self.pipeline);

            render_pass_pipeline.set_bind_group("global", self.global_uniform.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group("tex_vpf", self.tex_vpf.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group("tex_color", self.tex_color.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group("tex_float", self.tex_float.get_bind_group(), &[]);

            self.mesh.draw(&mut render_pass_pipeline);
        }

        // Copy back step:
        {
            self.tex_vpf_tmp.copy_all_to(&mut self.tex_vpf, encoder);
            self.tex_color_tmp.copy_all_to(&mut self.tex_color, encoder);
            self.tex_float_tmp.copy_all_to(&mut self.tex_float, encoder);
        }

        self.sc += 1;
    }
}

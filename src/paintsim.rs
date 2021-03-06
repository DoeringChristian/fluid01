use crate::wgpu_utils::binding::{GetBindGroupLayout, GetBindGroup, BindGroup};
use crate::wgpu_utils::buffer::{Buffer, self};
use crate::wgpu_utils::uniform::{self, UniformBindGroup, Uniform, UniformVec};
use crate::wgpu_utils::mesh::Drawable;
use crate::wgpu_utils::pipeline::{shader_with_shaderc, VertexStateBuilder, FragmentStateBuilder, PipelineLayoutBuilder, RenderPipelineBuilder, RenderPassBuilder, PipelineLayout, ComputePipeline};
use crate::wgpu_utils::render_target::ColorAttachment;
use crate::wgpu_utils::{texture::Texture, mesh::Mesh, vert::Vert2, pipeline};
use crate::GlobalShaderData;
use crate::wgpu_utils::binding::CreateBindGroupLayout;
use anyhow::*;

#[allow(non_camel_case_types)]
pub enum PaintPipelineLayout{
    global_uniform = 0,
    tex_vpf,
    tex_color,
    tex_float,
}

impl PaintPipelineLayout{
    pub fn create_pipeline_layout(device: &wgpu::Device) -> PipelineLayout{
        PipelineLayoutBuilder::new()
            .push(&UniformBindGroup::<GlobalShaderData>::create_bind_group_layout(device, None))
            .push(&BindGroup::<Texture>::create_bind_group_layout(device, None))
            .push(&BindGroup::<Texture>::create_bind_group_layout(device, None))
            .push(&BindGroup::<Texture>::create_bind_group_layout(device, None))
            .create(device, None)
    }
}

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
    pipeline_blurwh: pipeline::RenderPipeline,
    pipeline_blurwv: pipeline::RenderPipeline,
    pipeline_src_to_color: pipeline::RenderPipeline,

    ppl_comp: ComputePipeline,

    global_uniform: uniform::UniformBindGroup<GlobalShaderData>,
    in_buffer: BindGroup<Buffer<i32>>,
    out_buffer: BindGroup<Buffer<i32>>,
    
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

        let global_uniform = UniformBindGroup::<GlobalShaderData>::new(device, GlobalShaderData{
            size: [tex_src.size[0] as f32, tex_src.size[1] as f32],
            time: 0.0,
            _pad0: 0.0,
        });

        let comp_shader = shader_with_shaderc(device, include_str!("shaders/comp_test01.glsl"), shaderc::ShaderKind::Compute, "main", None)?;

        let in_buffer = BindGroup::new(Buffer::new_storage(device, None, &[0, 1, 2, 3]), device);
        let out_buffer = BindGroup::new(Buffer::new_storage(device, None, &[0, 1, 2, 3]), device);

        let comp_layout = PipelineLayoutBuilder::new()
            .push(&BindGroup::<Buffer<i32>>::create_bind_group_layout(device, None))
            .push(&BindGroup::<Buffer<i32>>::create_bind_group_layout(device, None))
            .create(device, None);

        let ppl_comp = pipeline::ComputePipelineBuilder::new(&comp_shader)
            .set_layout(&comp_layout)
            .build(device);

        // Simulation Pipeline:
        let vert_shader = shader_with_shaderc(device, include_str!("shaders/vf_paint04.glsl"), shaderc::ShaderKind::Vertex, "main", None)?;
        let frag_shader = shader_with_shaderc(device, include_str!("shaders/vf_paint04.glsl"), shaderc::ShaderKind::Fragment, "main", None)?;

        let vert_state = VertexStateBuilder::new(&vert_shader)
            .push_vert_layout(mesh.vert_buffer_layout())
            .build();

        let frag_state = FragmentStateBuilder::new(&frag_shader)
            .push_target_replace(wgpu::TextureFormat::Rgba32Float)
            .push_target_replace(wgpu::TextureFormat::Rgba32Float)
            .push_target_replace(wgpu::TextureFormat::Rgba32Float)
            .build();
        
        let pipeline_layout = PaintPipelineLayout::create_pipeline_layout(device);
        /*
        let pipeline_layout = PipelineLayoutBuilder::new()
            .push(global_uniform.get_bind_group_layout())
            .push(tex_vpf.get_bind_group_layout())
            .push(tex_color.get_bind_group_layout())
            .push(tex_float.get_bind_group_layout())
            .create(device, None);
        */

        let pipeline = RenderPipelineBuilder::new(vert_state, frag_state)
            .set_layout(&pipeline_layout)
            .build(device);

        // BlurWV Pipeline:
        let vert_shader = shader_with_shaderc(device, include_str!("shaders/vf_blurwv.glsl"), shaderc::ShaderKind::Vertex, "main", None)?;
        let frag_shader = shader_with_shaderc(device, include_str!("shaders/vf_blurwv.glsl"), shaderc::ShaderKind::Fragment, "main", None)?;

        let vert_state = VertexStateBuilder::new(&vert_shader)
            .push_vert_layout(mesh.vert_buffer_layout())
            .build();

        let frag_state = FragmentStateBuilder::new(&frag_shader)
            .push_target_replace(wgpu::TextureFormat::Rgba32Float)
            .build();

        let pipeline_layout = PipelineLayoutBuilder::new()
            .push(global_uniform.get_bind_group_layout())
            .push(tex_vpf.get_bind_group_layout())
            .create(device, None);

        let pipeline_blurwv = RenderPipelineBuilder::new(vert_state, frag_state)
            .set_layout(&pipeline_layout)
            .build(device);

        // BlurWH Pipeline:
        let vert_shader = shader_with_shaderc(device, include_str!("shaders/vf_blurwh.glsl"), shaderc::ShaderKind::Vertex, "main", None)?;
        let frag_shader = shader_with_shaderc(device, include_str!("shaders/vf_blurwh.glsl"), shaderc::ShaderKind::Fragment, "main", None)?;

        let vert_state = VertexStateBuilder::new(&vert_shader)
            .push_vert_layout(mesh.vert_buffer_layout())
            .build();

        let frag_state = FragmentStateBuilder::new(&frag_shader)
            .push_target_replace(wgpu::TextureFormat::Rgba32Float)
            .build();

        let pipeline_layout = PipelineLayoutBuilder::new()
            .push(global_uniform.get_bind_group_layout())
            .push(tex_vpf.get_bind_group_layout())
            .create(device, None);

        let pipeline_blurwh = RenderPipelineBuilder::new(vert_state, frag_state)
            .set_layout(&pipeline_layout)
            .build(device);


        // Initialisation Pipeline:
        let vert_shader = shader_with_shaderc(device, include_str!("shaders/vf_src_to_color.glsl"), shaderc::ShaderKind::Vertex, "main", None)?;
        let frag_shader = shader_with_shaderc(device, include_str!("shaders/vf_src_to_color.glsl"), shaderc::ShaderKind::Fragment, "main", None)?;

        let vert_state = VertexStateBuilder::new(&vert_shader)
            .push_vert_layout(mesh.vert_buffer_layout())
            .build();

        let frag_state = FragmentStateBuilder::new(&frag_shader)
            .push_target_replace(wgpu::TextureFormat::Rgba32Float)
            .build();
        
        let pipeline_layout = PipelineLayoutBuilder::new()
            .push(tex_src.get_bind_group_layout())
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
            pipeline_blurwh,
            pipeline_blurwv,
            pipeline_src_to_color,
            ppl_comp,
            in_buffer,
            out_buffer,
            sc: 0,
        })
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder){
        {
            let mut render_pass = RenderPassBuilder::new()
                .push_color_attachment(self.tex_color.view.color_attachment_clear())
                .begin(encoder, None);

            let mut render_pass_pipeline = render_pass.set_pipeline(&self.pipeline_src_to_color);

            render_pass_pipeline.set_bind_group(0, self.tex_src.get_bind_group(), &[]);
            
            self.mesh.draw(&mut render_pass_pipeline);
        }
    }

    pub fn step(&mut self, queue: &mut wgpu::Queue, encoder: &mut wgpu::CommandEncoder, device: &wgpu::Device){
        self.global_uniform.borrow_ref(queue).time = self.sc as f32 /60.;

        // test compute_shader
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{
                label: None,
            });

            cpass.set_pipeline(&self.ppl_comp.pipeline);
            cpass.set_bind_group(0, self.in_buffer.get_bind_group(), &[]);
            cpass.set_bind_group(1, self.out_buffer.get_bind_group(), &[]);
            cpass.dispatch(self.in_buffer.len() as u32, 1, 1);
            /*
            let mapped_memory = self.out_buffer.content.buffer.slice(..);
            let mapped_range = mapped_memory.get_mapped_range();
            let res = mapped_range.first().unwrap();
            println!("{:?}", res);

            self.out_buffer.content.buffer.unmap();
            */
        }
        println!("{:?}", self.out_buffer.slice(..).map_blocking(device).as_ref());

        // Simulation step:
        {
            let mut render_pass = RenderPassBuilder::new()
                .push_color_attachment(self.tex_vpf_tmp.view.color_attachment_clear())
                .push_color_attachment(self.tex_color_tmp.view.color_attachment_clear())
                .push_color_attachment(self.tex_float_tmp.view.color_attachment_clear())
                .begin(encoder, None);

            let mut render_pass_pipeline = render_pass.set_pipeline(&self.pipeline);

            render_pass_pipeline.set_bind_group(PaintPipelineLayout::global_uniform as u32, self.global_uniform.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group(PaintPipelineLayout::tex_vpf as u32, self.tex_vpf.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group(PaintPipelineLayout::tex_color as u32, self.tex_color.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group(PaintPipelineLayout::tex_float as u32, self.tex_float.get_bind_group(), &[]);

            self.mesh.draw(&mut render_pass_pipeline);
        }
        // Blur Vertically
        {
            self.tex_vpf_tmp.copy_all_to(&mut self.tex_vpf, encoder);
            let mut render_pass = RenderPassBuilder::new()
                .push_color_attachment(self.tex_vpf_tmp.view.color_attachment_clear())
                .begin(encoder, None);

            let mut render_pass_pipeline = render_pass.set_pipeline(&self.pipeline_blurwv);

            render_pass_pipeline.set_bind_group(0, self.global_uniform.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group(1, self.tex_vpf.get_bind_group(), &[]);

            self.mesh.draw(&mut render_pass_pipeline);
        }
        // Blur Horizontally
        {
            self.tex_vpf_tmp.copy_all_to(&mut self.tex_vpf, encoder);
            let mut render_pass = RenderPassBuilder::new()
                .push_color_attachment(self.tex_vpf_tmp.view.color_attachment_clear())
                .begin(encoder, None);

            let mut render_pass_pipeline = render_pass.set_pipeline(&self.pipeline_blurwh);

            render_pass_pipeline.set_bind_group(0, self.global_uniform.get_bind_group(), &[]);
            render_pass_pipeline.set_bind_group(1, self.tex_vpf.get_bind_group(), &[]);

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

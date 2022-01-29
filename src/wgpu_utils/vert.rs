use bytemuck::*;
use super::buffer::*;
use wgpu::util::DeviceExt;

pub trait Vert: bytemuck::Pod 
+ bytemuck::Zeroable 
+ Copy + Clone
{
    fn buffer_layout() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vert2{
    pub pos: [f32; 2],
    pub uv: [f32; 2],
}

impl Vert2{
    pub const QUAD_VERTS: [Vert2; 4] = [
        Vert2{pos: [-1.0, -1.0], uv: [0.0, 1.0]},
        Vert2{pos: [1.0, -1.0], uv: [1.0, 1.0]},
        Vert2{pos: [1.0, 1.0], uv: [1.0, 0.0]},
        Vert2{pos: [-1.0, 1.0], uv: [0.0, 0.0]},
    ];

    pub const QUAD_IDXS: [u32; 6] = [
        0, 1, 2,
        2, 3, 0,
    ];
}

impl Vert for Vert2{
    fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2
        ];
        wgpu::VertexBufferLayout{
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBS,
        }
    }
}

impl<V: Vert> ToBuffer for &[V]{
    fn create_buffer(&self, device: &wgpu::Device, usage: wgpu::BufferUsages) -> anyhow::Result<wgpu::Buffer> {
        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor{
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(*self),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        Ok(buffer)
    }
}

impl<V: Vert> ToVertBuffer for &[V]{
    fn create_vert_buffer(&self, device: &wgpu::Device) -> anyhow::Result<wgpu::Buffer>{
        self.create_buffer(device, wgpu::BufferUsages::VERTEX)
    }
}


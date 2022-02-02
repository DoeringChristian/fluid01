use anyhow::*;
use wgpu::util::DeviceExt;
use std::{marker::PhantomData, ops::{Deref, DerefMut}, borrow::{Borrow, BorrowMut}};
use binding::CreateBindGroupLayout;

use super::binding;

///
/// A struct mutably referencing a Uniform to edit its content and update it when UniformRef is
/// droped.
///
pub struct UniformRef<'ur, C: bytemuck::Pod>{
    queue: &'ur mut wgpu::Queue,
    uniform: &'ur mut Uniform<C>,
}


impl<C: bytemuck::Pod> Deref for UniformRef<'_, C>{
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.uniform.uniform_vec.content[0]
    }
}

impl<C: bytemuck::Pod> DerefMut for UniformRef<'_, C>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.uniform.uniform_vec.content[0]
    }
}

impl<C: bytemuck::Pod> Drop for UniformRef<'_, C>{
    fn drop(&mut self) {
        self.uniform.uniform_vec.update_int(self.queue);
    }
}

pub struct UniformVecRef<'ur, C: bytemuck::Pod>{
    queue: &'ur mut wgpu::Queue,
    uniform_vec: &'ur mut UniformVec<C>,
}

impl<C: bytemuck::Pod> Deref for UniformVecRef<'_, C>{
    type Target = [C];

    fn deref(&self) -> &Self::Target{
        &self.uniform_vec.content
    }
}

impl<C: bytemuck::Pod> DerefMut for UniformVecRef<'_, C>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.uniform_vec.content
    }
}

impl<C: bytemuck::Pod> Drop for UniformVecRef<'_, C>{
    fn drop(&mut self){
        self.uniform_vec.update_int(self.queue);
    }
}

pub struct Buffer<C: bytemuck::Pod>{
    buffer: wgpu::Buffer,
    len: usize,
    _pd: PhantomData<C>,
}

impl<C: bytemuck::Pod> Buffer<C>{
    pub fn new_empty(device: &wgpu::Device, usage: wgpu::BufferUsages, label: wgpu::Label, len: usize) -> Self{
        let buffer = device.create_buffer(&wgpu::BufferDescriptor{
            label,
            size: (len * std::mem::size_of::<C>()) as u64,
            usage,
            mapped_at_creation: false,
        });

        Self{
            buffer,
            len,
            _pd: PhantomData,
        }
    }

    pub fn new(device: &wgpu::Device, usage: wgpu::BufferUsages, label: wgpu::Label, data: &[C]) -> Self{
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label,
            contents: bytemuck::cast_slice(data),
            usage,
        });

        Self{
            buffer,
            len: data.len(),
            _pd: PhantomData,
        }
    }

    pub fn new_vert(device: &wgpu::Device, label: wgpu::Label, data: &[C]) -> Self{
        Self::new(device, wgpu::BufferUsages::VERTEX, label, data)
    }
    
    pub fn new_index(device: &wgpu::Device, label: wgpu::Label, data: &[C]) -> Self{
        Self::new(device, wgpu::BufferUsages::INDEX, label, data)
    }

    pub fn len(&self) -> usize{
        self.len / std::mem::size_of::<C>()
    }
}

pub struct DynamicBuffer<C: bytemuck::Pod>(Buffer<C>);

impl<C: bytemuck::Pod> DynamicBuffer<C>{
    pub fn new_empty(device: &wgpu::Device, usage: wgpu::BufferUsages, label: wgpu::Label, len: usize) -> Self{
        Self(
            Buffer::new_empty(device, usage | wgpu::BufferUsages::COPY_DST, label, len)
        )
    }

    pub fn new(device: &wgpu::Device, usage: wgpu::BufferUsages, label: wgpu::Label, data: &[C]) -> Self{
        Self(
            Buffer::new(device, usage | wgpu::BufferUsages::COPY_DST, label, data)
        )
    }

    pub fn new_vert(device: &wgpu::Device, label: wgpu::Label, data: &[C]) -> Self{
        Self::new(device, wgpu::BufferUsages::VERTEX, label, data)
    }

    pub fn new_index(device: &wgpu::Device, label: wgpu::Label, data: &[C]) -> Self{
        Self::new(device, wgpu::BufferUsages::INDEX, label, data)
    }

    pub fn new_uniform(device: &wgpu::Device, label: wgpu::Label, data: &[C]) -> Self{
        Self::new(device, wgpu::BufferUsages::UNIFORM, label, data)
    }

    pub fn write_buffer(&mut self, queue: &wgpu::Queue, offset: usize, data: &[C]){
        queue.write_buffer(&self.0.buffer, (offset * std::mem::size_of::<C>()) as u64, bytemuck::cast_slice(data));
    }

    // TODO: write resize implementation.
    pub fn resize(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, len: usize){
        let size = len * std::mem::size_of::<C>();
        unimplemented!()
    }
}

impl<C: bytemuck::Pod> Deref for DynamicBuffer<C>{
    type Target = Buffer<C>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<C: bytemuck::Pod> DerefMut for DynamicBuffer<C>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// TODO: BufferVec implementation.
pub struct BufferVec<C: bytemuck::Pod>{
    buffer: wgpu::Buffer,
    len: usize,
    capacity: usize,
    content: Vec<C>,

    _ty: PhantomData<C>,
}

impl<C: bytemuck::Pod> BufferVec<C>{

}

impl<C: bytemuck::Pod> Deref for Buffer<C>{
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<C: bytemuck::Pod> DerefMut for Buffer<C>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

pub struct UniformVec<C: bytemuck::Pod>{
    buffer: DynamicBuffer<C>,

    content: Vec<C>,
}

impl<C: bytemuck::Pod> UniformVec<C>{
    fn name() -> &'static str{
        let type_name = std::any::type_name::<C>();
        let pos = type_name.rfind(':').unwrap();
        &type_name[(pos + 1)..]
    }

    pub fn new(device: &wgpu::Device, src: &[C]) -> Self{
        let buffer = DynamicBuffer::new_uniform(
            device, 
            Some(&format!("UniformBuffer: {}", Self::name())),
            src,
        );

        Self{
            buffer,
            content: Vec::from(src),
        }
    }

    pub fn update_int(&mut self, queue: &wgpu::Queue){
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.content));
    }

    pub fn borrow_ref<'ur>(&'ur mut self, queue: &'ur mut wgpu::Queue) -> UniformVecRef<'ur, C>{
        UniformVecRef{
            queue,
            uniform_vec: self,
        }
    }
}

pub struct Uniform<C: bytemuck::Pod>{
    uniform_vec: UniformVec<C>,
}

impl<C: bytemuck::Pod> Uniform<C>{
    pub fn new(device: &wgpu::Device, src: C) -> Self{
        Self{
            uniform_vec: UniformVec::new(device, &[src])
        }
    }

    pub fn borrow_ref<'ur>(&'ur mut self, queue: &'ur mut wgpu::Queue) -> UniformRef<'ur, C>{
        UniformRef{
            queue,
            uniform: self,
        }
    }
}

impl<C: bytemuck::Pod> binding::BindGroupContent for Uniform<C>{
    fn push_entries_to(bind_group_layout_builder: &mut binding::BindGroupLayoutBuilder) {
        bind_group_layout_builder.push_entry_all_ref(binding::wgsl::uniform());
    }

    fn push_resources_to<'bgb>(&'bgb self, bind_group_builder: &mut binding::BindGroupBuilder<'bgb>) {
        bind_group_builder.resource_ref(self.uniform_vec.buffer.as_entire_binding());
    }
}

/*
impl<C: bytemuck::Pod> Deref for Uniform<C>{
    type Target = UniformVec<C>;

    fn deref(&self) -> &Self::Target {
        &self.uniform_vec
    }
}

impl<C: bytemuck::Pod> DerefMut for Uniform<C>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.uniform_vec
    }
}
*/

pub type UniformBindGroup<C> = binding::BindGroup<Uniform<C>>;

impl<C: bytemuck::Pod> UniformBindGroup<C>{
    pub fn new_zeroed(device: &wgpu::Device) -> Self{
        binding::BindGroup::new(Uniform::new(device, C::zeroed()), device)
    }

    pub fn new_with_data(device: &wgpu::Device, src: C) -> Self{
        binding::BindGroup::new(Uniform::new(device, src), device)
    }
}

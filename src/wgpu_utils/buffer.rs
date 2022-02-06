use anyhow::*;
use wgpu::util::DeviceExt;
use std::{marker::PhantomData, ops::{Deref, DerefMut}, borrow::{Borrow, BorrowMut}};
use binding::CreateBindGroupLayout;

use super::binding;


pub struct Buffer<C: bytemuck::Pod>{
    pub buffer: wgpu::Buffer,
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

    pub fn new_comp(device: &wgpu::Device, label: wgpu::Label, data: &[C]) -> Self{
        Self::new(device, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::MAP_READ, label, data)
    }
    
    pub fn new_index(device: &wgpu::Device, label: wgpu::Label, data: &[C]) -> Self{
        Self::new(device, wgpu::BufferUsages::INDEX, label, data)
    }

    pub fn len(&self) -> usize{
        self.len / std::mem::size_of::<C>()
    }
}

impl<C: bytemuck::Pod> binding::BindGroupContent for Buffer<C>{
    fn push_entries_to(bind_group_layout_builder: &mut binding::BindGroupLayoutBuilder) {
        bind_group_layout_builder.push_entry_all_ref(binding::wgsl::buffer(false))
    }

    fn push_resources_to<'bgb>(&'bgb self, bind_group_builder: &mut binding::BindGroupBuilder<'bgb>) {
        bind_group_builder.resource_ref(self.as_entire_binding())
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

impl<C: bytemuck::Pod> binding::BindGroupContent for DynamicBuffer<C>{
    fn push_entries_to(bind_group_layout_builder: &mut binding::BindGroupLayoutBuilder) {
        bind_group_layout_builder.push_entry_all_ref(binding::wgsl::buffer(false))
    }

    fn push_resources_to<'bgb>(&'bgb self, bind_group_builder: &mut binding::BindGroupBuilder<'bgb>) {
        bind_group_builder.resource_ref(self.as_entire_binding())
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

pub struct MappedBuffer<C: bytemuck::Pod>(Buffer<C>);

impl<C: bytemuck::Pod> MappedBuffer<C>{
    pub fn new_empty(device: &wgpu::Device, usage: wgpu::BufferUsages, label: wgpu::Label, len: usize) -> Self{
        Self(
            Buffer::new_empty(device, usage | wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::MAP_WRITE, label, len)
        )
    }

    pub fn new(device: &wgpu::Device, usage: wgpu::BufferUsages, label: wgpu::Label, data: &[C]) -> Self{
        Self(
            Buffer::new(device, usage | wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::MAP_WRITE, label, data)
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
}

impl<C: bytemuck::Pod> Deref for MappedBuffer<C>{
    type Target = Buffer<C>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<C: bytemuck::Pod> DerefMut for MappedBuffer<C>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

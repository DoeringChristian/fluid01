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
    queue: &'ur wgpu::Queue,
    uniform: &'ur mut Uniform<C>,
}

impl<C: bytemuck::Pod> Deref for UniformRef<'_, C>{
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.uniform.content
    }
}

impl<C: bytemuck::Pod> DerefMut for UniformRef<'_, C>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.uniform.content
    }
}

impl<C: bytemuck::Pod> Drop for UniformRef<'_, C>{
    fn drop(&mut self) {
        self.uniform.update_int(self.queue);
    }
}

pub struct Buffer<C: bytemuck::Pod>{
    buffer: wgpu::Buffer,
    size: usize,
    _pd: PhantomData<C>,
}

impl<C: bytemuck::Pod> Buffer<C>{

    pub fn new(device: &wgpu::Device, usage: wgpu::BufferUsages, label: wgpu::Label, num: usize) -> Self{
        let size = std::mem::size_of::<C>() * num;
        
        let buffer = device.create_buffer(&wgpu::BufferDescriptor{
            label,
            size: size as u64,
            usage,
            mapped_at_creation: false,
        });

        Self{
            buffer,
            size,
            _pd: PhantomData,
        }
    }

    pub fn new_with_data(device: &wgpu::Device, usage: wgpu::BufferUsages, label: wgpu::Label, data: &[C]) -> Self{
        let size = std::mem::size_of::<C>() * data.len();

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label,
            contents: bytemuck::cast_slice(data),
            usage,
        });

        Self{
            buffer,
            size,
            _pd: PhantomData,
        }
    }

    pub fn new_vert_with_data(device: &wgpu::Device, label: wgpu::Label, data: &[C]) -> Self{
        Self::new_with_data(device, wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC, label, data)
    }
    
    pub fn new_index_with_data(device: &wgpu::Device, label: wgpu::Label, data: &[C]) -> Self{
        Self::new_with_data(device, wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC, label, data)
    }

    pub fn write_buffer(&mut self, queue: &wgpu::Queue, offset: usize, data: &[C]){
        queue.write_buffer(&self.buffer, (offset * std::mem::size_of::<C>()) as u64, bytemuck::cast_slice(data));
    }

    // TODO: write resize implementation.
    pub fn resize(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, len: usize){
        let size = len * std::mem::size_of::<C>();
        unimplemented!()
    }

    pub fn len(&self) -> usize{
        self.size / std::mem::size_of::<C>()
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




pub struct Uniform<C: bytemuck::Pod>{
    buffer: wgpu::Buffer,
    _pd: PhantomData<C>,

    content: C,
}

impl<C: bytemuck::Pod> Uniform<C>{
    fn name() -> &'static str{
        let type_name = std::any::type_name::<C>();
        let pos = type_name.rfind(':').unwrap();
        &type_name[(pos + 1)..]
    }
    
    pub fn new(device: &wgpu::Device) -> Self{
        let buffer = device.create_buffer(&wgpu::BufferDescriptor{
            label: Some(&format!("UniformBuffer: {}", Self::name())),
            size: std::mem::size_of::<C>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Uniform{
            buffer,
            _pd: PhantomData,
            content: C::zeroed(),
        }
    }

    pub fn borrow_ref<'ur>(&'ur mut self, queue: &'ur wgpu::Queue) -> UniformRef<'ur, C>{
        UniformRef{
            queue,
            uniform: self,
        }
    }

    pub fn new_with_data(device: &wgpu::Device, src: C) -> Self{
        let buffer = device.create_buffer(&wgpu::BufferDescriptor{
            label: Some(&format!("UniformBuffer: {}", Self::name())),
            size: std::mem::size_of::<C>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        let mapped_memory = buffer.slice(..);
        mapped_memory.get_mapped_range_mut().clone_from_slice(bytemuck::bytes_of(&src));

        buffer.unmap();

        Self{
            buffer,
            _pd: PhantomData,
            content: src,
        }
    }

    pub fn update_int(&mut self, queue: &wgpu::Queue){
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&self.content));
    }

    pub fn binding_resource(&self) -> wgpu::BindingResource{
        self.buffer.as_entire_binding()
    }

}

impl<C: bytemuck::Pod> binding::BindGroupContent for Uniform<C>{
    fn push_entries_to(bind_group_layout_builder: &mut binding::BindGroupLayoutBuilder) {
        bind_group_layout_builder.push_entry_all_ref(binding::wgsl::uniform());
    }

    fn push_resources_to<'bgb>(&'bgb self, bind_group_builder: &mut binding::BindGroupBuilder<'bgb>) {
        bind_group_builder.resource_ref(self.buffer.as_entire_binding());
    }
}

pub type UniformBindGroup<C> = binding::BindGroup<Uniform<C>>;

impl<C: bytemuck::Pod> UniformBindGroup<C>{
    pub fn new_zeroed(device: &wgpu::Device) -> Self{
        binding::BindGroup::new(Uniform::new_with_data(device, C::zeroed()), device)
    }

    pub fn new_with_data(device: &wgpu::Device, src: C) -> Self{
        binding::BindGroup::new(Uniform::new_with_data(device, src), device)
    }
}

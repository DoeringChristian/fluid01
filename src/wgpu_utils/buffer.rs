use anyhow::*;
use wgpu::util::DeviceExt;
use std::{marker::PhantomData, ops::{Deref, DerefMut}, borrow::Borrow};
use binding::CreateBindGroupLayout;

use super::binding;

pub trait ToBuffer{
    fn create_buffer(&self, device: &wgpu::Device, usage: wgpu::BufferUsages) -> Result<wgpu::Buffer>;
}

pub trait ToVertBuffer: ToBuffer{
    fn create_vert_buffer(&self, device: &wgpu::Device) -> Result<wgpu::Buffer>;
}

pub trait ToIdxBuffer: ToBuffer{
    fn create_idx_buffer(&self, device: &wgpu::Device) -> Result<wgpu::Buffer>;
}

pub trait ToUniformBuffer{
    //fn uniform_label() -> &'static str;
    fn create_uniform_buffer(&self, device: &wgpu::Device) -> Result<wgpu::Buffer>;
    fn update_uniform_buffer(&self, queue: &wgpu::Queue, dst: &mut wgpu::Buffer);
}


impl<T: bytemuck::Pod> ToUniformBuffer for T{
    fn create_uniform_buffer(&self, device: &wgpu::Device) -> Result<wgpu::Buffer> {

        let buffer = device.create_buffer(&wgpu::BufferDescriptor{
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<Self>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        let mapped_memory = buffer.slice(..);
        mapped_memory.get_mapped_range_mut().clone_from_slice(bytemuck::bytes_of(self));

        buffer.unmap();

        Ok(buffer)
    }

    fn update_uniform_buffer(&self, queue: &wgpu::Queue, dst: &mut wgpu::Buffer) {
        queue.write_buffer(&dst, 0, bytemuck::bytes_of(self));
    }
}



impl ToBuffer for &[u32]{
    fn create_buffer(&self, device: &wgpu::Device, usage: wgpu::BufferUsages) -> Result<wgpu::Buffer> {
        Ok(device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: None,
            contents: bytemuck::cast_slice(*self),
            usage,
        }))
    }
}

impl ToIdxBuffer for &[u32]{
    fn create_idx_buffer(&self, device: &wgpu::Device) -> Result<wgpu::Buffer> {
        self.create_buffer(device, wgpu::BufferUsages::INDEX)
    }
}

///
/// A struct mutably referencing a Uniform to edit its content and update it when UniformRef is
/// droped.
///
pub struct UniformRef<'ur, C: bytemuck::Pod>{
    pub queue: &'ur wgpu::Queue,
    pub uniform: &'ur mut Uniform<C>,
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

// TODO: remove new without data and add content directly as type.
pub struct Uniform<C: bytemuck::Pod>{
    buffer: wgpu::Buffer,
    content_type: PhantomData<C>,

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

        let binding_group_layout = binding::BindGroupLayoutBuilder::new()
            .push_entry_all(binding::wgsl::uniform())
            .create(device, None);

        let binding_group = binding::BindGroupBuilder::new(&binding_group_layout)
            .resource(buffer.as_entire_binding())
            .create(device, None);

        Uniform{
            buffer,
            content_type: PhantomData,
            content: C::zeroed(),
        }
    }

    pub fn borrow_ref<'ur>(&'ur mut self, queue: &'ur wgpu::Queue) -> UniformRef<'ur, C>{
        UniformRef{
            queue,
            uniform: self,
        }
    }

    pub fn new_with_data(device: &wgpu::Device, src: &C) -> Self{
        let buffer = device.create_buffer(&wgpu::BufferDescriptor{
            label: Some(&format!("UniformBuffer: {}", Self::name())),
            size: std::mem::size_of::<C>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        let mapped_memory = buffer.slice(..);
        mapped_memory.get_mapped_range_mut().clone_from_slice(bytemuck::bytes_of(src));

        buffer.unmap();

        let binding_group_layout = binding::BindGroupLayoutBuilder::new()
            .push_entry_all(binding::wgsl::uniform())
            .create(device, None);

        let binding_group = binding::BindGroupBuilder::new(&binding_group_layout)
            .resource(buffer.as_entire_binding())
            .create(device, None);

        Self{
            buffer,
            content_type: PhantomData,
            content: *src,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, src: &C){
        let new_content = bytemuck::bytes_of(src);
        if bytemuck::bytes_of(&self.content) == new_content{
            return;
        }

        queue.write_buffer(&self.buffer, 0, new_content);
        self.content = *src;
    }

    pub fn update_int(&mut self, queue: &wgpu::Queue){
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&self.content));
    }

    pub fn binding_resource(&self) -> wgpu::BindingResource{
        self.buffer.as_entire_binding()
    }

}

impl<C: bytemuck::Pod> binding::BindGroupContent for Uniform<C>{
    fn push_entries_to(&self, bind_group_layout_builder: &mut binding::BindGroupLayoutBuilder) {
        bind_group_layout_builder.push_entry_all_ref(binding::wgsl::uniform());
    }

    fn push_resources_to<'bgb>(&'bgb self, bind_group_builder: &mut binding::BindGroupBuilder<'bgb>) {
        bind_group_builder.resource_ref(self.buffer.as_entire_binding());
    }
}

impl<C: bytemuck::Pod> binding::CreateBindGroupLayout for Uniform<C>{
    fn create_bind_group_layout(device: &wgpu::Device, label: Option<&str>) -> binding::BindGroupLayoutWithDesc {
        binding::BindGroupLayoutBuilder::new()
            .push_entry_all(binding::wgsl::uniform())
            .create(device, None)
    }
}

impl<C: bytemuck::Pod> binding::CreateBindGroup for Uniform<C>{
    fn create_bind_group(&self, device: &wgpu::Device, layout: &binding::BindGroupLayoutWithDesc, label: Option<&str>) -> wgpu::BindGroup {
        let bind_group_layout = Uniform::<C>::create_bind_group_layout(device, None);
        binding::BindGroupBuilder::new(&bind_group_layout)
            .resource(self.buffer.as_entire_binding())
            .create(device, label)
    }
}

pub type UniformBindGroup<C> = binding::BindGroup<Uniform<C>>;

impl<C: bytemuck::Pod> UniformBindGroup<C>{
    pub fn new_with_data(device: &wgpu::Device, src: &C) -> Self{
        binding::BindGroup::new(Uniform::new_with_data(device, src), device)
    }
}

/*
pub struct UniformBindGroup<C>{
    uniform_buffer: UniformBuffer<C>,

    pub binding_group_layout: binding::BindGroupLayoutWithDesc,
    pub binding_group: wgpu::BindGroup,
}

impl<C: bytemuck::Pod> UniformBindGroup<C>{
    fn name() -> &'static str{
        let type_name = std::any::type_name::<C>();
        let pos = type_name.rfind(':').unwrap();
        &type_name[(pos + 1)..]
    }
    
    pub fn new(device: &wgpu::Device) -> Self{

        let uniform_buffer = UniformBuffer::new(device);

        let binding_group_layout = binding::BindGroupLayoutBuilder::new()
            .push_entry_all(binding::wgsl::uniform())
            .create(device, None);

        let binding_group = binding::BindGroupBuilder::new(&binding_group_layout)
            .resource(uniform_buffer.binding_resource())
            .create(device, None);

        UniformBindGroup{
            uniform_buffer,
            binding_group_layout,
            binding_group,
        }
    }

    pub fn new_with_data(device: &wgpu::Device, src: &C) -> Self{
        let uniform_buffer = UniformBuffer::new_with_data(device, src);

        let binding_group_layout = binding::BindGroupLayoutBuilder::new()
            .push_entry_all(binding::wgsl::uniform())
            .create(device, None);

        let binding_group = binding::BindGroupBuilder::new(&binding_group_layout)
            .resource(uniform_buffer.binding_resource())
            .create(device, None);

        Self{
            uniform_buffer,
            binding_group_layout,
            binding_group,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, src: &C){
        self.uniform_buffer.update(queue, src)
    }

    pub fn update_int(&mut self, queue: &wgpu::Queue){
        self.uniform_buffer.update_int(queue);
    }

    pub fn get_content(&mut self) -> &mut C{
        self.uniform_buffer.get_content()
    }
}

impl<C: bytemuck::Pod> binding::GetBindGroupLayout for UniformBindGroup<C>{
    fn get_bind_group_layout<'l>(&'l self) -> &'l binding::BindGroupLayoutWithDesc {
        &self.binding_group_layout
    }
}

impl<C: bytemuck::Pod> binding::GetBindGroup for UniformBindGroup<C>{
    fn get_bind_group<'l>(&'l self) -> &'l wgpu::BindGroup {
        &self.binding_group
    }
}

impl<C: bytemuck::Pod> binding::CreateBindGroupLayout for UniformBindGroup<C>{
    fn create_bind_group_layout(device: &wgpu::Device, label: Option<&str>) -> binding::BindGroupLayoutWithDesc {
        binding::BindGroupLayoutBuilder::new()
            .push_entry_all(binding::wgsl::uniform())
            .create(device, None)
    }
}

impl<C: bytemuck::Pod> binding::CreateBindGroupLayoutVT for UniformBindGroup<C>{
    fn create_bind_group_layout_vt(&self, device: &wgpu::Device, label: Option<&str>) -> binding::BindGroupLayoutWithDesc {
        binding::BindGroupLayoutBuilder::new()
            .push_entry_all(binding::wgsl::uniform())
            .create(device, label)
    }
}
*/

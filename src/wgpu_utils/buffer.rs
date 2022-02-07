use anyhow::*;
use wgpu::util::DeviceExt;
use std::{marker::PhantomData, ops::{Deref, DerefMut, RangeBounds}, borrow::{Borrow, BorrowMut}, future, cell::Cell};
use binding::CreateBindGroupLayout;
use std::mem::ManuallyDrop;

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

    pub fn new_storage(device: &wgpu::Device, label: wgpu::Label, data: &[C]) -> Self{
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

pub struct MappedBufferView<'mbr, C: bytemuck::Pod>{
    mapped_buffer: &'mbr MappedBuffer<C>,
    buffer_view: ManuallyDrop<wgpu::BufferView<'mbr>>,
}

impl<'mbr, C: bytemuck::Pod> AsRef<[C]> for MappedBufferView<'mbr, C>{
    fn as_ref(&self) -> &[C] {
        bytemuck::cast_slice(self.buffer_view.as_ref())
    }
}

impl<'mbr, C: bytemuck::Pod> Deref for MappedBufferView<'mbr, C>{
    type Target = [C];

    fn deref(&self) -> &Self::Target {
        bytemuck::cast_slice(self.buffer_view.as_ref())
    }
}

impl<'mbr, C: bytemuck::Pod> Drop for MappedBufferView<'mbr, C>{
    fn drop(&mut self) {
        // SAFETY: Dropping buffer view before unmap is required.
        // self.buffer_view is also not used afterwards.
        unsafe{
            ManuallyDrop::drop(&mut self.buffer_view);
        }
        self.mapped_buffer.unmap();
    }
}

pub struct MappedBufferViewMut<'mbr, C: bytemuck::Pod>{
    mapped_buffer: &'mbr MappedBuffer<C>,
    buffer_view: ManuallyDrop<wgpu::BufferViewMut<'mbr>>,
}

impl<'mbr, C: bytemuck::Pod> AsMut<[C]> for MappedBufferViewMut<'mbr, C>{
    fn as_mut(&mut self) -> &mut [C] {
        bytemuck::cast_slice_mut(self.buffer_view.as_mut())
    }
}

impl<'mbr, C: bytemuck::Pod> Deref for MappedBufferViewMut<'mbr, C>{
    type Target = [C];

    fn deref(&self) -> &Self::Target {
        bytemuck::cast_slice(self.buffer_view.as_ref())
    }
}

impl<'mbr, C: bytemuck::Pod> DerefMut for MappedBufferViewMut<'mbr, C>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        bytemuck::cast_slice_mut(self.buffer_view.as_mut())
    }
}

impl<'mbr, C: bytemuck::Pod> Drop for MappedBufferViewMut<'mbr, C>{
    fn drop(&mut self) {
        // SAFETY: Dropping buffer view before unmap is required.
        // self.buffer_view is also not used afterwards.
        unsafe{
            ManuallyDrop::drop(&mut self.buffer_view);
        }
        self.mapped_buffer.buffer.unmap();
    }
}

///
/// A MappedBuffer is a Buffer, that can be mapped into CPU Memory.
///
/// It wraps the wgpu::Buffer with the content of type C to prevent type missmatch.
///
/// ```rust
/// let array = [0, 1, 2, 3, 4];
/// let mapped_buffer = MappedBuffer::new_storage(device, None, array);
///
/// mapped_buffer.slice_blocking(..)[0] = 1;
///
/// let i = mapped_buffer.slice(..)[0];
/// ```
/// TODO: Add new_mapped_at_creation.
pub struct MappedBuffer<C: bytemuck::Pod>{ 
    buffer: Buffer<C>,
}

impl<C: bytemuck::Pod> MappedBuffer<C>{
    pub fn new_empty(device: &wgpu::Device, usage: wgpu::BufferUsages, label: wgpu::Label, len: usize) -> Self{
        Self{ 
            buffer: Buffer::new_empty(device, usage | wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::MAP_WRITE, label, len),
        }
    }

    pub fn new(device: &wgpu::Device, usage: wgpu::BufferUsages, label: wgpu::Label, data: &[C]) -> Self{
        Self{ 
            buffer: Buffer::new(device, usage | wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::MAP_WRITE, label, data),
        }
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

    pub fn new_storage(device: &wgpu::Device, label: wgpu::Label, data: &[C]) -> Self{
        Self::new(device, wgpu::BufferUsages::STORAGE, label, data)
    }

    ///
    /// Get an imutable mapped buffer view to the buffer.
    ///
    /// self has to be mut since we only can only have one mapped buffer view.
    ///
    pub async fn slice_async<'mbr, S: RangeBounds<wgpu::BufferAddress>>(&'mbr mut self, bounds: S, device: &wgpu::Device) -> MappedBufferView<'mbr, C>{
        let buffer_slice = self.buffer.slice(bounds);
        let mapping = buffer_slice.map_async(wgpu::MapMode::Read);

        device.poll(wgpu::Maintain::Wait);

        mapping.await.unwrap();

        MappedBufferView{
            mapped_buffer: self,
            buffer_view: ManuallyDrop::new(buffer_slice.get_mapped_range()),
        }
    }

    ///
    /// Get an imutable MappedBufferView to the buffer blocking the thread (not recomended).
    ///
    pub fn slice_blocking<'mbr, S: RangeBounds<wgpu::BufferAddress>>(&'mbr mut self, bounds: S, device: &wgpu::Device) -> MappedBufferView<'mbr, C>{
        pollster::block_on(self.slice_async(bounds, device))
    }

    // TODO: async and not async methodes for slicing buffer
    ///
    /// Get MappedBufferViewMut to the buffer.
    /// Only for writing to the buffer.
    ///
    pub async fn slice_async_mut<'mbr, S: RangeBounds<wgpu::BufferAddress>>(&'mbr mut self, bounds: S, device: &wgpu::Device) -> MappedBufferViewMut<'mbr, C>{
        let buffer_slice = self.buffer.slice(bounds);
        let mapping = buffer_slice.map_async(wgpu::MapMode::Write);

        device.poll(wgpu::Maintain::Wait);

        mapping.await.unwrap();

        MappedBufferViewMut{
            mapped_buffer: self,
            buffer_view: ManuallyDrop::new(buffer_slice.get_mapped_range_mut()),
        }
    }

    pub fn slice_blocking_mut<'mbr, S: RangeBounds<wgpu::BufferAddress>>(&'mbr mut self, bounds: S, device: &wgpu::Device) -> MappedBufferViewMut<'mbr, C>{
        pollster::block_on(self.slice_async_mut(bounds, device))
    }
}

impl<C: bytemuck::Pod> Deref for MappedBuffer<C>{
    type Target = Buffer<C>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<C: bytemuck::Pod> DerefMut for MappedBuffer<C>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

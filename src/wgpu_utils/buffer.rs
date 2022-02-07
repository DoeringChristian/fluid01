use anyhow::*;
use wgpu::util::DeviceExt;
use std::{marker::PhantomData, ops::{Deref, DerefMut, RangeBounds}, borrow::{Borrow, BorrowMut}, future, cell::Cell};
use binding::CreateBindGroupLayout;
use std::mem::ManuallyDrop;
use std::ops::Bound;

use super::binding;

/*
pub enum BufferTyped<C: bytemuck::Pod>{
    SrcBuffer(Buffer<C>),
    DstBuffer(Buffer<C>),
}
*/

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
        self.len
    }

    /// TODO: Add buffer slice and copy operator for following syntax: 
    ///
    /// ``` rust
    /// buffer.slice(0..3).copy(offset).to(dst_buffer);
    /// ```
    pub fn copy_to_buffer<S: RangeBounds<wgpu::BufferAddress>>(&self, encoder: &mut wgpu::CommandEncoder, dst: &mut Buffer<C>, src_bounds: S, dst_offset: wgpu::BufferAddress){
        let start_bound = src_bounds.start_bound();
        let end_bound = src_bounds.end_bound();

        let start_bound = match start_bound{
            Bound::Unbounded => 0 as wgpu::BufferAddress,
            Bound::Included(offset) => {offset + 0},
            Bound::Excluded(offset) => {offset + 1},
        };

        let end_bound = match end_bound{
            Bound::Unbounded => {(self.len() -1) as wgpu::BufferAddress},
            Bound::Included(offset) => {offset - 0},
            Bound::Excluded(offset) => {offset - 1},
        };

        let start_bound = start_bound * std::mem::size_of::<C>() as u64;
        let end_bound = end_bound * std::mem::size_of::<C>() as u64;

        let copy_size = end_bound - start_bound;

        let dst_offset = dst_offset * std::mem::size_of::<C>() as u64;

        encoder.copy_buffer_to_buffer(
            &self.buffer,
            start_bound,
            &dst.buffer,
            dst_offset,
            copy_size
        );
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
/// A slice of the buffer that can be mapped.
///
pub struct MappedBufferSlice<'mbs, C: bytemuck::Pod>{
    mapped_buffer: &'mbs MappedBuffer<C>,
    buffer_slice: wgpu::BufferSlice<'mbs>,
}

impl<'mbs, C: bytemuck::Pod> MappedBufferSlice<'mbs, C>{
    ///
    /// Map the slice whilst polling the device.
    ///
    pub async fn map_async_poll(self, device: &wgpu::Device) -> MappedBufferView<'mbs, C>{
        let mapping = self.buffer_slice.map_async(wgpu::MapMode::Read);

        device.poll(wgpu::Maintain::Wait);

        mapping.await.unwrap();

        MappedBufferView{
            mapped_buffer: self.mapped_buffer,
            buffer_view: ManuallyDrop::new(self.buffer_slice.get_mapped_range()),
        }
    }

    ///
    /// Map the slice asynchronously.
    /// wgpu::Device::poll has to be called before this Future will complete.
    ///
    pub async fn map_async(self) -> MappedBufferView<'mbs, C>{
        let mapping = self.buffer_slice.map_async(wgpu::MapMode::Read);

        mapping.await.unwrap();

        MappedBufferView{
            mapped_buffer: self.mapped_buffer,
            buffer_view: ManuallyDrop::new(self.buffer_slice.get_mapped_range()),
        }
    }

    ///
    /// Map the slice and block this thread untill maping is complete.
    ///
    /// ```rust
    /// println!("{}", slice.map_blocking(device)[0]);
    /// ```
    ///
    pub fn map_blocking(self, device: &wgpu::Device) -> MappedBufferView<'mbs, C>{
        pollster::block_on(self.map_async_poll(device))
    }

    ///
    /// Map the slice mutably whilst polling the device.
    ///
    ///
    pub async fn map_async_poll_mut(self, device: &wgpu::Device) -> MappedBufferViewMut<'mbs, C>{
        let mapping = self.buffer_slice.map_async(wgpu::MapMode::Write);

        device.poll(wgpu::Maintain::Wait);

        mapping.await.unwrap();

        MappedBufferViewMut{
            mapped_buffer: self.mapped_buffer,
            buffer_view: ManuallyDrop::new(self.buffer_slice.get_mapped_range_mut()),
        }
    }

    ///
    /// Map the slice asynchronously for writing to the buffer.
    /// wgpu::Device::poll has to be called before this Future will complete.
    ///
    pub async fn map_async_mut(self) -> MappedBufferViewMut<'mbs, C>{
        let mapping = self.buffer_slice.map_async(wgpu::MapMode::Write);

        mapping.await.unwrap();

        MappedBufferViewMut{
            mapped_buffer: self.mapped_buffer,
            buffer_view: ManuallyDrop::new(self.buffer_slice.get_mapped_range_mut()),
        }
    }

    ///
    /// Map the slice mutably and block this thread untill maping is complete.
    ///
    /// ```rust
    /// slice.map_blocking_mut(device)[0] = 1;
    /// ```
    ///
    pub fn map_blocking_mut(self, device: &wgpu::Device) -> MappedBufferViewMut<'mbs, C>{
        pollster::block_on(self.map_async_poll_mut(device))
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
    /// Get a slice of the buffer for mapping.
    ///
    /// ```rust
    /// mapped_buffer.slice(..).map_blocking_mut(device)[0] = 1;
    /// println!("{}", mapped_buffer.slice(..).map_blocking(device)[0]) // should return 1
    /// ```
    ///
    pub fn slice<'mbr, S: RangeBounds<wgpu::BufferAddress>>(&'mbr mut self, bounds: S) -> MappedBufferSlice<'mbr, C>{
        MappedBufferSlice{
            mapped_buffer: self,
            buffer_slice: self.buffer.slice(bounds),
        }
    }
}

impl<C: bytemuck::Pod> binding::BindGroupContent for MappedBuffer<C>{
    fn push_entries_to(bind_group_layout_builder: &mut binding::BindGroupLayoutBuilder) {
        Buffer::<C>::push_entries_to(bind_group_layout_builder);
    }

    fn push_resources_to<'bgb>(&'bgb self, bind_group_builder: &mut binding::BindGroupBuilder<'bgb>) {
        self.buffer.push_resources_to(bind_group_builder);
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

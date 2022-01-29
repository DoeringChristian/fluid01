#[allow(unused)]
use anyhow::*;
use std::marker::PhantomData;
use std::sync::Arc;
use std::any::{Any, TypeId};

pub trait ToBindGroupLayout{
    fn create_bind_group_layout(device: &wgpu::Device, label: Option<&str>) -> BindGroupLayoutWithDesc;
}

pub trait GetBindGroupLayout{
    fn get_bind_group_layout<'l>(&'l self) -> &'l BindGroupLayoutWithDesc;
}

pub trait GetBindGroup{
    fn get_bind_group<'l>(&'l self) -> &'l wgpu::BindGroup;
}



pub trait ToBindGroup: ToBindGroupLayout{
    fn create_bind_group(&self, device: &wgpu::Device, layout: &BindGroupLayoutWithDesc, label: Option<&str>) -> wgpu::BindGroup;
}

pub trait ToBindGroupLayouts{
    fn bind_group_layouts<'l>(&'l self) -> Vec<&'l wgpu::BindGroupLayout>;
}



pub struct BindGroupLayoutWithDesc{
    pub layout: wgpu::BindGroupLayout,
    pub entries: Vec<wgpu::BindGroupLayoutEntry>,
}

pub struct BindGroupLayoutBuilder{
    index: u32,
    entries: Vec<wgpu::BindGroupLayoutEntry>,
}

impl BindGroupLayoutBuilder{
    pub fn new() -> Self{
        Self{
            index: 0,
            entries: Vec::new(),
        }
    }

    pub fn entry(mut self, entry: wgpu::BindGroupLayoutEntry) -> Self{
        self.entries.push(entry);
        self.index = entry.binding + 1;
        self
    }

    pub fn push_entry(self, visibility: wgpu::ShaderStages, ty: wgpu::BindingType) -> Self{
        let binding = self.index;
        self.entry(wgpu::BindGroupLayoutEntry{
            binding,
            visibility,
            ty,
            count: None,
        })
    }

    pub fn push_entry_compute(self, ty: wgpu::BindingType) -> Self{
        self.push_entry(wgpu::ShaderStages::COMPUTE, ty)
    }

    pub fn push_entry_fragment(self, ty: wgpu::BindingType) -> Self{
        self.push_entry(wgpu::ShaderStages::FRAGMENT, ty)
    }

    pub fn push_entry_vertex(self, ty: wgpu::BindingType) -> Self{
        self.push_entry(wgpu::ShaderStages::VERTEX, ty)
    }

    pub fn push_entry_all(self, ty: wgpu::BindingType) -> Self{
        self.push_entry(wgpu::ShaderStages::all(), ty)
    }

    pub fn create(self, device: &wgpu::Device, label: Option<&str>) -> BindGroupLayoutWithDesc{
        BindGroupLayoutWithDesc{
            layout: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
                entries: &self.entries,
                label,
            }),
            entries: self.entries,
        }
    }
}

pub struct BindGroupBuilder<'l>{
    layout_with_desc: &'l BindGroupLayoutWithDesc,
    entries: Vec<wgpu::BindGroupEntry<'l>>,
}

impl<'l> BindGroupBuilder<'l>{
    pub fn new(layout_with_desc: &'l BindGroupLayoutWithDesc) -> Self{
        BindGroupBuilder{
            layout_with_desc,
            entries: Vec::with_capacity(layout_with_desc.entries.len()),
        }
    }

    pub fn resource(mut self, resource: wgpu::BindingResource<'l>) -> Self{
        assert_lt!(self.entries.len(), self.layout_with_desc.entries.len());
        self.entries.push(wgpu::BindGroupEntry{
            binding: self.layout_with_desc.entries[self.entries.len()].binding,
            resource,
        });
        self
    }

    pub fn sampler(mut self, sampler: &'l wgpu::Sampler) -> Self{
        self.resource(wgpu::BindingResource::Sampler(sampler))
    }

    pub fn texture(mut self, texture_view: &'l wgpu::TextureView) -> Self{
        self.resource(wgpu::BindingResource::TextureView(texture_view))
    }

    pub fn create(&self, device: &wgpu::Device, label: Option<&str>) -> wgpu::BindGroup{
        assert_eq!(self.entries.len(), self.layout_with_desc.entries.len());
        device.create_bind_group(&wgpu::BindGroupDescriptor{
            label,
            layout: &self.layout_with_desc.layout,
            entries: &self.entries,
        })
    }
}


mod glsl{
    pub fn buffer(read_only: bool) -> wgpu::BindingType {
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    }

    pub fn uniform() -> wgpu::BindingType {
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    }

    pub fn sampler(filtering: bool) -> wgpu::BindingType {
        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
    }

    #[allow(non_snake_case)]
    pub fn texture2D() -> wgpu::BindingType {
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        }
    }

    #[allow(non_snake_case)]
    pub fn texture2DArray() -> wgpu::BindingType {
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2Array,
            multisampled: false,
        }
    }

    #[allow(non_snake_case)]
    pub fn itexture2D() -> wgpu::BindingType {
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Sint,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        }
    }

    #[allow(non_snake_case)]
    pub fn utexture2D() -> wgpu::BindingType {
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Uint,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        }
    }

    #[allow(non_snake_case)]
    pub fn texture3D() -> wgpu::BindingType {
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D3,
            multisampled: false,
        }
    }

    #[allow(non_snake_case)]
    pub fn itexture3D() -> wgpu::BindingType {
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Sint,
            view_dimension: wgpu::TextureViewDimension::D3,
            multisampled: false,
        }
    }

    #[allow(non_snake_case)]
    pub fn utexture3D() -> wgpu::BindingType {
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Uint,
            view_dimension: wgpu::TextureViewDimension::D3,
            multisampled: false,
        }
    }

    #[allow(non_snake_case)]
    pub fn textureCube() -> wgpu::BindingType {
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::Cube,
            multisampled: false,
        }
    }

    pub fn image2D(format: wgpu::TextureFormat, access: wgpu::StorageTextureAccess) -> wgpu::BindingType {
        wgpu::BindingType::StorageTexture {
            access,
            view_dimension: wgpu::TextureViewDimension::D2,
            format: format,
        }
    }

    pub fn image2DArray(format: wgpu::TextureFormat, access: wgpu::StorageTextureAccess) -> wgpu::BindingType {
        wgpu::BindingType::StorageTexture {
            access,
            view_dimension: wgpu::TextureViewDimension::D2Array,
            format: format,
        }
    }

    pub fn image3D(format: wgpu::TextureFormat, access: wgpu::StorageTextureAccess) -> wgpu::BindingType {
        wgpu::BindingType::StorageTexture {
            access,
            view_dimension: wgpu::TextureViewDimension::D3,
            format: format,
        }
    }
}

pub mod wgsl{
    pub fn uniform() -> wgpu::BindingType{
        wgpu::BindingType::Buffer{
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    }

    pub fn sampler() -> wgpu::BindingType{
        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
    }

    pub fn texture_2d() -> wgpu::BindingType{
        wgpu::BindingType::Texture{
            sample_type: wgpu::TextureSampleType::Float{ filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        }
    }
}

use std::io::Cursor;

use crate::{
    error::AppResult,
    project::{
        Project, ShaderId, TextureId,
        texture::{Texture, TextureCreationContext, TextureSource},
    },
};

pub struct HdrLoader {
    texture_format: wgpu::TextureFormat,
    equirect_layout: wgpu::BindGroupLayout,
    equirect_to_cubemap: wgpu::ComputePipeline,
}

impl HdrLoader {
    pub fn new(
        device: &wgpu::Device,
        project: &Project,
        equirectangular_shader_id: ShaderId,
    ) -> AppResult<Self> {
        let shader = project.shaders.get(equirectangular_shader_id).unwrap();
        let module = shader.create_wgpu_shader_module(device)?;
        let texture_format = wgpu::TextureFormat::Rgba32Float;
        let equirect_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("HdrLoader::equirect_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: texture_format,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&equirect_layout)],
            immediate_size: 0,
        });

        let equirect_to_cubemap =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("equirect_to_cubemap"),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some("compute_equirect_to_cubemap"),
                compilation_options: Default::default(),
                cache: None,
            });

        Ok(Self {
            equirect_to_cubemap,
            texture_format,
            equirect_layout,
        })
    }

    pub fn from_equirectangular_bytes(
        &self,
        project: &mut Project,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        dst_size: u32,
    ) -> AppResult<TextureId> {
        let hdr_decoder = image::codecs::hdr::HdrDecoder::new(Cursor::new(data))?;
        let meta = hdr_decoder.metadata();

        #[cfg(not(target_arch = "wasm32"))]
        let pixels = {
            let mut pixels = vec![[0.0, 0.0, 0.0, 0.0]; meta.width as usize * meta.height as usize];
            hdr_decoder.read_image_transform(
                |pix| {
                    let rgb = pix.to_hdr();
                    [rgb.0[0], rgb.0[1], rgb.0[2], 1.0f32]
                },
                &mut pixels[..],
            )?;
            pixels
        };
        #[cfg(target_arch = "wasm32")]
        let pixels = hdr_decoder
            .read_image_native()?
            .into_iter()
            .map(|pix| {
                let rgb = pix.to_hdr();
                [rgb.0[0], rgb.0[1], rgb.0[2], 1.0f32]
            })
            .collect::<Vec<_>>();

        let size = wgpu::Extent3d {
            width: meta.width,
            height: meta.height,
            depth_or_array_layers: 1,
        };

        let src = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.texture_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let src_view = src.create_view(&wgpu::TextureViewDescriptor::default());

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &src,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bytemuck::cast_slice(&pixels),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.width * std::mem::size_of::<[f32; 4]>() as u32),
                rows_per_image: Some(size.height),
            },
            size,
        );

        let dst_texture = Texture::new(
            &TextureCreationContext {
                dimensions: &project.dimensions,
                device,
                queue,
            },
            "Sky Texture".to_string(),
            self.texture_format,
            wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            TextureSource::Manual {
                size: wgpu::Extent3d {
                    width: dst_size,
                    height: dst_size,
                    depth_or_array_layers: 6,
                },
            },
        )?;

        // we can create without adding to the project because this D2Array will be only used by the shader
        let dst_texture_view = dst_texture
            .inner()
            .create_view(&wgpu::TextureViewDescriptor {
                label: Some("compute shader destination texture view"),
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            });

        let dst_texture_id = project.textures.register(dst_texture);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute shader bind group"),
            layout: &self.equirect_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&src_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&dst_texture_view),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&Default::default());
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("compute shader pass"),
            timestamp_writes: None,
        });

        let num_workgroups = (dst_size + 15) / 16;
        pass.set_pipeline(&self.equirect_to_cubemap);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(num_workgroups, num_workgroups, 6);

        drop(pass);

        queue.submit([encoder.finish()]);

        Ok(dst_texture_id)
    }
}

// use crate::renpy_interop::*;

use std::sync::Arc;

use gltf_for_renpy_flatbuffer::{ImageNameArgs, RGBAColor};

#[derive(Clone, Debug)]
pub enum ImageData {
    ImageName,
    RGBA(Vec<u8>),
    Color([f32; 4]),
}

// Image data to be sent to the python code later
#[derive(Clone, Debug)]
pub struct RenpyImage {
    pub name: String,
    pub data: ImageData,
    pub width: u32,
    pub height: u32,
}

impl RenpyImage {
    pub fn load_image(
        raw_image: &Option<Arc<image::RgbaImage>>,
        raw_image_name: &Option<std::sync::Arc<String>>,
        factor: &Option<cgmath::Vector4<f32>>,
        use_embed_textures: bool,
    ) -> Self {
        let texture_name: String;
        let data: ImageData;
        let image_size: (u32, u32);

        if let Some(image) = &raw_image {
            let raw_name = (*(*raw_image_name).clone().unwrap_or_default()).clone();
            texture_name = raw_name;

            image_size = image.dimensions();

            if use_embed_textures {
                let texture: Vec<u8> = image.to_vec();
                // All factors are preapplied for embeded textures
                data = ImageData::RGBA(texture)
            } else {
                data = ImageData::ImageName;
                // We can't really apply factor to here tbh...
            }
        } else {
            image_size = (0, 0);

            if let Some(factor) = factor {
                data = ImageData::Color((*factor).into());
            } else {
                unimplemented!("Pretty sure this should not be hit but I am not sure tbh");
            }

            texture_name = String::new();
        }

        RenpyImage {
            name: texture_name,
            data,
            width: image_size.0,
            height: image_size.1,
        }
    }

    pub fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<super::flatbuffer::Image<'a>> {
        let name = builder.create_string(&self.name);

        let data_type;
        let data;

        match &self.data {
            ImageData::ImageName => {
                data_type = super::flatbuffer::ImageData::ImageName;
                data = Some(
                    super::flatbuffer::ImageName::create(builder, &ImageNameArgs {})
                        .as_union_value(),
                );
            }
            ImageData::RGBA(image_data) => {
                data_type = super::flatbuffer::ImageData::RGBA;
                let rgba_data = builder.create_vector(image_data);
                data = Some(
                    super::flatbuffer::RGBAImageData::create(
                        builder,
                        &super::flatbuffer::RGBAImageDataArgs {
                            data: Some(rgba_data),
                        },
                    )
                    .as_union_value(),
                );
            }
            ImageData::Color(color) => {
                data_type = super::flatbuffer::ImageData::Color;
                data = Some(
                    super::flatbuffer::RGBAColorData::create(
                        builder,
                        &gltf_for_renpy_flatbuffer::RGBAColorDataArgs {
                            data: Some(&RGBAColor::new(color)),
                        },
                    )
                    .as_union_value(),
                );
            }
        }

        super::flatbuffer::Image::create(
            builder,
            &super::flatbuffer::ImageArgs {
                name: Some(name),
                data_type,
                data,
                width: self.width,
                height: self.height,
            },
        )
    }
}

use crate::utils::GltfData;
use cgmath::*;
use image::{GrayImage, RgbaImage};
use std::sync::Arc;

#[derive(Clone, Debug)]
/// A set of parameter values that are used to define the metallic-roughness
/// material model from Physically-Based Rendering (PBR) methodology.
pub struct PbrMaterial {
    /// The `base_color_factor` contains scaling factors for the red, green,
    /// blue and alpha component of the color. If no texture is used, these
    /// values will define the color of the whole object in **RGB** color space.
    pub base_color_factor: Vector4<f32>,

    /// The `base_color_texture` is the main texture that will be applied to the
    /// object.
    ///
    /// The texture contains RGB(A) components in **sRGB** color space.
    pub base_color_texture: Option<Arc<RgbaImage>>,

    /// The name used for the base color texture
    pub base_color_texture_name: Option<Arc<String>>,

    /// Contains the metalness value
    pub metallic_texture: Option<Arc<GrayImage>>,

    /// The name used for the metallic_roughness texture
    pub metallic_roughness_texture_name: Option<Arc<String>>,

    /// `metallic_factor` is multiply to the `metallic_texture` value. If no
    /// texture is given, then the factor define the metalness for the whole
    /// object.
    pub metallic_factor: f32,

    /// Contains the roughness value
    pub roughness_texture: Option<Arc<GrayImage>>,

    /// `roughness_factor` is multiply to the `roughness_texture` value. If no
    /// texture is given, then the factor define the roughness for the whole
    /// object.
    pub roughness_factor: f32,
}

impl PbrMaterial {
    pub(crate) fn load(pbr: gltf::material::PbrMetallicRoughness, data: &mut GltfData) -> Self {
        let mut material = Self {
            base_color_factor: pbr.base_color_factor().into(),
            ..Default::default()
        };
        if let Some(texture) = pbr.base_color_texture() {
            let mut texture_data = data.load_base_color_image(&texture.texture());
            if let Some(pixels) = std::sync::Arc::get_mut(&mut texture_data) {
                let base_color_factor = pbr.base_color_factor();
                if base_color_factor.ne(&[1.0, 1.0, 1.0, 1.0]) {
                    let factor_slice = base_color_factor.as_slice();
                    for pixel_data in pixels.pixels_mut() {
                        let final_color: Vec<u8> = pixel_data
                            .0
                            .iter()
                            .zip(factor_slice.iter())
                            .map(|(a, b)| ((*a as f32) * b) as u8)
                            .collect();
                        pixel_data[0] = final_color[0];
                        pixel_data[1] = final_color[1];
                        pixel_data[2] = final_color[2];
                        pixel_data[3] = final_color[3];
                    }
                }
            }

            material.base_color_texture = Some(texture_data);
            material.base_color_texture_name = Some(Arc::new(
                texture
                    .texture()
                    .source()
                    .name()
                    .unwrap_or_default()
                    .to_owned(),
            ));
        }

        material.roughness_factor = pbr.roughness_factor();
        material.metallic_factor = pbr.metallic_factor();

        if let Some(texture) = pbr.metallic_roughness_texture() {
            if material.metallic_factor > 0. {
                material.metallic_texture = Some(data.load_gray_image(&texture.texture(), 2));
                material.base_color_texture_name = Some(Arc::new(
                    texture
                        .texture()
                        .source()
                        .name()
                        .unwrap_or_default()
                        .to_owned(),
                ));
            }

            if material.roughness_factor > 0. {
                material.roughness_texture = Some(data.load_gray_image(&texture.texture(), 1));
            }
            let texture_name = Arc::new(
                texture
                    .texture()
                    .source()
                    .name()
                    .unwrap_or_default()
                    .to_owned(),
            );
            material.metallic_roughness_texture_name = Some(texture_name);
        }

        material
    }
}

impl Default for PbrMaterial {
    fn default() -> Self {
        PbrMaterial {
            base_color_factor: Vector4::new(1., 1., 1., 1.),
            base_color_texture: None,
            base_color_texture_name: None,
            metallic_factor: 0.,
            metallic_texture: None,
            roughness_factor: 0.,
            roughness_texture: None,
            metallic_roughness_texture_name: None,
        }
    }
}

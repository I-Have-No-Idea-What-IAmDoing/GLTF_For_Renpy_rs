use std::ops::Neg;

use gltf_for_renpy_flatbuffer::{self as flatbuffer, MeshArgs};
use gltf_loader::model::{MorphTarget, Skeleton, VertexAttributeIterator};
use nohash_hasher::IntSet;

use crate::gltf_loader::utils::DecomposedTransform;
use crate::{AnimationSet, FlatbufferConversion, RenpyImage, SimpleFlatbufferConversion};

use super::{ObjectId, property::Properties};

#[derive(Clone, Debug)]
pub struct Mesh {
    pub name: String,

    pub id: ObjectId,

    pub vertexes: Vec<f32>,

    pub triangles: Vec<u32>,

    pub default_transform: DecomposedTransform,

    pub skeleton: Option<Skeleton>,
    pub bone_indexes: Vec<u16>,
    pub bone_weights: Vec<f32>,

    pub morph_targets: Vec<MorphTarget>,
    pub morph_weights: Vec<f32>,

    pub animations: Vec<AnimationSet>,

    pub uvs: Vec<f32>,

    pub texture: RenpyImage,

    pub properties: Properties,
}

impl Mesh {
    pub fn create(
        model: &gltf_loader::Model,
        scene_name: String,
        use_embed_textures: bool,
    ) -> super::GltfObject {
        let mut model_points: Vec<f32> = Vec::with_capacity(model.vertices_len().saturating_mul(3));
        let mut uvs: Vec<f32> = Vec::with_capacity(model.vertices_len().saturating_mul(2));

        let id = model.index();

        for vertex in model.vertices() {
            model_points.push(vertex.position.x);
            // We need to negate the y position as if we don't then the model will be upside down???
            model_points.push(vertex.position.y.neg());
            model_points.push(vertex.position.z);

            uvs.push(vertex.tex_coords.x);
            uvs.push(vertex.tex_coords.y);
        }

        let mut triangles: Vec<u32> = Vec::with_capacity(model.indices_len().saturating_mul(3));

        if let Some(indexes) = model.indices() {
            let index_iter = indexes.chunks_exact(3);

            for chunk in index_iter {
                triangles.extend_from_slice(chunk);
            }
        }

        let pbr_material = &model.material().pbr;

        let image = RenpyImage::load_image(
            &pbr_material.base_color_texture,
            &pbr_material.base_color_texture_name,
            &Some(pbr_material.base_color_factor),
            use_embed_textures,
        );

        let name = format!(
            "{}:{}:{}",
            scene_name,
            model.mesh_name().unwrap_or("Model"),
            model.primitive_index()
        );

        let default_transform = model.transform().to_owned().to_renpy_coords(false);

        let animations: Vec<AnimationSet> = AnimationSet::from_node(model.animations());

        let morph_targets: Vec<MorphTarget> = model.morph_targets().clone();
        let morph_weights: Vec<f32> = model.morph_weights().clone();

        let skeleton: Option<Skeleton> = model.skeleton().clone();
        let bone_indexes = model.bone_indexes().clone();
        let bone_weights = model.bone_weights().clone();

        let mesh = Mesh {
            name,
            id,
            vertexes: model_points,
            morph_targets,
            morph_weights,
            skeleton,
            triangles,
            default_transform,
            animations,
            uvs,
            texture: image,
            properties: Vec::new(),
            bone_indexes,
            bone_weights,
        };

        let mut associated_object_ids: IntSet<ObjectId> = IntSet::default();
        associated_object_ids.insert(mesh.id);

        super::GltfObject::Mesh(associated_object_ids, Box::new(mesh))
    }

    pub fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<flatbuffer::Mesh<'a>> {
        let name = builder.create_string(&self.name);

        let animations = if self.animations.is_empty() {
            Some(builder.create_vector::<flatbuffers::WIPOffset<_>>(&[]))
        } else {
            let animation_buffer: Vec<_> = self
                .animations
                .iter()
                .map(|set| set.to_flatbuffer(builder))
                .collect();
            Some(builder.create_vector(&animation_buffer))
        };

        let properties: Vec<_> = self
            .properties
            .iter()
            .map(|props| props.to_flatbuffer(builder))
            .collect();

        let properties = builder.create_vector(&properties);

        let points = builder.create_vector(&self.vertexes);
        let triangles = builder.create_vector(&self.triangles);

        let (has_uvs, uvs) = if self.uvs.is_empty() {
            (false, None)
        } else {
            (true, Some(builder.create_vector(&self.uvs)))
        };

        let texture = self.texture.to_flatbuffer(builder);

        let default_morph_weights = if self.morph_weights.is_empty() {
            None
        } else {
            Some(builder.create_vector(&self.morph_weights))
        };

        let (target_count, morph_targets) = if self.morph_targets.is_empty() {
            (0_u8, None)
        } else {
            let morph_buffer: Vec<_> = self
                .morph_targets
                .iter()
                .map(|set| set.to_flatbuffer(builder))
                .collect();

            #[allow(clippy::cast_possible_truncation)]
            (
                // There will not be more than 255 morph targets lmao
                self.morph_targets.len().min(u8::MAX.into()) as u8,
                Some(builder.create_vector(&morph_buffer)),
            )
        };

        let skeleton = self
            .skeleton
            .as_ref()
            .map(|skeleton| skeleton.to_flatbuffer(builder));

        let layout = flatbuffer::LayoutType::new(
            has_uvs,
            target_count,
            !self.bone_indexes.is_empty() && !self.bone_weights.is_empty(),
        );

        // This may be a mess, but it is my contained mess...
        // Everything here is the best way to use my limited rust knowledge to automate this tedious task
        let attributes = {
            let mut uv_iter: VertexAttributeIterator<_, 2> =
                VertexAttributeIterator::new(self.uvs.iter());
            let target_iter = self
                .morph_targets
                .iter()
                .flat_map(|set| &set.blend_shapes)
                .flat_map(|shape| shape.as_attribute_slice());
            let mut target_iter: VertexAttributeIterator<_, 3> =
                VertexAttributeIterator::new(target_iter);

            let mut bone_index_iter: VertexAttributeIterator<_, 4> =
                VertexAttributeIterator::new(self.bone_indexes.iter());

            let mut bone_weights_iter: VertexAttributeIterator<_, 4> =
                VertexAttributeIterator::new(self.bone_weights.iter());

            const ATTRIBUTE_STRIDE: usize = 3_usize
                .saturating_add(2)
                .saturating_add(4)
                .saturating_add(4);
            let mut attributes_vec: Vec<f32> =
                Vec::with_capacity(self.vertexes.len().saturating_mul(ATTRIBUTE_STRIDE));

            for _ in 0..self.vertexes.len() {
                let uvs = uv_iter.get_attributes();
                for uv in uvs {
                    if let Some(uv) = *uv {
                        attributes_vec.push(*uv);
                    }
                }

                let targets = target_iter.get_attributes();
                for target in targets.iter().flatten() {
                    attributes_vec.push(*target);
                }

                let bones = bone_index_iter.get_attributes();
                for bone in bones {
                    if let Some(bone) = *bone {
                        attributes_vec.push(*bone as f32);
                    }
                }

                let weights = bone_weights_iter.get_attributes();
                for weight in weights {
                    if let Some(weight) = *weight {
                        attributes_vec.push(*weight);
                    }
                }
            }

            Some(builder.create_vector(&attributes_vec))
        };

        flatbuffer::Mesh::create(
            builder,
            &MeshArgs {
                name: Some(name),
                id: self.id as u64,
                transform: Some(&self.default_transform.to_flatbuffer()),
                points: Some(points),
                morph_targets,
                default_morph_weights,
                triangles: Some(triangles),
                animations,
                skeleton,
                uvs,
                layout_type: Some(&layout),
                attributes,
                texture: Some(texture),
                properties: Some(properties),
            },
        )
    }
}

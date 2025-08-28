use gltf_for_renpy_flatbuffer as flatbuffer;

use gltf_loader::utils::DecomposedTransform;
use nohash_hasher::IntSet;

use crate::{AnimationSet, FlatbufferConversion, SimpleFlatbufferConversion};

use super::{
    ObjectId,
    property::{Properties, Property},
};

/// A Point Object with no Model
/// Basically a way to describe a space in 3D along with scale and rotation
#[derive(Clone, Debug)]
pub struct Empty {
    pub id: usize,

    pub name: String,

    pub transform: DecomposedTransform,

    pub animations: Vec<AnimationSet>,

    pub properties: Properties,
}

impl Empty {
    pub fn create(empty: &gltf_loader::Empty, scene_name: String) -> super::GltfObject {
        let properties = Property::load(empty.extras.clone());

        let transform = empty.transform().clone().to_renpy_coords(false);

        let name = format!(
            "{}:{}",
            scene_name,
            empty.name.clone().unwrap_or("Empty".to_owned())
        );

        let animations = AnimationSet::from_node(empty.animations());

        // X has to be negated because in gltf, +X is left, while in renpy, +X is right.
        // I am not sure why I have to negate z...
        let loaded_empty = Empty {
            id: empty.id,
            name,
            transform,
            animations,
            properties,
        };

        let mut associated_object_ids: IntSet<ObjectId> = IntSet::default();
        associated_object_ids.insert(empty.id);

        super::GltfObject::Empty(associated_object_ids, Box::new(loaded_empty))
    }

    pub fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<flatbuffer::Empties<'a>> {
        // The return type is dictated by the flatbuffer schema and can remain.
        let name = builder.create_string(&self.name);

        let animation_offsets: Vec<_> = self
            .animations
            .iter()
            .map(|set| set.to_flatbuffer(builder))
            .collect();

        let animations = Some(builder.create_vector(&animation_offsets));

        let properties: Vec<_> = self
            .properties
            .iter()
            .map(|props| props.to_flatbuffer(builder))
            .collect();
        let properties = builder.create_vector(&properties);

        flatbuffer::Empties::create(
            builder,
            &flatbuffer::EmptiesArgs {
                id: self.id as u64,
                name: Some(name),
                transform: Some(&self.transform.to_flatbuffer()),
                animations,
                properties: Some(properties),
            },
        )
    }
}

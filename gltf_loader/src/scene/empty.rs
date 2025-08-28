use crate::{
    GLTFAnimation, get_extras,
    utils::{DecomposedTransform, GltfData, convert_extra},
};
use cgmath::*;
use gltf::scene::Node;
use std::collections::HashMap;

/// An Empty Node
#[derive(Clone, Debug, Default)]
pub struct Empty {
    /// Camera name. Requires the `names` feature.
    pub name: Option<String>,

    /// Json Index
    pub id: usize,

    /// Scene extra data. Requires the `extras` feature.
    pub extras: Option<HashMap<String, String>>,

    pub(crate) parent_nodes: Vec<usize>,

    // The default transform
    pub(crate) static_transform: DecomposedTransform,

    pub(crate) animations: Vec<GLTFAnimation>,
}

impl Empty {
    pub(crate) fn load(node: &Node, parents: Vec<usize>, data: &mut GltfData) -> Self {
        let transform = DecomposedTransform::convert_from_gltf(node.transform());

        let animations = data.animations.remove(&node.index()).unwrap_or_default();

        let mut empty = Self {
            static_transform: transform,
            parent_nodes: parents,
            animations,
            ..Default::default()
        };

        empty.id = node.index();

        empty.name = node.name().map(String::from);

        empty.extras = get_extras!(node);

        empty
    }

    /// Returns position of the origin of the empty    
    pub fn position(&self) -> cgmath::Vector3<f32> {
        self.static_transform.translation
    }

    /// Returns the transform of the empty    
    pub fn transform(&self) -> &DecomposedTransform {
        &self.static_transform
    }

    /// Returns the parent of the object
    pub fn parents(self) -> Vec<usize> {
        self.parent_nodes
    }

    /// Returns position of the origin of the empty    
    pub fn rotation(&self) -> cgmath::Euler<Deg<f32>> {
        match self.static_transform.rotation {
            crate::utils::RotationTransform::Quaternion(quaternion) => {
                let raw_euler = cgmath::Euler::<cgmath::Rad<f32>>::from(quaternion);
                cgmath::Euler::<Deg<f32>>::new(
                    cgmath::Deg::<f32>::from(raw_euler.x),
                    cgmath::Deg::<f32>::from(raw_euler.y),
                    cgmath::Deg::<f32>::from(raw_euler.z),
                )
            }
            crate::utils::RotationTransform::Euler(euler) => euler,
        }
    }

    /// Animations associated with this model
    pub fn animations(&self) -> &Vec<GLTFAnimation> {
        &self.animations
    }

    /// Animations associated with this model
    pub fn animations_mut(&mut self) -> &mut Vec<GLTFAnimation> {
        &mut self.animations
    }
}

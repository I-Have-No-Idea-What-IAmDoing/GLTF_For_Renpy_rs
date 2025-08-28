use crate::get_extras;
use crate::utils::convert_extra;
use crate::utils::{GlobalNodeIdentifier, GltfData};
use std::collections::HashMap;

use super::Vertex;

const BIND_CONVERSION_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
);

/// Skeleton from a GLTF skin
#[derive(Clone, Debug)]
pub struct Skeleton {
    /// The internal index from the json file
    pub id: usize,

    /// User-provided name
    pub name: String,

    /// The root of the skeleton tree
    pub root_index: GlobalNodeIdentifier,

    /// Matrixes used to bring coordinates being skinned into the same space as the joint
    pub inverse_bind_matrixes: Vec<cgmath::Matrix4<f32>>,

    /// List of bones assoicated with the skeleton
    pub bones: Vec<GlobalNodeIdentifier>,

    /// Extra user data
    pub extras: HashMap<String, String>,
}

impl Skeleton {
    pub(crate) fn load(skin: &gltf::Skin, data: &GltfData) -> (usize, Skeleton) {
        let id = skin.index();
        let name = match skin.name() {
            Some(name) => name.to_string(),
            None => format!("Skeleton {id}"),
        };

        let buffers = &data.buffers;
        let reader = skin.reader(|buffer| Some(&buffers[buffer.index()]));

        let mut bind_matrixes = Vec::new();
        if let Some(mats) = reader.read_inverse_bind_matrices() {
            for mat in mats {
                let mat = cgmath::Matrix4::from(mat);

                bind_matrixes.push(BIND_CONVERSION_MATRIX * mat * BIND_CONVERSION_MATRIX);
            }
        }

        let mut joints = Vec::new();
        for joint in skin.joints() {
            let id = joint.index();

            joints.push(GlobalNodeIdentifier::NodeId(id));
        }

        let root_index;
        if let Some(root_node) = skin.skeleton() {
            let id = root_node.index();

            let index = joints.iter().position(|bone| match bone {
                GlobalNodeIdentifier::SceneRoot => false,
                GlobalNodeIdentifier::NodeId(bone) => *bone == id,
                GlobalNodeIdentifier::ObjectIndex(_) => false,
            });

            if let Some(index) = index {
                root_index = GlobalNodeIdentifier::ObjectIndex(index);
            } else {
                root_index = GlobalNodeIdentifier::NodeId(id);
            }
        } else {
            root_index = GlobalNodeIdentifier::SceneRoot;
        }

        // Load extras (Copied from extra loading from model loader)
        let extras: HashMap<String, String> = get_extras!(skin).unwrap_or_default();

        (
            id,
            Skeleton {
                name,
                id,
                root_index,
                bones: joints,
                inverse_bind_matrixes: bind_matrixes,
                extras,
            },
        )
    }
}

/// Morph Targets
#[derive(Clone, Debug)]
pub struct MorphTarget {
    /// User-provided name
    pub name: String,

    /// Each attributes correspond to a vertex that is modified
    pub blend_shapes: Vec<Vertex>,
}

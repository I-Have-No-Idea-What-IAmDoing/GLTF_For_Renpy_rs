mod camera;
mod empty;
mod light;

/// Contains model and material
/// # Usage
/// Check [Model](struct.Model.html) for more information about how to use this module.
pub mod model;

use std::collections::HashMap;
use std::fmt::Display;

use crate::utils::{DecomposedTransform, convert_extra};
use crate::{GltfData, get_extras};
pub use camera::{Camera, Projection};
use ego_tree::Tree;
pub use empty::Empty;
pub use light::Light;
pub use model::{Material, Model};

use gltf::scene::Node;

/// Contains cameras, models and lights of a scene.
#[derive(Clone, Debug)]
pub struct Scene {
    /// Scene name. Requires the `names` feature.
    pub name: Option<String>,
    /// Scene extra data. Requires the `extras` feature.
    pub extras: Option<HashMap<String, String>>,
    /// List of models in the scene
    // pub models: Vec<Model>,
    // /// List of cameras in the scene
    // pub cameras: Vec<Camera>,
    // /// List of lights in the scene
    // pub lights: Vec<Light>,
    /// List of empty nodes that contain data
    // pub empties: Vec<Empty>,
    pub objects: Tree<SceneObject>,
}

impl Default for Scene {
    fn default() -> Self {
        Scene {
            name: Default::default(),
            objects: Tree::new(SceneObject::Root),
            extras: Default::default(),
        }
    }
}

/// Object types in the scene tree
#[derive(Clone, Debug)]
pub enum SceneObject {
    /// Root Node
    Root,
    /// Node that contains a object
    Mesh(Box<Model>),
    /// Node that contains an empty
    Empties(Box<Empty>),
}

impl Display for SceneObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneObject::Root => f.write_str("Root"),
            SceneObject::Mesh(model) => {
                f.write_fmt(format_args!("{}[Mesh]", model.mesh_name.clone().unwrap()))
            }

            SceneObject::Empties(empty) => {
                f.write_fmt(format_args!("{}[Mesh]", empty.name.clone().unwrap()))
            }
        }
    }
}

impl Scene {
    pub(crate) fn load(gltf_scene: gltf::Scene, data: &mut GltfData) -> Self {
        let mut scene = Self {
            name: gltf_scene.name().map(String::from),
            ..Default::default()
        };

        scene.extras = get_extras!(gltf_scene);

        let root_node = scene.objects.root().id();

        for node in gltf_scene.nodes() {
            scene.read_node(root_node, &node, Vec::new(), data);
        }

        scene
    }

    fn read_node(
        &mut self,
        tree_node: ego_tree::NodeId,
        gltf_node: &Node,
        mut parents: Vec<usize>,
        data: &mut GltfData,
    ) {
        let mut tree_node = self.objects.get_mut(tree_node).unwrap();

        let transform = DecomposedTransform::convert_from_gltf(gltf_node.transform());

        let mut loaded_attribute: u8 = 0;

        // // Load camera
        if let Some(_camera) = gltf_node.camera() {
            // self.cameras.push(Camera::load(camera, &transform));
            loaded_attribute += 1;
        }

        // // Load light
        if let Some(_light) = gltf_node.light() {
            // self.lights.push(Light::load(light, &transform));
            loaded_attribute += 1;
        }

        // Load model
        if let Some(mesh) = gltf_node.mesh() {
            for (i, primitive) in mesh.primitives().enumerate() {
                tree_node.append(SceneObject::Mesh(Box::new(Model::load(
                    gltf_node,
                    &mesh,
                    i,
                    primitive,
                    parents.clone(),
                    &transform,
                    data,
                ))));
            }
            loaded_attribute += 1;
        }

        if loaded_attribute == 0 {
            tree_node.append(SceneObject::Empties(Box::new(Empty::load(
                gltf_node,
                parents.clone(),
                data,
            ))));
        }

        let current_node = if let Some(node) = tree_node.last_child() {
            node.id()
        } else {
            tree_node.id()
        };

        // Recurse on children
        for child in gltf_node.children() {
            parents.push(gltf_node.index());
            let parents = parents.clone();
            self.read_node(current_node, &child, parents, data);
        }
    }
}

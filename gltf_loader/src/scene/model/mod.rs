mod material;
mod mode;
mod skin;
mod vertex;

use crate::{GLTFAnimation, get_extras, utils::*};
use cgmath::*;
use std::{collections::HashMap, sync::Arc};

pub use material::*;
pub use mode::*;
pub use skin::*;
pub use vertex::*;

/// Geometry to be rendered with the given material.
///
/// # Examples
///
/// ### Classic rendering
///
/// In most cases you want to use `triangles()`, `lines()` and `points()`
/// to get the geometry of the model.
///
/// ```
/// # use gltf_loader::*;
/// # use gltf_loader::model::Mode;
/// # let model = Model::default();
/// match model.mode() {
///   Mode::Triangles | Mode::TriangleFan | Mode::TriangleStrip => {
///     let triangles = model.triangles().unwrap();
///     // Render triangles...
///   },
///   Mode::Lines | Mode::LineLoop | Mode::LineStrip => {
///     let lines = model.lines().unwrap();
///     // Render lines...
///   }
///   Mode::Points => {
///     let points = model.points().unwrap();
///     // Render points...
///   }
/// }
/// ```
///
/// ### OpenGL style rendering
///
/// You will need the vertices and the indices if existing.
///
/// ```
/// # use gltf_loader::*;
/// # use gltf_loader::model::Mode;
/// # let model = Model::default();
/// let vertices = model. vertices();
/// let indices = model.indices();
/// match model.mode() {
///   Mode::Triangles => {
///     if let Some(indices) = indices.as_ref() {
///       // glDrawElements(GL_TRIANGLES, indices.len(), GL_UNSIGNED_INT, 0);
///     } else {
///       // glDrawArrays(GL_TRIANGLES, 0, vertices.len());
///     }
///   },
///   // ...
/// # _ => unimplemented!(),
/// }
/// ```
#[derive(Clone, Debug, Default)]
pub struct Model {
    pub(crate) mesh_name: Option<String>,
    pub(crate) mesh_extras: Option<HashMap<String, String>>,
    pub(crate) primitive_extras: Option<HashMap<String, String>>,

    pub(crate) index: usize,
    pub(crate) primitive_index: usize,

    pub(crate) vertices: Vec<Vertex>,
    pub(crate) indices: Option<Vec<u32>>,

    pub(crate) morph_targets: Vec<MorphTarget>,
    pub(crate) mode: Mode,
    pub(crate) parent_nodes: Vec<usize>,

    // The default transform
    pub(crate) static_transform: DecomposedTransform,
    pub(crate) default_weights: Vec<f32>,

    pub(crate) skeleton: Option<Skeleton>,
    pub(crate) bone_indexes: Vec<u16>,
    pub(crate) bone_weights: Vec<f32>,

    pub(crate) material: Arc<Material>,
    pub(crate) animations: Vec<GLTFAnimation>,

    pub(crate) has_normals: bool,
    pub(crate) has_tangents: bool,
    pub(crate) has_tex_coords: bool,
}

impl Model {
    /// Mesh name. Requires the `names` feature.
    ///
    /// A `Model` in easy-gltf represents a primitive in gltf, so if a mesh has multiple primitives, you will
    /// get multiple `Model`s with the same name. You can use `primitive_index` to get which primitive the `Model`
    /// corresponds to.
    pub fn mesh_name(&self) -> Option<&str> {
        self.mesh_name.as_deref()
    }

    /// Index of the Primitive of the Mesh that this `Model` corresponds to.
    pub fn primitive_index(&self) -> usize {
        self.primitive_index
    }

    /// Index of the Node that this `Model` corresponds to.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns the ids of the parents of the object
    pub fn parents(&self) -> &Vec<usize> {
        &self.parent_nodes
    }

    /// Mesh extra data. Requires the `extras` feature.
    pub fn mesh_extras(&self) -> &Option<HashMap<String, String>> {
        &self.mesh_extras
    }

    /// Primitive extra data. Requires the `extras` feature.
    pub fn primitive_extras(&self) -> &Option<HashMap<String, String>> {
        &self.primitive_extras
    }

    /// Material to apply to the whole model.
    pub fn material(&self) -> Arc<Material> {
        self.material.clone()
    }

    /// List of raw `vertices` of the model. You might have to use the `indices`
    /// to render the model.
    ///
    /// **Note**: If you're not rendering with **OpenGL** you probably want to use
    /// `triangles()`, `lines()` or `points()` instead.
    pub fn vertices(&self) -> &Vec<Vertex> {
        &self.vertices
    }

    /// Length of raw `vertices` list of the model
    pub fn vertices_len(&self) -> usize {
        self.vertices.len()
    }

    /// Potential list of `indices` to render the model using raw `vertices`.
    ///
    /// **Note**: If you're **not** rendering with **OpenGL** you probably want to use
    /// `triangles()`, `lines()` or `points()` instead.
    pub fn indices(&self) -> Option<&Vec<u32>> {
        self.indices.as_ref()
    }

    /// The length of the list of `indices` to render the model using raw `vertices`.
    /// If model drawn with XYZ then the result is 0
    pub fn indices_len(&self) -> usize {
        match &self.indices {
            Some(index_vec) => index_vec.len(),
            None => 0,
        }
    }

    /// The type of primitive to render.
    /// You have to check the `mode` to render the model correctly.
    ///
    /// Then you can either use:
    /// * `vertices()` and `indices()` to arrange the data yourself (useful for **OpenGL**).
    /// * `triangles()` or `lines()` or `points()` according to the returned mode.
    pub fn mode(&self) -> Mode {
        self.mode.clone()
    }

    /// List of triangles ready to be rendered.
    ///
    /// **Note**: This function will return an error if the mode isn't `Triangles`, `TriangleFan`
    /// or `TriangleStrip`.
    pub fn triangles(&self) -> Result<Vec<Triangle>, BadMode> {
        let mut triangles = vec![];
        let indices = (0..self.vertices.len() as u32).collect();
        let indices = self.indices().unwrap_or(&indices);

        match self.mode {
            Mode::Triangles => {
                for i in (0..indices.len()).step_by(3) {
                    triangles.push([
                        self.vertices[indices[i] as usize],
                        self.vertices[indices[i + 1] as usize],
                        self.vertices[indices[i + 2] as usize],
                    ]);
                }
            }
            Mode::TriangleStrip => {
                for i in 0..(indices.len() - 2) {
                    triangles.push([
                        self.vertices[indices[i] as usize + i % 2],
                        self.vertices[indices[i + 1 - i % 2] as usize],
                        self.vertices[indices[i + 2] as usize],
                    ]);
                }
            }
            Mode::TriangleFan => {
                for i in 1..(indices.len() - 1) {
                    triangles.push([
                        self.vertices[indices[0] as usize],
                        self.vertices[indices[i] as usize],
                        self.vertices[indices[i + 1] as usize],
                    ]);
                }
            }
            _ => return Err(BadMode { mode: self.mode() }),
        }
        Ok(triangles)
    }

    /// List of lines ready to be rendered.
    ///
    /// **Note**: This function will return an error if the mode isn't `Lines`, `LineLoop`
    /// or `LineStrip`.
    pub fn lines(&self) -> Result<Vec<Line>, BadMode> {
        let mut lines = vec![];
        let indices = (0..self.vertices.len() as u32).collect();
        let indices = self.indices().unwrap_or(&indices);
        match self.mode {
            Mode::Lines => {
                for i in (0..indices.len()).step_by(2) {
                    lines.push([
                        self.vertices[indices[i] as usize],
                        self.vertices[indices[i + 1] as usize],
                    ]);
                }
            }
            Mode::LineStrip | Mode::LineLoop => {
                for i in 0..(indices.len() - 1) {
                    lines.push([
                        self.vertices[indices[i] as usize],
                        self.vertices[indices[i + 1] as usize],
                    ]);
                }
            }
            _ => return Err(BadMode { mode: self.mode() }),
        }
        if self.mode == Mode::LineLoop {
            lines.push([
                self.vertices[indices[0] as usize],
                self.vertices[indices[indices.len() - 1] as usize],
            ]);
        }

        Ok(lines)
    }

    /// List of points ready to be renderer.
    ///
    /// **Note**: This function will return an error if the mode isn't `Points`.
    pub fn points(&self) -> Result<&Vec<Vertex>, BadMode> {
        match self.mode {
            Mode::Points => Ok(&self.vertices),
            _ => Err(BadMode { mode: self.mode() }),
        }
    }

    /// The initial transform applied to the vertices
    pub fn transform(&self) -> &DecomposedTransform {
        &self.static_transform
    }

    /// Animations associated with this model
    pub fn animations(&self) -> &Vec<GLTFAnimation> {
        &self.animations
    }

    /// Animations associated with this model
    pub fn animations_mut(&mut self) -> &mut Vec<GLTFAnimation> {
        &mut self.animations
    }

    /// Indicate if the vertices contains normal information.
    ///
    /// **Note**: If this function return `false` all vertices has a normal field
    /// initialized to `zero`.
    pub fn has_normals(&self) -> bool {
        self.has_normals
    }

    /// Indicate if the vertices contains tangents information.
    ///
    /// **Note**: If this function return `false` all vertices has a tangent field
    /// initialized to `zero`.
    pub fn has_tangents(&self) -> bool {
        self.has_tangents
    }

    /// Indicate if the vertices contains texture coordinates information.
    ///
    /// **Note**: If this function return `false` all vertices has a tex_coord field
    /// initialized to `zero`.
    pub fn has_tex_coords(&self) -> bool {
        self.has_tex_coords
    }

    /// List of final morph target values, they are ordered in the same way as the vertices
    pub fn morph_targets(&self) -> &Vec<MorphTarget> {
        &self.morph_targets
    }

    /// List of weights to use by default for morph targets
    pub fn morph_weights(&self) -> &Vec<f32> {
        &self.default_weights
    }

    /// The skin associated with the model
    pub fn skeleton(&self) -> &Option<Skeleton> {
        &self.skeleton
    }

    /// The skin associated with the model
    pub fn bone_indexes(&self) -> &Vec<u16> {
        &self.bone_indexes
    }

    /// The skin associated with the model
    pub fn bone_weights(&self) -> &Vec<f32> {
        &self.bone_weights
    }
    // fn apply_transform_position(pos: [f32; 3], transform: &Matrix4<f32>) -> Vector3<f32> {
    //     let pos = Vector4::new(pos[0], pos[1], pos[2], 1.);
    //     let res = transform * pos;
    //     Vector3::new(res.x / res.w, res.y / res.w, res.z / res.w)
    // }

    // fn apply_transform_vector(vec: [f32; 3], transform: &Matrix4<f32>) -> Vector3<f32> {
    //     let vec = Vector4::new(vec[0], vec[1], vec[2], 0.);
    //     (transform * vec).truncate()
    // }

    // fn apply_transform_tangent(tangent: [f32; 4], transform: &Matrix4<f32>) -> Vector4<f32> {
    //     let tang = Vector4::new(tangent[0], tangent[1], tangent[2], 0.);
    //     let mut tang = transform * tang;
    //     tang[3] = tangent[3];
    //     tang
    // }

    pub(crate) fn load(
        node: &gltf::Node,
        mesh: &gltf::Mesh,
        primitive_index: usize,
        primitive: gltf::Primitive,
        parents: Vec<usize>,
        decomposed_transform: &DecomposedTransform,
        data: &mut GltfData,
    ) -> Self {
        let buffers = &data.buffers;
        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

        let indices = reader
            .read_indices()
            .map(|indices| indices.into_u32().collect());

        // Init vertices with the position
        let mut vertices: Vec<_> = reader
            .read_positions()
            .unwrap_or_else(|| panic!("The model primitive doesn't contain positions"))
            .map(|pos| Vertex {
                position: Vector3::new(pos[0], pos[1], pos[2]),
                ..Default::default()
            })
            .collect();

        // Fill normals
        let has_normals = if let Some(normals) = reader.read_normals() {
            for (i, normal) in normals.enumerate() {
                vertices[i].normal = normal.into();
            }
            true
        } else {
            false
        };

        // Fill tangents
        let has_tangents = if let Some(tangents) = reader.read_tangents() {
            for (i, tangent) in tangents.enumerate() {
                vertices[i].tangent = tangent.into();
            }
            true
        } else {
            false
        };

        // Texture coordinates
        let has_tex_coords = if let Some(tex_coords) = reader.read_tex_coords(0) {
            for (i, tex_coords) in tex_coords.into_f32().enumerate() {
                vertices[i].tex_coords = Vector2::from(tex_coords);
            }
            true
        } else {
            false
        };

        let mesh_extras: Option<HashMap<String, String>> = get_extras!(mesh);

        let primitive_extras: Option<HashMap<String, String>> = get_extras!(primitive);

        let animations = data.animations.remove(&node.index()).unwrap_or_default();

        let mut morph_targets = Vec::new();
        let mut target_names: Vec<String> = Vec::new();

        // Ugly ass code to get the name of morph targets if it exists
        if let Some(x) = &mesh_extras
            && let Some(name_array) = x.get("targetNames")
            && let Ok(gltf::json::Value::Array(target_name)) =
                gltf::json::deserialize::from_str::<gltf::json::Value>(name_array)
        {
            target_names.extend(target_name.iter().map(|x| {
                if let Some(name) = x.as_str() {
                    name.to_string()
                } else {
                    String::new()
                }
            }));
        }

        for (index, (position, _normal, _tangent)) in reader.read_morph_targets().enumerate() {
            let mut blend_shapes = Vec::new();

            if let Some(position) = position {
                blend_shapes.extend(position.map(|pos| Vertex {
                    position: Vector3::from(pos),
                    ..Default::default()
                }));
            }

            let name = if let Some(name) = target_names.get(index) {
                name.clone()
            } else {
                format!("Key {index}").to_string()
            };

            morph_targets.push(MorphTarget { name, blend_shapes });
        }

        let mut bone_indexes = Vec::with_capacity(vertices.len() * 4);
        let mut bone_weights = Vec::with_capacity(vertices.len() * 4);

        if let Some(joint_sets) = reader.read_joints(0) {
            for joint_index in joint_sets.into_u16() {
                bone_indexes.extend_from_slice(&joint_index);
            }
        }

        if let Some(joint_sets) = reader.read_weights(0) {
            for weights in joint_sets.into_f32() {
                bone_weights.extend_from_slice(&weights);
            }
        }

        let default_weights = if let Some(weight_slice) = node.weights() {
            let mut weights = weight_slice.to_vec();
            weights.resize(4, 0.0);
            weights
        } else if let Some(mesh_weight_slice) = mesh.weights() {
            let mut weights = mesh_weight_slice.to_vec();
            weights.resize(4, 0.0);
            weights
        } else {
            vec![0.0; 4]
        };

        let skeleton = if let Some(skin) = node.skin() {
            if let Some(skin) = data.skeletons.get(&skin.index()) {
                Some(skin.to_owned())
            } else {
                eprintln!("Error: Skeleton not in data struct ({})", skin.index());
                None
            }
        } else {
            None
        };

        Model {
            mesh_name: mesh.name().map(String::from),
            mesh_extras,
            primitive_extras,
            index: node.index(),
            primitive_index,
            vertices,
            indices,
            parent_nodes: parents,
            static_transform: decomposed_transform.clone(),
            morph_targets,
            default_weights,
            material: Material::load(primitive.material(), data),
            animations,
            mode: primitive.mode().into(),
            has_normals,
            has_tangents,
            has_tex_coords,
            skeleton,
            bone_indexes,
            bone_weights,
        }
    }
}

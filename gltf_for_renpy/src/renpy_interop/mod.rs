// Most of this can be refactored once I figure out how to make this a python c module

use std::{
    ffi::{CString, c_char},
    fmt::{Debug, Display},
    mem,
    ops::{Deref, DerefMut},
    ptr::{null_mut, slice_from_raw_parts},
};

use cgmath::{Matrix, Matrix4};
use gltf_loader::utils::RotationTransform;

use crate::gltf_objects;

// This is generic container for return data that needs to be sent to python
#[repr(C)]
#[derive(Clone, Debug)]
pub struct GLTFResult<T> {
    pub result_type: super::ResultCode,
    pub error_description: PyString,
    // The actual data is just a pointer to content
    pub content: Nullable<T>,
}

impl<T> GLTFResult<T> {
    pub fn new(result_type: super::ResultCode, description: String, content: T) -> *const Self {
        Box::into_raw(Box::new(GLTFResult {
            result_type,
            error_description: description.into(),
            content: Nullable::new(content),
        }))
    }

    pub fn ok(content: T) -> *const Self {
        Box::into_raw(Box::new(GLTFResult {
            result_type: super::ResultCode::Ok,
            error_description: PyString::empty(),
            content: Nullable::new(content),
        }))
    }

    pub fn error(result_type: super::ResultCode, description: String) -> *const Self {
        Box::into_raw(Box::new(GLTFResult {
            result_type,
            error_description: description.into(),
            content: Nullable::null(),
        }))
    }

    pub fn is_ok(&self) -> bool {
        self.result_type == super::ResultCode::Ok
    }
}

// Just a wrapper over a pointer to get rid of some tedium on the rust side
#[repr(transparent)]
#[derive(Clone)]
pub struct Nullable<T>(pub *mut T);

impl<T> Nullable<T> {
    pub fn new(obj: T) -> Self {
        Nullable(Box::into_raw(Box::new(obj)))
    }

    pub fn null() -> Self {
        Nullable(null_mut())
    }

    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl<T> Deref for Nullable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        #[allow(clippy::unwrap_used)]
        unsafe {
            self.0.as_ref().unwrap()
        }
    }
}

impl<T> DerefMut for Nullable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        #[allow(clippy::unwrap_used)]
        unsafe {
            self.0.as_mut().unwrap()
        }
    }
}

impl<T> Drop for Nullable<T> {
    fn drop(&mut self) {
        if self.0.is_null() {
            return;
        }

        unsafe {
            drop(Box::from_raw(self.0));
        }
    }
}

impl<T> Default for Nullable<T>
where
    T: Default,
{
    fn default() -> Self {
        Nullable::new(T::default())
    }
}

impl<T> Debug for Nullable<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

// Just an array that with some quality of life features for sending over to python
#[repr(C)]
#[derive(Debug)]
pub struct ImmutableRenpyList<T> {
    pub content: *const T,
    pub len: usize,
}

impl<T> Clone for ImmutableRenpyList<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let mut return_value: Vec<T> = Vec::with_capacity(self.len);
        unsafe {
            let old_list = slice_from_raw_parts(self.content, self.len)
                .as_ref()
                .unwrap_or_default();

            return_value.clone_from_slice(old_list);
        }

        ImmutableRenpyList::from(return_value)
    }
}

impl<T> ImmutableRenpyList<T> {
    pub fn new(content: *const T, len: usize) -> *const ImmutableRenpyList<T> {
        Box::into_raw(Box::new(ImmutableRenpyList { content, len }))
    }

    pub fn empty() -> ImmutableRenpyList<T> {
        ImmutableRenpyList {
            content: std::ptr::null(),
            len: 0,
        }
    }

    pub fn from(mut list: Vec<T>) -> Self {
        list.shrink_to_fit();
        let len = list.len();

        let rv = ImmutableRenpyList {
            content: list.as_ptr(),
            len,
        };
        mem::forget(list);
        rv
    }

    pub fn from_slice(list: &[T]) -> Self {
        let len = list.len();

        ImmutableRenpyList {
            content: list.as_ptr(),
            len,
        }
    }
}

impl<T> Drop for ImmutableRenpyList<T> {
    fn drop(&mut self) {
        if self.content.is_null() {
            return;
        }

        unsafe {
            Vec::from_raw_parts(self.content as *mut T, self.len, self.len);
        }
    }
}

// Wrapper of rust strings to python strings
#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct PyString(pub *mut c_char);

impl PyString {
    fn empty() -> Self {
        PyString(null_mut())
    }
}

impl From<CString> for PyString {
    fn from(value: CString) -> Self {
        PyString(CString::into_raw(value))
    }
}

impl From<String> for PyString {
    fn from(value: String) -> Self {
        PyString(CString::into_raw(CString::new(value).unwrap_or_default()))
    }
}

impl From<PyString> for String {
    fn from(value: PyString) -> String {
        unsafe { CString::from_raw(value.0).into_string().unwrap_or_default() }
    }
}

impl Display for PyString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let x = CString::from_raw(self.0).into_string().unwrap_or_default();
            f.write_str(&x)
        }
    }
}

impl Drop for PyString {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                drop(CString::from_raw(self.0));
            }
        }
    }
}

pub type NodeID = u32;

// The object tree
// I rolled my own cause... I needed it to be sent to renpy, so I wanted the structure be the same as the schema
// But honestly this is kind of a reach lol
#[derive(Clone, Debug, Default)]
pub struct SceneTree {
    pub nodes: Vec<SceneNode>,
    pub roots: Vec<NodeID>,
}

#[derive(Debug)]
pub struct NodeNotFoundInTree;

impl Display for NodeNotFoundInTree {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Node was not found in the tree")
    }
}

impl std::error::Error for NodeNotFoundInTree {}

impl SceneTree {
    pub fn new() -> Self {
        SceneTree {
            nodes: Vec::new(),
            roots: Vec::new(),
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn push_root(&mut self, value: gltf_objects::GltfObject) -> NodeID {
        let new_index = self.nodes.len();
        self.nodes.push(SceneNode::new(value));
        self.roots.push(new_index as NodeID);

        // Truncation is fine since we will realistically not go over this... hopefully?
        new_index as NodeID
    }

    pub fn get_node(&self, node: NodeID) -> Option<&SceneNode> {
        #[allow(clippy::cast_possible_truncation)]
        // Truncation is fine since we will realistically not go over this... hopefully?
        self.nodes.get(node as usize)
    }

    pub fn get_value(&self, node: NodeID) -> Option<&gltf_objects::GltfObject> {
        match self.nodes.get(node as usize) {
            Some(node) => Some(&node.value),
            None => None,
        }
    }

    // Find Node based off gltf_object node
    pub fn find_node(&self, find_id: usize) -> Result<NodeID, NodeNotFoundInTree> {
        for (id, node) in self.nodes.iter().enumerate() {
            if node.value.is_same_id(find_id) {
                #[allow(clippy::cast_possible_truncation)]
                return Ok(id as NodeID);
            }
        }

        Err(NodeNotFoundInTree)
    }

    pub fn push(
        &mut self,
        root_node: NodeID,
        value: gltf_objects::GltfObject,
    ) -> Result<NodeID, NodeNotFoundInTree> {
        let new_index = self.nodes.len();
        self.nodes.push(SceneNode::new(value));
        match self.nodes.get_mut(root_node as usize) {
            Some(root_node) => {
                #[allow(clippy::cast_possible_truncation)]
                // Truncation is fine since we will realistically not go over this... hopefully?
                root_node.children.push(new_index as NodeID);
            }
            _ => {
                return Err(NodeNotFoundInTree);
            }
        }

        #[allow(clippy::cast_possible_truncation)]
        // Truncation is fine since we will realistically not go over this... hopefully?
        Ok(new_index as NodeID)
    }
}

#[derive(Clone, Debug)]
pub struct SceneNode {
    pub children: Vec<NodeID>,
    pub value: gltf_objects::GltfObject,
}

impl SceneNode {
    fn new(value: gltf_objects::GltfObject) -> Self {
        SceneNode {
            children: Vec::new(),
            value,
        }
    }
}

// These 2 Traits to reduce repetition and give me the little autocomplete

// Flatbuffer that doesn't need the builder because they are not variable width(?)
pub trait SimpleFlatbufferConversion {
    type Output;

    fn to_flatbuffer(&self) -> Self::Output;
}

// Objects that need the builder because they are tables or have string or something
pub trait FlatbufferConversion {
    type Output<'a>;

    fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<Self::Output<'a>>;
}

impl SimpleFlatbufferConversion for cgmath::Vector3<f32> {
    type Output = super::flatbuffer::Vector3;

    fn to_flatbuffer(&self) -> Self::Output {
        super::flatbuffer::Vector3::new(self.x, self.y, self.z)
    }
}

impl SimpleFlatbufferConversion for cgmath::Vector4<f32> {
    type Output = super::flatbuffer::Vector4;

    fn to_flatbuffer(&self) -> Self::Output {
        super::flatbuffer::Vector4::new(self.x, self.y, self.z, self.w)
    }
}

impl SimpleFlatbufferConversion for cgmath::Quaternion<f32> {
    type Output = super::flatbuffer::Vector4;

    fn to_flatbuffer(&self) -> Self::Output {
        let mut res = cgmath::Vector4::unit_w();
        let s = self.v;
        res.x = s.x;
        res.y = s.y;
        res.z = s.z;
        res.w = self.s;

        res.to_flatbuffer()
    }
}

impl SimpleFlatbufferConversion for RotationTransform {
    type Output = super::flatbuffer::Vector4;

    fn to_flatbuffer(&self) -> Self::Output {
        match self.clone() {
            RotationTransform::Quaternion(quaternion) => quaternion.to_flatbuffer(),
            RotationTransform::Euler(euler) => {
                super::flatbuffer::Vector4::new(euler.x.0, euler.y.0, euler.z.0, 1.0)
            }
        }
    }
}

impl FlatbufferConversion for gltf_loader::model::MorphTarget {
    type Output<'a> = super::flatbuffer::MorphTargets<'a>;

    fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<Self::Output<'a>> {
        let mut translation_vec = Vec::with_capacity(self.blend_shapes.len());
        for vertex in &self.blend_shapes {
            translation_vec.push(vertex.position.to_flatbuffer());
        }

        let name = builder.create_string(&self.name);
        let translation_vec = builder.create_vector(&translation_vec);

        super::flatbuffer::MorphTargets::create(
            builder,
            &super::flatbuffer::MorphTargetsArgs {
                name: Some(name),
                translation: Some(translation_vec),
            },
        )
    }
}

impl SimpleFlatbufferConversion for Matrix4<f32> {
    type Output = super::flatbuffer::Matrix4;

    fn to_flatbuffer(&self) -> Self::Output {
        let temp: [[f32; 4]; 4] = self.to_owned().into();
        let temp: [f32; 16] = temp.as_flattened().try_into().unwrap_or_default();
        super::flatbuffer::Matrix4::new(&temp)
    }
}
impl SimpleFlatbufferConversion for gltf_loader::utils::GlobalNodeIdentifier {
    type Output = super::flatbuffer::GlobalNodeIdentifier;

    fn to_flatbuffer(&self) -> Self::Output {
        match self {
            gltf_loader::utils::GlobalNodeIdentifier::SceneRoot => {
                super::flatbuffer::GlobalNodeIdentifier::new(
                    super::flatbuffer::GlobalIdType::SceneRoot,
                    0,
                )
            }
            gltf_loader::utils::GlobalNodeIdentifier::NodeId(id) => {
                super::flatbuffer::GlobalNodeIdentifier::new(
                    super::flatbuffer::GlobalIdType::NodeID,
                    *id as u64,
                )
            }
            gltf_loader::utils::GlobalNodeIdentifier::ObjectIndex(id) => {
                super::flatbuffer::GlobalNodeIdentifier::new(
                    super::flatbuffer::GlobalIdType::ObjectIndex,
                    *id as u64,
                )
            }
        }
    }
}
impl FlatbufferConversion for gltf_loader::model::Skeleton {
    type Output<'a> = super::flatbuffer::Skeleton<'a>;

    fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<Self::Output<'a>> {
        let name = builder.create_string(&self.name);
        let inverse_bind_matrices = Some(
            builder.create_vector_from_iter(
                self.inverse_bind_matrixes
                    .iter()
                    .map(|mat| mat.transpose().to_flatbuffer()),
            ),
        );

        let root_index = Some(self.root_index.to_flatbuffer());

        // Ugly code to get extras...
        let properties: Vec<_> = self
            .extras
            .iter()
            .map(|(name, value)| {
                gltf_objects::property::Property {
                    name: name.to_owned(),
                    value: value.to_owned(),
                }
                .to_flatbuffer(builder)
            })
            .collect();

        let properties = Some(builder.create_vector(&properties));

        let bones =
            Some(builder.create_vector_from_iter(self.bones.iter().map(|id| id.to_flatbuffer())));

        let args = super::flatbuffer::SkeletonArgs {
            id: self.id as u64,
            name: Some(name),
            root_index: root_index.as_ref(),
            inverse_bind_matrixes: inverse_bind_matrices,
            bones,
            properties,
        };

        super::flatbuffer::Skeleton::create(builder, &args)
    }
}

use cgmath::*;
use std::iter::Iterator;

/// Represents the 3 vertices of a triangle.
pub type Triangle = [Vertex; 3];

/// Represents the 2 vertices of a line.
pub type Line = [Vertex; 2];

/// Contains a position, normal and texture coordinates vectors.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    /// Position
    pub position: Vector3<f32>,
    /// Normalized normal
    pub normal: Vector3<f32>,
    /// Tangent normal
    /// The w component is the handedness of the tangent basis (can be -1 or 1)
    pub tangent: Vector4<f32>,
    /// Texture coordinates
    pub tex_coords: Vector2<f32>,
    // pub RGBA:Vector4<f32>,
}

impl Vertex {
    /// Get position as a tuple
    pub fn position(&self) -> (f32, f32, f32) {
        Into::<(f32, f32, f32)>::into(self.position)
    }

    /// Get vertex as a slice
    pub fn as_attribute_slice(&self) -> [f32; 3] {
        Into::<[f32; 3]>::into(self.position)
        // Into::<[f32; 3]>::into(self.normal)
        // Into::<[f32; 4]>::into(self.tangent)
        // Into::<[f32; 2]>::into(self.tex_coords)
    }
}

impl Default for Vertex {
    fn default() -> Self {
        Vertex {
            position: Zero::zero(),
            normal: Zero::zero(),
            tangent: Zero::zero(),
            tex_coords: Zero::zero(),
        }
    }
}

/// Iterator to create attributes for loading into renpy
pub struct VertexAttributeIterator<Iter, const STRIDE: usize>
where
    Iter: Iterator,
{
    iter: Iter,
    next_stride: [Option<Iter::Item>; STRIDE],
}

impl<IterType, const STRIDE: usize> VertexAttributeIterator<IterType, { STRIDE }>
where
    IterType: Iterator,
{
    /// Create Struct
    pub fn new(iter: IterType) -> Self {
        VertexAttributeIterator {
            iter,
            next_stride: [const { None }; STRIDE],
        }
    }

    /// Get the set of values for the current vertex
    pub fn get_attributes(&mut self) -> &[Option<IterType::Item>] {
        let mut i = 0;
        for val in self.iter.by_ref() {
            self.next_stride[i] = Some(val);
            i += 1;

            if i >= STRIDE {
                break;
            }
        }

        &self.next_stride
    }
}

// impl<IterType, const STRIDE: usize> VertexAttributeIterator<IterType, { STRIDE }> where IterType: Iterator, IterType::Item: Default{

//     /// Create Struct
//     pub fn new(iter: IterType) -> Self{
//         VertexAttributeIterator{
//             iter,
//             next_stride: std::array::from_fn(|_| IterType::Item::default()),
//         }
//     }

//     /// Get the set of values for the current vertex
//     pub fn get_attributes(&mut self) -> &[IterType::Item] {
//         let mut i = 0;
//         while let Some(val) = self.iter.next() {
//             self.next_stride[i] = val;
//             i += 1;

//             if i >= STRIDE {
//                 break;
//             }
//         }

//         &self.next_stride
//     }
// }

// trait VertexAttribute: Iterator {
//     // const STRIDE: usize;

//     fn get_attributes(&self) -> [f32];
// }

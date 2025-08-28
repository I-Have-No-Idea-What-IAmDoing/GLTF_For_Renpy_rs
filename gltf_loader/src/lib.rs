#![deny(missing_docs)]
// This whole crate is just a modification of the easy-gltf crate that I modified because it did not have all the features I wanted, but it was easier than just using the gltf-rust crate :/

//! This crate is intended to load [glTF 2.0](https://www.khronos.org/gltf), a
//! file format designed for the efficient transmission of 3D assets.
//!
//! It's base on [gltf](https://github.com/gltf-rs/gltf) crate but has an easy-to-use output.
//!
//! # Installation
//!
//! ```toml
//! [dependencies]
//! easy-gltf="1.1.2"
//! ```
//!
//! # Example
//!
//! ```
//! let scenes = gltf_loader::load("tests/cube.glb").expect("Failed to load glTF");
//! for scene in scenes {
//!     println!("{}", scene.objects);
//! }
//! ```

mod animation;
mod scene;

/// random utils to make life easier
pub mod utils;

use rayon::prelude::*;
use rustc_hash::FxHashMap;
use scene::model::Skeleton;
use std::path::Path;
use utils::GltfData;

pub use animation::*;
pub use scene::*;

/// Load scenes from path to a glTF 2.0.
///
/// Note: You can use this function with either a `Gltf` (standard `glTF`) or `Glb` (binary glTF).
///
/// # Example
///
/// ```
/// let scenes = gltf_loader::load("tests/cube.glb").expect("Failed to load glTF");
/// println!("Scenes: {}", scenes[0].name.clone().unwrap()); // Output "Scenes: cube"
/// ```
pub fn load<P>(path: P) -> anyhow::Result<Vec<Scene>>
where
    P: AsRef<Path>,
{
    // Run gltf
    let (doc, buffers, images) = gltf::import(&path)?;

    // Init data and collection useful for conversion
    let mut data = GltfData::new(buffers, images, &path);

    // Todo: Look into 'Chili' instead for lower overhead
    //  This can work by creating joining a thread for every 2 textures.
    let raw_images: Vec<_> = doc
        .textures()
        .par_bridge()
        .map(|tex| (tex.index(), data.load_texture(&tex)))
        .collect();
    data.raw_images.extend(raw_images);

    // Convert the animation separately because it's in the document and not in the scene
    let animation_result: Vec<Vec<(usize, GLTFAnimation)>> = doc
        .animations()
        .par_bridge()
        .map(|animation| GLTFAnimation::load(animation, &data))
        .collect();

    data.animations = {
        let mut res: FxHashMap<usize, Vec<GLTFAnimation>> = FxHashMap::default();
        for animation in animation_result {
            for (node, anim_value) in animation {
                res.entry(node).or_default().push(anim_value);
            }
        }
        res
    };

    data.skeletons = doc
        .skins()
        .par_bridge()
        .map(|skin| Skeleton::load(&skin, &data))
        .collect();

    // Convert gltf -> easy_gltf
    let mut scenes_res: Vec<Scene> = vec![];
    for scene in doc.scenes() {
        scenes_res.push(Scene::load(scene, &mut data));
    }

    Ok(scenes_res)
}

// #[cfg(test)]
// mod tests {
//     use crate::model::Mode;
//     use crate::*;
//     use cgmath::*;

//     macro_rules! assert_delta {
//         ($x:expr, $y:expr, $d:expr) => {
//             if !($x - $y < $d || $y - $x < $d) {
//                 panic!();
//             }
//         };
//     }

//     #[test]
//     fn check_cube_glb() {
//         let scenes = load("tests/cube.glb").unwrap();
//         assert_eq!(scenes.len(), 1);
//         let scene = &scenes[0];
//         assert_eq!(scene.cameras.len(), 1);
//         assert_eq!(scene.lights.len(), 3);
//         assert_eq!(scene.models.len(), 1);
//     }

//     #[test]
//     fn check_different_meshes() {
//         let scenes = load("tests/complete.glb").unwrap();
//         assert_eq!(scenes.len(), 1);
//         let scene = &scenes[0];
//         for model in scene.models.iter() {
//             match model.mode() {
//                 Mode::Triangles | Mode::TriangleFan | Mode::TriangleStrip => {
//                     assert!(model.triangles().is_ok());
//                 }
//                 Mode::Lines | Mode::LineLoop | Mode::LineStrip => {
//                     assert!(model.lines().is_ok());
//                 }
//                 Mode::Points => {
//                     assert!(model.points().is_ok());
//                 }
//             }
//         }
//     }

//     #[test]
//     fn check_cube_gltf() {
//         let _ = load("tests/cube_classic.gltf").unwrap();
//     }

//     #[test]
//     fn check_default_texture() {
//         let _ = load("tests/box_sparse.glb").unwrap();
//     }

//     #[test]
//     fn check_camera() {
//         let scenes = load("tests/cube.glb").unwrap();
//         let scene = &scenes[0];
//         let cam = &scene.cameras[0];
//         assert!((cam.position() - Vector3::new(7.3589, 4.9583, 6.9258)).magnitude() < 0.1);
//     }

//     #[test]
//     fn check_lights() {
//         let scenes = load("tests/cube.glb").unwrap();
//         let scene = &scenes[0];
//         for light in scene.lights.iter() {
//             match light {
//                 Light::Directional {
//                     direction,
//                     color: _,
//                     intensity,
//                     ..
//                 } => {
//                     assert!((direction - Vector3::new(0.6068, -0.7568, -0.2427)).magnitude() < 0.1);
//                     assert_delta!(intensity, 542., 0.01);
//                 }
//                 Light::Point {
//                     position,
//                     color: _,
//                     intensity,
//                     ..
//                 } => {
//                     assert!((position - Vector3::new(4.0762, 5.9039, -1.0055)).magnitude() < 0.1);
//                     assert_delta!(intensity, 1000., 0.01);
//                 }
//                 Light::Spot {
//                     position,
//                     direction,
//                     color: _,
//                     intensity,
//                     inner_cone_angle: _,
//                     outer_cone_angle,
//                     ..
//                 } => {
//                     assert!((position - Vector3::new(4.337, 15.541, -8.106)).magnitude() < 0.1);
//                     assert!(
//                         (direction - Vector3::new(-0.0959, -0.98623, 0.1346)).magnitude() < 0.1
//                     );
//                     assert_delta!(intensity, 42., 0.01);
//                     assert_delta!(outer_cone_angle, 40., 0.01);
//                 }
//             }
//         }
//     }

//     #[test]
//     fn check_model() {
//         let scenes = load("tests/cube.glb").unwrap();
//         let scene = &scenes[0];
//         let model = &scene.models[0];
//         assert!(model.has_normals());
//         assert!(model.has_tex_coords());
//         assert!(model.has_tangents());
//         for t in model.triangles().unwrap().iter().flatten() {
//             let pos = t.position;
//             assert!(pos.x > -0.01 && pos.x < 1.01);
//             assert!(pos.y > -0.01 && pos.y < 1.01);
//             assert!(pos.z > -0.01 && pos.z < 1.01);

//             // Check that the tangent w component is 1 or -1
//             assert_eq!(t.tangent.w.abs(), 1.);
//         }
//     }

//     #[test]
//     fn check_material() {
//         let scenes = load("tests/head.glb").unwrap();
//         let scene = &scenes[0];
//         let mat = &scene.models[0].material;
//         assert!(mat.pbr.base_color_texture.is_some());
//         assert_eq!(mat.pbr.metallic_factor, 0.);
//     }

//     #[test]
//     fn check_invalid_path() {
//         assert!(load("tests/invalid.glb").is_err());
//     }
// }

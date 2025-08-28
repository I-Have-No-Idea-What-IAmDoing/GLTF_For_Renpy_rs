// Lints from: https://corrode.dev/blog/pitfalls-of-safe-rust/

// Arithmetic
#![deny(arithmetic_overflow)] // Prevent operations that would cause integer overflow
#![deny(clippy::checked_conversions)] // Suggest using checked conversions between numeric types
#![deny(clippy::cast_possible_truncation)] // Detect when casting might truncate a value
#![deny(clippy::cast_sign_loss)] // Detect when casting might lose sign information
#![deny(clippy::cast_possible_wrap)] // Detect when casting might cause value to wrap around
#![deny(clippy::cast_precision_loss)] // Detect when casting might lose precision
#![deny(clippy::integer_division)] // Highlight potential bugs from integer division truncation
#![deny(clippy::arithmetic_side_effects)] // Detect arithmetic operations with potential side effects
#![deny(clippy::unchecked_duration_subtraction)] // Ensure duration subtraction won't cause underflow

// Unwraps
#![warn(clippy::unwrap_used)] // Discourage using .unwrap() which can cause panics
#![warn(clippy::expect_used)] // Discourage using .expect() which can cause panics
#![deny(clippy::panicking_unwrap)] // Prevent unwrap on values known to cause panics
#![deny(clippy::option_env_unwrap)] // Prevent unwrapping environment variables which might be absent

// Array indexing
#![deny(clippy::indexing_slicing)] // Avoid direct array indexing and use safer methods like .get()

// Path handling
#![deny(clippy::join_absolute_paths)] // Prevent issues when joining paths with absolute paths

// Serialization issues
#![deny(clippy::serde_api_misuse)] // Prevent incorrect usage of Serde's serialization/deserialization API

// Unbounded input
#![deny(clippy::uninit_vec)] // Prevent creating uninitialized vectors which is unsafe

// Unsafe code detection
#![deny(unnecessary_transmutes)]

use gltf_for_renpy_flatbuffer as flatbuffer;

pub mod animations;
pub mod gltf_objects;
pub mod images;
pub mod renpy_interop;

use animations::*;
use gltf_loader::{self};
use gltf_objects::{
    GltfObject,
    empty::Empty,
    mesh::Mesh,
    property::{Properties, Property},
};
use images::*;
use renpy_interop::*;

use gltf_loader::Scene;

use std::{
    collections::HashMap,
    ffi::{CStr, c_char, c_void},
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
};

#[unsafe(no_mangle)]
pub extern "C" fn free_scene_list(ptr: *mut c_void) {
    if ptr.is_null() || !ptr.is_aligned() {
        return;
    }

    let ptr = ptr as *mut GLTFResult<ImmutableRenpyList<u8>>;

    unsafe {
        drop(Box::from_raw(ptr));
    }
}

macro_rules! gltf_try {
    ($code_block:expr, $err_code:expr) => {
        match $code_block {
            Ok(good) => good,
            Err(err) => return GLTFResult::error($err_code, err.to_string()),
        }
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResultCode {
    Ok = 0,
    NullPath = -1,
    InvalidPath = -2,
    BadFileProcessing = -3,
    DatabaseOpenFailure = -4,
    DatabaseCreationFailure = -5,
    DatabaseInsertionFailure = -6,
    DatabaseExtractionFailure = -7,
    DatabaseTransactionFailure = -8,
}

impl std::fmt::Display for ResultCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResultCode::{self:?}")
    }
}

fn get_from_cache(db_path: &str, model_path: &str) -> anyhow::Result<ImmutableRenpyList<u8>> {
    #[cfg(feature = "rocksdb")]
    match DB::open_default(db_path) {
        Ok(db) => {
            let mut hasher = DefaultHasher::default();
            model_path.hash(&mut hasher);
            match db.get(hasher.finish().to_be_bytes()) {
                Ok(val) => match val {
                    Some(val) => {
                        let res = ImmutableRenpyList::from(val);
                        return GLTFResult::ok(res);
                    }
                    None => {
                        return GLTFResult::error(
                            ResultCode::DatabaseExtractionFailure,
                            "The value could not be found in the database".to_string(),
                        );
                    }
                },
                Err(err) => {
                    return GLTFResult::error(
                        ResultCode::DatabaseExtractionFailure,
                        err.to_string(),
                    );
                }
            }
        }
        Err(err) => return GLTFResult::error(ResultCode::DatabaseOpenFailure, err.to_string()),
    };

    #[cfg(feature = "sqlite")]
    match rusqlite::Connection::open(db_path) {
        Ok(connection) => {
            let mut hasher = DefaultHasher::default();
            model_path.hash(&mut hasher);

            #[allow(clippy::cast_possible_truncation)]
            // Truncation is fine since it's just a hash
            let hash = hasher.finish() as u32;

            let mut query = connection.prepare("SELECT data FROM models WHERE id = ?1")?;
            let result =
                query.query_row(rusqlite::params![hash], |row| row.get::<usize, Vec<u8>>(0));

            let blob_val = result?;
            let rv = ImmutableRenpyList::from(blob_val);

            Ok(rv)
        }
        Err(err) => Err(err.into()),
    }
}

// TODO: Consider using Mesh Optimizer to speed up rendering?
// TODO: Switch To ASSIMP so that we can use any format instead of only GLTF
//  We probably use gltf as a base
//  Only problem is that right now ASSIMP does not support rust :/

/// # Safety
///
/// Even more untested shit
#[unsafe(no_mangle)]
pub unsafe fn load_from_cache(
    db_path: *const c_char,
    model_path: *const c_char,
) -> *const GLTFResult<ImmutableRenpyList<u8>> {
    unsafe {
        if db_path.is_null() {
            return GLTFResult::error(
                ResultCode::NullPath,
                "The database path that was given was a null pointer.".to_string(),
            );
        }

        if model_path.is_null() {
            return GLTFResult::error(
                ResultCode::NullPath,
                "The model that was given was a null pointer.".to_string(),
            );
        }

        let model_path = gltf_try!(CStr::from_ptr(model_path).to_str(), ResultCode::InvalidPath);

        let raw_db_path = CStr::from_ptr(db_path);

        let db_path = raw_db_path.to_str();

        if let Ok(db_path) = db_path {
            match std::fs::exists(db_path) {
                Ok(does_exists) if !does_exists => {
                    return GLTFResult::error(
                        ResultCode::InvalidPath,
                        "The path to the database did not exist.".to_string(),
                    );
                }
                Err(err) => return GLTFResult::error(ResultCode::InvalidPath, err.to_string()),
                _ => {}
            };

            let rv = gltf_try!(
                get_from_cache(db_path, model_path),
                ResultCode::DatabaseExtractionFailure
            );
            return GLTFResult::ok(rv);
        }

        GLTFResult::error(ResultCode::InvalidPath, "The path contained could not be converted in Rust. This is likely because it did not contain valid UFT-8 characters.".to_string())
    }
}

/// # Safety
///
/// The most untested shit
#[unsafe(no_mangle)]
pub unsafe fn load_all_from_cache(
    db_path: *const c_char,
    model_path: *const *const c_char,
    model_count: usize,
) -> *const GLTFResult<ImmutableRenpyList<ImmutableRenpyList<u8>>> {
    unsafe {
        if db_path.is_null() {
            return GLTFResult::error(
                ResultCode::NullPath,
                "The database path that was given was a null pointer.".to_string(),
            );
        }

        if model_path.is_null() {
            return GLTFResult::error(
                ResultCode::NullPath,
                "The model that was given was a null pointer.".to_string(),
            );
        }

        let raw_db_path = CStr::from_ptr(db_path);
        let db_path = gltf_try!(raw_db_path.to_str(), ResultCode::InvalidPath);

        let mut model_vec = Vec::with_capacity(model_count);
        for index in 0..model_count {
            let model_path = *model_path.wrapping_add(index);
            if !model_path.is_null() && model_path.is_aligned() {
                let model_path =
                    gltf_try!(CStr::from_ptr(model_path).to_str(), ResultCode::InvalidPath);

                let rv = gltf_try!(
                    get_from_cache(db_path, model_path),
                    ResultCode::DatabaseExtractionFailure
                );
                model_vec.push(rv);
            }
        }

        GLTFResult::ok(ImmutableRenpyList::from(model_vec))
    }
}

/// # Safety
///
/// Untested shit
#[unsafe(no_mangle)]
pub unsafe fn save_all_to_cache(
    db_path: *const c_char,
    model_paths: *const *const c_char,
    model_path_length: usize,
) -> *const GLTFResult<bool> {
    if db_path.is_null() {
        return GLTFResult::error(
            ResultCode::NullPath,
            "The path that was given was a null pointer.".to_string(),
        );
    }

    let raw_db_path = unsafe { CStr::from_ptr(db_path) };

    let db_path = raw_db_path.to_str();

    if let Ok(db_path) = db_path {
        let db_path: &Path = Path::new(db_path);

        // Create the directory if it does not exist for the database file
        if let Some(p) = db_path.parent() {
            gltf_try!(std::fs::create_dir_all(p), ResultCode::InvalidPath)
        };

        // I am not sure if this works since I really only use sqlite
        #[cfg(feature = "rocksdb")]
        match DB::open_default(path) {
            Ok(db) => {
                for index in 0..model_path_length {
                    unsafe {
                        let model_path = model_paths.wrapping_add(index);
                        let model_path = *model_path;

                        if !model_path.is_null() && model_path.is_aligned() {
                            let model_path = gltf_try!(
                                CStr::from_ptr(model_path).to_str(),
                                ResultCode::InvalidPath
                            );

                            let mut hasher = DefaultHasher::default();
                            model_path.hash(&mut hasher);

                            let model = gltf_try!(
                                load_scene_list(path, true),
                                ResultCode::BadFileProcessing
                            );

                            gltf_try!(
                                db.put(hasher.finish().to_be_bytes(), model),
                                ResultCode::DatabaseInsertionFailure
                            );
                        } else {
                            return GLTFResult::error(
                                ResultCode::NullPath,
                                "One of the model path that was given was a null pointer."
                                    .to_string(),
                            );
                        }
                    }
                }

                return GLTFResult::ok(true);
            }
            Err(err) => return GLTFResult::error(ResultCode::DatabaseOpenFailure, err.to_string()),
        }

        #[cfg(feature = "sqlite")]
        {
            let mut connection = gltf_try!(
                rusqlite::Connection::open(db_path),
                ResultCode::DatabaseOpenFailure
            );

            // Starts a transaction so that everything is atomic which I think is good…?
            let tx = gltf_try!(
                connection.transaction(),
                ResultCode::DatabaseTransactionFailure
            );

            // We store all the model data in one big table with the id being the filepath through a hashing function
            gltf_try!(
                tx.execute(
                    "CREATE TABLE IF NOT EXISTS models (
                id INTEGER PRIMARY KEY,
                data BLOB)",
                    []
                ),
                ResultCode::DatabaseCreationFailure
            );
            gltf_try!(tx.commit(), ResultCode::DatabaseTransactionFailure);

            let tx = gltf_try!(
                connection.transaction(),
                ResultCode::DatabaseTransactionFailure
            );

            for index in 0..model_path_length {
                unsafe {
                    // We get the current model path from the array using pointer arithmetic
                    let model_path = model_paths.wrapping_add(index);
                    let model_path = *model_path;

                    if !model_path.is_null() && model_path.is_aligned() {
                        let model_path =
                            gltf_try!(CStr::from_ptr(model_path).to_str(), ResultCode::InvalidPath);

                        let mut hasher = DefaultHasher::default();
                        model_path.hash(&mut hasher);

                        #[allow(clippy::cast_possible_truncation)]
                        // Truncation is fine since it's just a hash
                        let hash = hasher.finish() as u32;

                        // Actually loads the model like normal
                        let model = gltf_try!(
                            load_scene_list(model_path, true),
                            ResultCode::BadFileProcessing
                        );

                        gltf_try!(
                            tx.execute(
                                "REPLACE INTO models
                                              VALUES (?1, ?2); ",
                                (hash, model)
                            ),
                            ResultCode::DatabaseInsertionFailure
                        );
                    } else {
                        return GLTFResult::error(
                            ResultCode::NullPath,
                            "One of the model path that was given was a null pointer or was not aligned.".to_string(),
                        );
                    }
                }
            }
            gltf_try!(tx.commit(), ResultCode::DatabaseTransactionFailure);
            return GLTFResult::ok(true);
        }
    }

    GLTFResult::error(ResultCode::InvalidPath, "The path contained could not be converted in Rust. This is likely because it did not contain valid UFT-8 characters.".to_string())
}

fn load_scene_list<T: AsRef<Path>>(path: T, use_embed_textures: bool) -> anyhow::Result<Vec<u8>> {
    let loaded_file = gltf_loader::load(path);

    let scenes: Vec<Scene> = match loaded_file {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return Err(err);
        }
    };

    let mut scene_list: Vec<gltf_objects::RenpyScene> = Vec::with_capacity(scenes.len());

    for scene in scenes {
        let scene_name = scene.name.clone().unwrap_or("Scene".to_owned());

        let mut gltf_object: SceneTree = SceneTree::new();
        let mut node_mapping: HashMap<ego_tree::NodeId, NodeID> = HashMap::default();
        let mut empty_index = Vec::new();
        let mut mesh_index = Vec::new();

        // Depth first search of the scene tree
        for object in scene.objects.root().descendants() {
            let value = match object.value() {
                gltf_loader::SceneObject::Root => continue,
                gltf_loader::SceneObject::Mesh(model) => {
                    Mesh::create(model, scene_name.clone(), use_embed_textures)
                }
                gltf_loader::SceneObject::Empties(empty) => {
                    Empty::create(empty, scene_name.clone())
                }
            };

            match object.parent() {
                Some(node) => {
                    let tree_index;
                    if let Some(node_id) = node_mapping.get(&node.id()) {
                        if let Ok(new_id) = gltf_object.push(*node_id, value) {
                            node_mapping.insert(object.id(), new_id);
                            tree_index = new_id;
                        } else {
                            // If we can't get the parent then something has gone really wrong so just skip to the next object
                            continue;
                        }
                    } else {
                        // If we can't find the node then it's either an orphan or a root node... so let's just say they are all roots :)
                        tree_index = gltf_object.push_root(value);
                        node_mapping.insert(object.id(), tree_index);
                    }

                    match object.value() {
                        // Objects with Parents can't be roots lol
                        gltf_loader::SceneObject::Root => unreachable!(),
                        gltf_loader::SceneObject::Mesh(_) => {
                            mesh_index.push(tree_index);
                        }
                        gltf_loader::SceneObject::Empties(_) => {
                            empty_index.push(tree_index);
                        }
                    }
                }
                None => {
                    // No parent means it's the root node which holds nothing and is already skipped for us
                    unreachable!()
                }
            }
        }

        let scene_properties = Property::load(scene.extras);

        scene_list.push(gltf_objects::RenpyScene {
            name: scene.name.clone().unwrap_or_default(),
            objects: gltf_object,
            properties: scene_properties,
            mesh_indexes: mesh_index,
            empty_indexes: empty_index,
        });
    }

    // Scene Graph Post-processing
    // - This is mostly to turn node relationships into indexes for faster processing
    for scene in &mut scene_list {
        let read_only_scene = scene.to_owned();
        for obj in &mut scene.objects.nodes {
            match &mut obj.value {
                GltfObject::Empty(_, _) => {}
                GltfObject::Mesh(_, mesh) => {
                    if let Some(skeleton) = &mut mesh.skeleton {
                        for bone in &mut skeleton.bones {
                            match bone {
                                gltf_loader::utils::GlobalNodeIdentifier::SceneRoot => {
                                    unreachable!()
                                }
                                gltf_loader::utils::GlobalNodeIdentifier::NodeId(old_id) => {
                                    let new_id = read_only_scene.objects.find_node(*old_id);
                                    if let Ok(new_id) = new_id {
                                        *bone =
                                            gltf_loader::utils::GlobalNodeIdentifier::ObjectIndex(
                                                new_id as usize,
                                            );
                                    }
                                }
                                gltf_loader::utils::GlobalNodeIdentifier::ObjectIndex(_) => {}
                            }
                        }
                    }
                }
            }
        }
    }

    let mut builder = flatbuffers::FlatBufferBuilder::new();

    let scene_list = scene_list
        .into_iter()
        .map(|old_scene| gltf_objects::convert_scene_to_flatbuffer(old_scene, &mut builder))
        .collect::<Vec<_>>();

    let scene_list = builder.create_vector(&scene_list);

    let scenes = flatbuffer::Scenes::create(
        &mut builder,
        &flatbuffer::ScenesArgs {
            scenes: Some(scene_list),
        },
    );

    builder.finish(scenes, Some("3dPY"));

    Ok(builder.finished_data().to_vec())
}

/// # Safety
///
/// Honestly it's pretty safe in that it works for me, but I have not really audited it lmao
#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_file(
    file_path: *const c_char,
    use_embed_textures: bool,
) -> *const GLTFResult<ImmutableRenpyList<u8>> {
    if file_path.is_null() {
        return GLTFResult::error(
            ResultCode::NullPath,
            "The path that was given was a null pointer.".to_string(),
        );
    }

    let raw_file_path = unsafe { CStr::from_ptr(file_path) };

    let file_path = raw_file_path.to_str();

    if let Ok(path) = file_path {
        let result = gltf_try!(
            load_scene_list(path, use_embed_textures),
            ResultCode::BadFileProcessing
        );
        let rv = ImmutableRenpyList::from(result);

        return GLTFResult::ok(rv);
    }

    GLTFResult::error(ResultCode::InvalidPath, "The path contained could not be converted in Rust. This is likely because it did not contain valid UFT-8 characters.".to_string())
}

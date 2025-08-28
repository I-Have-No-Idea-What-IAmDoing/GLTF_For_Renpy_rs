#![allow(unused)]
use cgmath::{InnerSpace, Quaternion};
use ego_tree::{Tree, tree};
use gltf_for_renpy_flatbuffer as flatbuffer;
use gltf_loader::utils::{quaterions_to_euler, quaterions_to_zyx_euler};
use itertools::Itertools;

use gltf_for_renpy::animations::*;
use gltf_for_renpy::images::*;
use gltf_for_renpy::*;
use std::ffi::{CString, c_void};
use std::io::Write;
use std::mem;
use std::ptr::slice_from_raw_parts;

#[test]
fn basic_test() {
    let file_path = CString::new("tests/TestCubeModel.glb").unwrap();
    unsafe {
        let model_list = load_file(file_path.as_ptr(), true);
        match (*model_list).result_type {
            ResultCode::Ok => {
                let content = &*(*model_list).content;

                let scenes = slice_from_raw_parts(content.content, content.len);
                let scenes = flatbuffer::root_as_scenes(&*scenes).unwrap();

                println!("{:?}", scenes.scenes().len());

                for x in scenes.scenes() {
                    for y in x.objects() {
                        let y = y.object_as_mesh().unwrap();
                        println!("{:?}", y.name());

                        // 24 Vertices * 3 Positions (X,Y,Z) Values
                        assert_eq!(y.points().len(), 24 * 3);
                        assert!(y.animations().is_empty());
                    }
                }
            }
            _ => {
                println!(
                    "{:?}: {}",
                    (*model_list).result_type,
                    ((*model_list).error_description)
                );
            }
        }

        free_scene_list(model_list as *mut c_void);
    }
}

fn print_node(node: &gltf_for_renpy_flatbuffer::Node) {
    if node.object_type() == flatbuffer::Object::Mesh {
        let y = node.object_as_mesh().unwrap();
        println!("{:?}", y.name());
    } else if node.object_type() == flatbuffer::Object::Empties {
        let y = node.object_as_empties().unwrap();

        println!("{:?}", y.name());
    } else {
        println!("idk...")
    }
}

#[test]
fn intergration() {
    let file_path = CString::new("tests/TestYukikioModelStylized.glb").unwrap();

    unsafe {
        let model_list = load_file(file_path.as_ptr(), true);
        match (*model_list).result_type {
            ResultCode::Ok => {
                let content = &*(*model_list).content;

                let scenes = slice_from_raw_parts(content.content, content.len);
                let scenes = flatbuffer::root_as_scenes(&*scenes).unwrap();

                println!("Scene Len: {:?}", scenes.scenes().len());

                for x in scenes.scenes() {
                    println!("Roots:");

                    println!("[");
                    for r_nodes in x.root_nodes() {
                        print_node(&x.objects().get(r_nodes as usize));
                    }
                    println!("]");

                    println!("\n");
                    println!("--{}--", x.name());
                    for y in x.objects() {
                        print_node(&y);
                    }
                }
            }
            _ => {
                let raw_description = CString::from_raw((*model_list).error_description.0);
                let error_description = raw_description.clone();
                mem::forget(raw_description);
                panic!(
                    "Error Type: {:?}\n  Error Description: {:?}",
                    (*model_list).result_type,
                    error_description
                );
            }
        }

        free_scene_list(model_list as *mut c_void);
    }
}

#[test]
fn animation_test() {
    let file_path = CString::new("tests/AnimationTest.glb").unwrap();

    unsafe {
        let model_list = load_file(file_path.as_ptr(), true);
        match (*model_list).result_type {
            ResultCode::Ok => {
                let content = &*(*model_list).content;

                let scenes = slice_from_raw_parts(content.content, content.len);
                let scenes = flatbuffer::root_as_scenes(&*scenes).unwrap();

                println!("Scene Len: {:?}", scenes.scenes().len());

                for x in scenes.scenes() {
                    for y in x.objects() {
                        if y.object_type() == flatbuffer::Object::Mesh {
                            let y = y.object_as_mesh().unwrap();
                            println!("{:?}", y.name());
                            for a in y.animations().iter() {
                                println!("Set Name: {:?}", a.name());

                                if let Some(anim) = a.animations() {
                                    println!("{:?}", anim.interpolation().unwrap().0);
                                    println!(
                                        "Anim dur: {:?} {}",
                                        anim.duration(),
                                        anim.target_type().variant_name().unwrap(),
                                    );

                                    let mut index = 0_u32;
                                    for frame in anim.frames().unwrap().iter() {
                                        // let value = frame.value().unwrap().rotation().unwrap();
                                        // let value = cgmath::Vector3::new(value.x(), value.y(), value.z());
                                        // if let Some(other) = list.last(){
                                        //     println!("{:?} * {:?} = {:?}", value, other, value.dot(*other));
                                        // }

                                        println!(
                                            "Frame ({:?}:{index}): {:#?}",
                                            frame.time(),
                                            frame.value().unwrap().rotation().unwrap()
                                        );
                                        index += 1;
                                    }
                                }
                            }
                        } else if y.object_type() == flatbuffer::Object::Empties {
                            let y = y.object_as_empties().unwrap();

                            println!("{:?}", y.name());
                        } else {
                            println!("idk...")
                        }
                    }
                }
            }
            _ => {
                let raw_description = CString::from_raw((*model_list).error_description.0);
                let error_description = raw_description.clone();
                mem::forget(raw_description);
                panic!(
                    "Error Type: {:?}\n  Error Description: {:?}",
                    (*model_list).result_type,
                    error_description
                );
            }
        }

        free_scene_list(model_list as *mut c_void);
    }
}

#[test]
fn cache_test() {
    let db_path: CString = CString::new(
        std::path::Path::new("./tests/cache/test.db")
            .to_str()
            .unwrap(),
    )
    .unwrap();

    let model_path = CString::new("tests/TestComplexAnimation.glb").unwrap();

    unsafe {
        let x = save_all_to_cache(db_path.as_ptr(), &model_path.as_ptr(), 1);
        if (*x).result_type != ResultCode::Ok {
            println!("{:?}: {}", (*x).result_type, ((*x).error_description));
        } else {
            println!("Insertion Success");
        }
    }

    unsafe {
        let x = load_from_cache(db_path.as_ptr(), model_path.as_ptr());
        if (*x).result_type != ResultCode::Ok {
            println!("{:?}: {}", (*x).result_type, ((*x).error_description));
        } else {
            println!("Extraction Success");
        }
        let content = &*(*x).content;

        let scenes = slice_from_raw_parts(content.content, content.len);

        let scenes = flatbuffer::root_as_scenes(&*scenes).unwrap();
        println!("{:?}", scenes.scenes().len());

        for scene in scenes.scenes() {
            println!("{}", scene.name());
        }
    }
}

#[test]
fn morph_targets_test() {
    let file_path = CString::new("tests/MorphTargets/MorphTargetsTest.glb").unwrap();

    unsafe {
        let model_list = load_file(file_path.as_ptr(), true);
        match (*model_list).result_type {
            ResultCode::Ok => {
                let content = &*(*model_list).content;

                let scenes = slice_from_raw_parts(content.content, content.len);
                let scenes = flatbuffer::root_as_scenes(&*scenes).unwrap();

                println!("Scene Len: {:?}", scenes.scenes().len());

                for x in scenes.scenes() {
                    for y in x.objects() {
                        if y.object_type() == flatbuffer::Object::Mesh {
                            let y = y.object_as_mesh().unwrap();
                            println!(
                                "Y: {:?}",
                                y.animations()
                                    .get(0)
                                    .animations()
                                    .unwrap()
                                    .frames()
                                    .unwrap()
                                    .len()
                            );

                            for z in y.morph_targets().unwrap() {
                                // println!("Z: {}", z.translation().unwrap().len());
                                for zz in z.translation().unwrap() {
                                    println!("Z: {} {} {}", zz.x(), zz.y(), zz.z());
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                let raw_description = CString::from_raw((*model_list).error_description.0);
                let error_description = raw_description.clone();
                mem::forget(raw_description);
                panic!(
                    "Error Type: {:?}\n  Error Description: {:?}",
                    (*model_list).result_type,
                    error_description
                );
            }
        }

        free_scene_list(model_list as *mut c_void);
    }
}

#[test]
fn skeleton_test() {
    let file_path = CString::new("tests/TestComplexAnimation.glb").unwrap();

    unsafe {
        let model_list = load_file(file_path.as_ptr(), true);
        match (*model_list).result_type {
            ResultCode::Ok => {
                let content = &*(*model_list).content;

                let scenes = slice_from_raw_parts(content.content, content.len);
                let scenes = flatbuffer::root_as_scenes(&*scenes).unwrap();

                println!("Scene Len: {:?}", scenes.scenes().len());

                for x in scenes.scenes() {
                    for y in x.objects() {
                        if y.object_type() == flatbuffer::Object::Mesh {
                            let m = y.object_as_mesh().unwrap();
                            if let Some(y) = m.skeleton() {
                                let mats = y.inverse_bind_matrixes().expect("mat gone lmayo");

                                // println!("mats: {}", mats.len());
                                // println!("root: {}", y.root_index().expect("root gone lol").id());
                                let bones = y.bones().expect("bone gone lel");
                                // println!("Bones ({}):", bones.len());
                                for bone in bones {
                                    print!(" {} |", bone.id());
                                }
                            }
                            print!("\n");
                        }

                        // if y.object_type() == flatbuffer::Object::Empties {
                        //     let y = y.object_as_empties().unwrap();
                        //     for x in y.animations(){
                        //         for x in x.animations().unwrap(){
                        //             for x in x.frames().unwrap(){
                        //                 // if y.name() == "Scene:mixamorig:Spine1" {
                        //                 //     dbg!(y.name() , x.value().unwrap().rotation());
                        //                 // }
                        //                 // else{
                        //                 //     break;
                        //                 // }
                        //                 break;
                        //             }
                        //         }
                        //     }
                        // }
                    }
                }
            }
            _ => {
                let raw_description = CString::from_raw((*model_list).error_description.0);
                let error_description = raw_description.clone();
                mem::forget(raw_description);
                panic!(
                    "Error Type: {:?}\n  Error Description: {:?}",
                    (*model_list).result_type,
                    error_description
                );
            }
        }

        free_scene_list(model_list as *mut c_void);
    }
}

#[test]
fn asad() {
    // let file_path =
    //     CString::new("tests/TestCubeModel.glb")
    //         .unwrap();
    // unsafe {
    //     let model_list = load_file(file_path.as_ptr(), true);
    // }

    // let x = ego_tree::tree!(
    //     "root" => {
    //         "child a" => {
    //             "grandchild 1",
    //         },
    //         "child b" => {
    //             "grandchild a",
    //             "grandchild b",
    //         },
    //         "child c",
    // });

    // for t in  x.root().descendants(){
    //     dbg!(t.value());
    // }
}

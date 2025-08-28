// use gltf_for_renpy::renpy_interop::SceneTree;

// #[test]
// fn basic_tree() {
//     let mut tree: SceneTree = SceneTree::new();
//     let x = tree.push_root(1);
//     let y = tree.push_root(2);
//     let z = tree.push(x, 3).unwrap();
//     tree.push(z, 4).unwrap();

//     assert_eq!(tree.nodes.len(), 4);
//     assert_eq!(tree.roots.len(), 2);
//     assert_eq!(tree.get_node(x).unwrap().value, 1);
//     assert_eq!(*tree.get_value(y).unwrap(), 2);
//     assert_eq!(tree.get_node(z).unwrap().children.len(), 1);
// }

use crate::SceneTree;
use crate::renpy_interop::NodeID;
use gltf_for_renpy_flatbuffer as flatbuffer;
use gltf_loader::utils::DecomposedTransform;
use nohash_hasher::IntSet;

pub type ObjectId = usize;

pub mod empty;
pub mod mesh;

pub mod property;

// First item refers to ID's associated with that object including any objects the object itself contain for fast id checking
// Second item is the actual object itself boxed on the heap for its own safety (oh no)
#[derive(Clone, Debug)]
pub enum GltfObject {
    Empty(IntSet<ObjectId>, Box<empty::Empty>),
    Mesh(IntSet<ObjectId>, Box<mesh::Mesh>),
}

impl GltfObject {
    pub fn is_same_id(&self, other_id: usize) -> bool {
        match &self {
            GltfObject::Empty(_, empty) => empty.id == other_id,
            GltfObject::Mesh(_, mesh) => mesh.id == other_id,
        }
    }
}

#[derive(Clone)]
pub struct RenpyScene {
    pub name: String,
    // The objects are stored as a tree instead of a flat list because it make it easier to apply transformation during animation like this
    pub objects: SceneTree,
    pub properties: crate::Properties,
    pub mesh_indexes: Vec<NodeID>,
    pub empty_indexes: Vec<NodeID>,
}

pub(crate) fn convert_scene_to_flatbuffer<'a>(
    old_scene: RenpyScene,
    builder: &mut flatbuffers::FlatBufferBuilder<'a>,
) -> flatbuffers::WIPOffset<flatbuffer::GltfScene<'a>> {
    let name = builder.create_string(&old_scene.name);

    let mut temp_prop = Vec::with_capacity(old_scene.properties.len());
    for property in old_scene.properties {
        let name = builder.create_string(&property.name);
        let value = builder.create_string(&property.value);

        let new_prop = flatbuffer::Property::create(
            builder,
            &flatbuffer::PropertyArgs {
                name: Some(name),
                value: Some(value),
            },
        );

        temp_prop.push(new_prop);
    }
    let properties = builder.create_vector(&temp_prop);

    let mut temp_nodes: Vec<flatbuffers::WIPOffset<gltf_for_renpy_flatbuffer::Node<'_>>> =
        Vec::new();

    for object in old_scene.objects.nodes {
        let children = builder.create_vector(&object.children);

        let (object_type, object) = match object.value {
            GltfObject::Empty(_, empty) => {
                let temp = empty.to_flatbuffer(builder);
                (flatbuffer::Object::Empties, temp.as_union_value())
            }
            GltfObject::Mesh(_, mesh) => {
                let temp = mesh.to_flatbuffer(builder);
                (flatbuffer::Object::Mesh, temp.as_union_value())
            }
        };

        temp_nodes.push(flatbuffer::Node::create(
            builder,
            &flatbuffer::NodeArgs {
                children: Some(children),
                object_type,
                object: Some(object),
            },
        ));
    }

    let nodes = builder.create_vector(&temp_nodes);
    let root_nodes = builder.create_vector(&old_scene.objects.roots);
    let empty_index = Some(builder.create_vector(old_scene.empty_indexes.as_slice()));
    let mesh_index = Some(builder.create_vector(old_scene.mesh_indexes.as_slice()));

    flatbuffer::GltfScene::create(
        builder,
        &flatbuffer::GltfSceneArgs {
            name: Some(name),
            properties: Some(properties),
            objects: Some(nodes),
            root_nodes: Some(root_nodes),
            model_index: mesh_index,
            empty_index,
        },
    )
}

impl crate::SimpleFlatbufferConversion for DecomposedTransform {
    type Output = flatbuffer::Transform;

    fn to_flatbuffer(&self) -> Self::Output {
        flatbuffer::Transform::new(
            &self.translation.to_flatbuffer(),
            &self.rotation.to_flatbuffer(),
            &self.scale.to_flatbuffer(),
        )
    }
}

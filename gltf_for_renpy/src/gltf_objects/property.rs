use std::collections::HashMap;

pub type Properties = Vec<Property>;

// Your Basic Key-Value Pair/Dictionary to store any type of metadata
#[derive(Clone, Debug)]
pub struct Property {
    pub name: String,
    pub value: String,
}

impl Property {
    pub fn load(data: Option<HashMap<String, String>>) -> Properties {
        data.map(|extras| {
            extras
                .into_iter()
                .map(|(name, value)| Property { name, value })
                .collect()
        })
        .unwrap_or_default()
    }

    pub fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<gltf_for_renpy_flatbuffer::Property<'a>> {
        let name = builder.create_string(&self.name);
        let value = builder.create_string(&self.value);

        gltf_for_renpy_flatbuffer::Property::create(
            builder,
            &gltf_for_renpy_flatbuffer::PropertyArgs {
                name: Some(name),
                value: Some(value),
            },
        )
    }
}

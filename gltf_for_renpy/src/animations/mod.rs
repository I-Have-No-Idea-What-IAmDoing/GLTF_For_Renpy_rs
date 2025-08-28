use std::fmt::Debug;

use gltf_loader::InterpolationTargets;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;

use crate::flatbuffer;
use crate::renpy_interop::*;

impl FlatbufferConversion for gltf_loader::GLTFAnimationFrame {
    type Output<'a> = flatbuffer::AnimationKeyFrames<'a>;

    fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<gltf_for_renpy_flatbuffer::AnimationKeyFrames<'a>> {
        let value = self.value.to_flatbuffer(builder);
        let value = Some(value);

        flatbuffer::AnimationKeyFrames::create(
            builder,
            &flatbuffer::AnimationKeyFramesArgs {
                time: self.time,
                value,
            },
        )
    }
}

impl FlatbufferConversion for gltf_loader::GLTFAnimationValue {
    type Output<'a> = gltf_for_renpy_flatbuffer::AnimationValues<'a>;

    fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<Self::Output<'a>> {
        let binding = self.transformation.translation.to_flatbuffer();
        let translation = Some(&binding);
        let binding = self.transformation.rotation.to_flatbuffer();
        let rotation = Some(&binding);
        let binding = self.transformation.scale.to_flatbuffer();
        let scale = Some(&binding);

        let weights = Some(builder.create_vector(&self.weights));

        flatbuffer::AnimationValues::create(
            builder,
            &flatbuffer::AnimationValuesArgs {
                translation,
                rotation,
                scale,
                weights,
            },
        )
    }
}

impl SimpleFlatbufferConversion for InterpolationTargets {
    type Output = flatbuffer::InterpolationTargets;

    fn to_flatbuffer(&self) -> Self::Output {
        // We convert to i8's because this is just an enum and this is the smallest value...
        flatbuffer::InterpolationTargets::new(
            flatbuffer::InterpolationTypes(self.translation as i8),
            flatbuffer::InterpolationTypes(self.rotation as i8),
            flatbuffer::InterpolationTypes(self.scale as i8),
            flatbuffer::InterpolationTypes(self.weights as i8),
        )
    }
}

#[derive(Clone, Debug)]
pub struct Animation {
    /// ID of the target node
    pub target: usize,
    pub interpolation: InterpolationTargets,

    pub frames: Vec<gltf_loader::GLTFAnimationFrame>,

    /// Duration of the entire animation in seconds
    pub duration: f32,
}

impl Animation {
    fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<flatbuffer::Animation<'a>> {
        let (target_type, target) = (flatbuffer::AnimationTargetTypes::Object, self.target);

        let frames: Vec<_> = self
            .frames
            .iter()
            .map(|frame| frame.to_flatbuffer(builder))
            .collect();
        let frames = builder.create_vector(&frames);

        flatbuffer::Animation::create(
            builder,
            &flatbuffer::AnimationArgs {
                target: target as u64,
                target_type,
                interpolation: Some(&self.interpolation.to_flatbuffer()),
                frames: Some(frames),
                duration: self.duration,
            },
        )
    }
}

// Animation associated with a name
// It's weirdly divided since this used to be able
// to hold multiple animation in a single set
// but I removed that for simplicity
#[derive(Clone, Debug)]
pub struct AnimationSet {
    pub name: String,
    pub animation: Animation,
}

impl AnimationSet {
    pub fn from_node(node: &Vec<gltf_loader::GLTFAnimation>) -> Vec<AnimationSet> {
        let mut animations: Vec<AnimationSet> = Vec::with_capacity(node.len());

        for animation in node {
            let name = animation.name.clone();

            let target: usize = animation.target;

            let mut frames: Vec<gltf_loader::GLTFAnimationFrame> = animation.frames.clone();
            frames
                .par_iter_mut()
                .for_each(|item| item.value.transformation.as_renpy_coords(true));

            animations.push(AnimationSet {
                name,
                animation: crate::Animation {
                    target,
                    interpolation: animation.interpolation,
                    frames,
                    duration: animation.duration,
                },
            });
        }

        animations
    }
}

impl FlatbufferConversion for AnimationSet {
    type Output<'a> = gltf_for_renpy_flatbuffer::AnimationSet<'a>;

    fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<Self::Output<'a>> {
        let name = Some(builder.create_string(&self.name));

        let animation = Some(self.animation.to_flatbuffer(builder));
        gltf_for_renpy_flatbuffer::AnimationSet::create(
            builder,
            &gltf_for_renpy_flatbuffer::AnimationSetArgs {
                name,
                animations: animation,
            },
        )
    }
}

use std::f32;

use cgmath::{Vector3, Vector4, VectorSpace};
use gltf::{
    Animation, Node,
    animation::{Interpolation, Reader},
};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use rustc_hash::FxHashMap;

use crate::utils::{DecomposedTransform, GltfData, RotationTransform};

/// Animation
#[derive(Debug, Clone)]
pub struct GLTFAnimation {
    /// Name of the animation
    pub name: String,

    /// The node to target         
    pub target: usize,

    /// Animation Key Frames
    pub frames: Vec<GLTFAnimationFrame>,

    /// Interpolation Style Between Keyframes     
    pub interpolation: InterpolationTargets,

    /// How long the animation lasts
    pub duration: f32,
}

/// Types of interpolation to use for the specific channel
#[derive(Clone, Copy, Debug, Default)]
pub enum InterpolationTypes {
    #[default]
    /// No interpolation is specified
    None,
    /// Step
    Step,
    /// Linear
    Linear,
    /// Cubic
    Cubic,
}

/// How Interpolation is applied for each animated property
#[derive(Copy, Clone, Debug, Default)]
pub struct InterpolationTargets {
    /// Translation Interpolation
    pub translation: InterpolationTypes,
    /// Rotation Interpolation
    pub rotation: InterpolationTypes,
    /// Scale Interpolation
    pub scale: InterpolationTypes,
    /// Weights Interpolation
    pub weights: InterpolationTypes,
}

impl InterpolationTypes {
    /// Convert from gltf crate type to native type
    pub fn convert(original: Interpolation) -> Self {
        match original {
            Interpolation::Linear => InterpolationTypes::Linear,
            Interpolation::Step => InterpolationTypes::Step,
            Interpolation::CubicSpline => InterpolationTypes::Cubic,
        }
    }
}

type AnimationFrameTimes = Vec<f32>;

/// Collected transformed values of a channel
pub enum GLTFAnimationRawValue {
    /// XYZ Translation     
    Translation(AnimationFrameTimes, Vec<Vector3<f32>>),
    /// XYZW Rotation
    Rotation(AnimationFrameTimes, Vec<Vector4<f32>>),
    /// XYZ Scaling
    Scaling(AnimationFrameTimes, Vec<Vector3<f32>>),
    /// Value of All Morph Target Weights
    MorphWeights(AnimationFrameTimes, Vec<Vec<f32>>),
}

impl<'a> GLTFAnimationRawValue {
    fn new<F: Clone + Fn(gltf::Buffer<'a>) -> Option<&'a [u8]>>(
        channel_reader: Reader<'a, 'a, F>,
    ) -> Self {
        let frame_times: Vec<f32>;

        if let Some(input_time) = channel_reader.read_inputs() {
            frame_times = input_time.collect();
        } else {
            frame_times = Vec::new();
        }

        // Each channel output the entire animation values for that property
        if let Some(output_values) = channel_reader.read_outputs() {
            match output_values {
                gltf::animation::util::ReadOutputs::Translations(translations) => {
                    Self::Translation(frame_times, translations.map(Vector3::from).collect())
                }
                gltf::animation::util::ReadOutputs::Scales(scales) => {
                    Self::Scaling(frame_times, scales.map(Vector3::from).collect())
                }
                gltf::animation::util::ReadOutputs::Rotations(rotations) => {
                    let rotations = rotations.into_f32();
                    Self::Rotation(frame_times, rotations.map(Vector4::from).collect())
                }
                gltf::animation::util::ReadOutputs::MorphTargetWeights(weights) => {
                    let weights: Vec<f32> = weights.into_f32().collect();

                    let weight_num = weights.len().div_ceil(frame_times.len());
                    let weight_iter = weights.into_iter().chunks(weight_num);
                    let weights = weight_iter
                        .into_iter()
                        .map(|chunk| {
                            let mut chunk = chunk.collect_vec();
                            chunk.resize_with(4, || 0.0);

                            chunk
                        })
                        .collect();

                    Self::MorphWeights(frame_times, weights)
                }
            }
        } else {
            unreachable!(
                "This should not be possible... I don't like using panic but this is supposed to be infallible"
            )
        }
    }
}

/// The actual value stored in the animation frames
#[derive(Clone, Debug, Default)]
pub struct GLTFAnimationValue {
    /// The transformation values for the animation
    pub transformation: DecomposedTransform,

    /// Value of All Morph Target Weights
    pub weights: Vec<f32>,
}

impl GLTFAnimationValue {
    fn from_node_defaults(default_node: &Node) -> Self {
        let (translation, rotation, scale) = default_node.transform().decomposed();

        let mut res = GLTFAnimationValue {
            transformation: DecomposedTransform {
                translation: translation.into(),
                rotation: rotation.into(),
                scale: scale.into(),
            },
            ..Default::default()
        };

        if let Some(weights) = default_node.weights() {
            res.weights.extend_from_slice(weights);
        }

        res
    }

    /// Creates a new frame value from a previous frame
    pub fn from_previous_frame(last_frame: &GLTFAnimationFrame) -> Self {
        let value = &last_frame.value;

        Self {
            transformation: DecomposedTransform {
                translation: value.transformation.translation,
                rotation: value.transformation.rotation.clone(),
                scale: value.transformation.scale,
            },
            weights: value.weights.clone(),
        }
    }
}

/// Animation Frames
#[derive(Default, Debug, Clone)]
pub struct GLTFAnimationFrame {
    /// Time
    pub time: f32,

    /// Value
    pub value: GLTFAnimationValue,
}

/// Simplified Animation Channel from GLTF
struct AnimationDataIterator {
    index: usize,
    is_finished: bool,
    interpolation_type: InterpolationTypes,
    data: GLTFAnimationRawValue,
    default_data: GLTFAnimationValue,
}

impl AnimationDataIterator {
    // Get the current time value if we can
    fn peek_time(&self) -> Option<f32> {
        match &self.data {
            GLTFAnimationRawValue::Translation(times, ..) => times.get(self.index).copied(),
            GLTFAnimationRawValue::Rotation(times, ..) => times.get(self.index).copied(),
            GLTFAnimationRawValue::Scaling(times, ..) => times.get(self.index).copied(),
            GLTFAnimationRawValue::MorphWeights(times, ..) => times.get(self.index).copied(),
        }
    }

    // Modify the next frame value based off of the data in the channel
    fn next(&mut self, anim_val: &mut GLTFAnimationValue, new_frame_time: f32) {
        /// Check if we should interpolate based off if the time matches or not
        // This is for situation like
        // Chan 1: 1.0 --------------------->  5.0
        // Chan 2: 1.0 -> 2.0 -> 3.0 -> 4.0 -> 5.0
        // Since I just want a single animation timeline I have to interpolate it
        // To bring all the channel to the lowest common denominator
        fn should_interpolate(
            this: &AnimationDataIterator,
            times: &[f32],
            new_time: f32,
        ) -> Option<f32> {
            if let Some(cur_time) = times.get(this.index)
                && *cur_time != new_time
            {
                return Some(*cur_time);
            }

            None
        }

        if !self.is_finished {
            let max_len;

            macro_rules! impl_raw_data_getter {
                ($times: ident, $item: ident, $( $data_type:ident ).+, $linear_func:path) => {
                    let new_val = if let Some(new_val) = $item.get(self.index){
                        new_val.clone()
                    }
                    else{
                        // Early Return Since There's No More Data Left
                        // This should be unreachable but it's best not to panic since this can be used within FFI
                        self.is_finished = true;
                        return;
                    };

                    if let Some(frame_time) = should_interpolate(self, $times, new_frame_time){
                        // This works since you can think of this as:
                        // start_time = 0
                        // duration = (start_time - new_frame_time)
                        // interp_amount = (start_time - frame_time) / duration
                        // Basically just a simple modification of the segment-normalized interpolation factor.
                        let interp_amount = frame_time / new_frame_time;

                        anim_val.$($data_type).+ = match self.interpolation_type {
                            InterpolationTypes::None => {
                               // Next val is probably a sane default for none even though this should not happen tbh
                               new_val.into()
                            },
                            // Step is just return the previous value since that make the most sense
                            InterpolationTypes::Step => anim_val.$($data_type).+.clone(),
                            InterpolationTypes::Linear => {
                                $linear_func(anim_val.$($data_type).+.clone() , new_val, interp_amount)
                            },
                            InterpolationTypes::Cubic => {
                                // I am not sure how you are supposed to this with the GLTF crate???
                                todo!("Cubic Types Are Not Supported")
                            },
                        };
                    }
                    else {
                        // If we don't interpolate then that means it's our time to set the next frame value
                        anim_val.$($data_type).+ = new_val.into();
                    }

                    max_len = $times.len();
                };
            }

            // The actual state machine to modify the frame value is above this btw
            // Using macro since it just the same shit with little modification and I wanted to use one
            match &self.data {
                GLTFAnimationRawValue::Translation(times, item) => {
                    impl_raw_data_getter!(
                        times,
                        item,
                        transformation.translation,
                        VectorSpace::lerp
                    );
                }
                GLTFAnimationRawValue::Rotation(times, item) => {
                    impl_raw_data_getter!(
                        times,
                        item,
                        transformation.rotation,
                        RotationTransform::slerp
                    );
                }
                GLTFAnimationRawValue::Scaling(times, item) => {
                    impl_raw_data_getter!(times, item, transformation.scale, VectorSpace::lerp);
                }
                GLTFAnimationRawValue::MorphWeights(times, item) => {
                    fn interpolate_weights(
                        orig_val: Vec<f32>,
                        new_val: Vec<f32>,
                        amount: f32,
                    ) -> Vec<f32> {
                        orig_val
                            .iter()
                            .zip(new_val)
                            .map(|(old_val, new_val)| {
                                (1.0 - amount) * (*old_val) + amount * (new_val)
                            })
                            .collect()
                    }

                    impl_raw_data_getter!(times, item, weights, interpolate_weights);
                }
            }

            // Increment the index and check if we finish or not
            // It probably not the most rusty way to do this but ehh...
            // It works and I had to go through so many iteration to get this shit out
            self.index += 1;
            if self.index >= max_len {
                self.is_finished = true;
            }
        }
    }
}

impl GLTFAnimation {
    /// Load the animation from the GLTF file
    pub fn load(animation: Animation, data: &GltfData) -> Vec<(usize, Self)> {
        let buffers = &data.buffers;

        let name = animation.name().unwrap_or_default().to_owned();
        let mut nodes_channels: FxHashMap<usize, Vec<AnimationDataIterator>> = FxHashMap::default();

        // First load all the channel info grouped by the node they modified to speed up animation creation later
        for animation_channel in animation.channels() {
            let target_node = animation_channel.target().node();
            let target_id = target_node.index();

            let interpolation_type =
                InterpolationTypes::convert(animation_channel.sampler().interpolation());

            let channel_reader = animation_channel.reader(|buffer| Some(&buffers[buffer.index()]));

            let channel_entry = nodes_channels.entry(target_id).or_default();

            channel_entry.push(AnimationDataIterator {
                index: 0,
                is_finished: false,
                interpolation_type,
                data: GLTFAnimationRawValue::new(channel_reader),
                default_data: GLTFAnimationValue::from_node_defaults(&target_node),
            });
        }

        let mut animations: FxHashMap<usize, GLTFAnimation> = FxHashMap::default();

        // Iterate over each node that participate in the current animation using the groups set up beforehand
        for (node_id, mut animation_channels) in nodes_channels {
            let animation_entry = animations.entry(node_id).or_insert(GLTFAnimation {
                name: name.clone(),
                target: node_id,
                frames: Vec::new(),
                interpolation: InterpolationTargets::default(),
                duration: 0.0,
            });

            let mut frames: Vec<GLTFAnimationFrame> = Vec::with_capacity(animation_channels.len());

            /// Find the next frame out of all the channels that has the smallest delta between the last frame (basically find the next frame)
            // The trick is that since this will be the same for all channels
            // we can do this stateless and just check if it matches as time is monotonic
            fn min_value(
                animation_channels: &[AnimationDataIterator],
            ) -> Option<(&AnimationDataIterator, f32)> {
                animation_channels
                    .iter()
                    .filter(|val| !val.is_finished)
                    .min_by_key(|val| {
                        OrderedFloat(if let Some(x) = val.peek_time() {
                            x
                        } else {
                            // This theoretically should not be possible since we filtered out finished ones
                            // but just in case we just return the largest time value so that it's impossible to be the minimum.
                            f32::MAX
                        })
                    })
                    .map(|iter| (iter, iter.peek_time().unwrap()))
            }

            // We loop through all the channels to find the next frame data
            // If we get a none it means all the data has been squeezed out!
            while let Some(min_iter) = min_value(&animation_channels) {
                // Copy it to please the borrow checker
                let min_val = min_iter.1;

                // Init the next frame since we will be modifying it instead of creating it at the end
                // This is where the default value from the node comes in since we have to maintain that in the animation
                let mut next_frame = if frames.is_empty() {
                    GLTFAnimationFrame {
                        time: min_val,
                        value: min_iter.0.default_data.clone(),
                    }
                } else {
                    GLTFAnimationFrame {
                        time: min_val,
                        value: GLTFAnimationValue::from_previous_frame(
                            frames
                                .last()
                                .expect("We already checked that frames is not empty."),
                        ),
                    }
                };

                for chan in &mut animation_channels {
                    // This is a bit wasteful to do in a loop since it only needs to be done once, but it's fine since it's so fast tbh...
                    match &chan.data {
                        GLTFAnimationRawValue::Translation(..) => {
                            animation_entry.interpolation.translation = chan.interpolation_type;
                        }
                        GLTFAnimationRawValue::Rotation(..) => {
                            animation_entry.interpolation.rotation = chan.interpolation_type;
                        }
                        GLTFAnimationRawValue::Scaling(..) => {
                            animation_entry.interpolation.scale = chan.interpolation_type;
                        }
                        GLTFAnimationRawValue::MorphWeights(..) => {
                            animation_entry.interpolation.weights = chan.interpolation_type;
                        }
                    }

                    // Modify the next frame based off the 'next frame time' collected
                    chan.next(&mut next_frame.value, min_val);
                }

                frames.push(next_frame);
            }

            // Duration is just what the last frame time since the time is relative to zero and is monotonic!
            if let Some(latest_frame) = frames.last() {
                animation_entry.duration = latest_frame.time;
            }

            animation_entry.frames = frames;
        }

        // Come and mop up boys, I am done here
        animations.drain().collect_vec()
    }
}

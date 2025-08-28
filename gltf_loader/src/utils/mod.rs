mod gltf_data;

pub use gltf_data::GltfData;

use cgmath::*;
use core::{f32, f64};
use gltf::json::Value;
use gltf::scene::Transform;
use std::collections::HashMap;
use std::ops::{Add, Mul, Neg};

/// Rotation Transformation
#[derive(Clone, Debug)]
pub enum RotationTransform {
    /// Rotation in Quaternion
    Quaternion(Quaternion<f32>),
    /// Rotation in Euler Degrees
    Euler(Euler<Deg<f32>>),
}

/// ID for an object in a scene
#[derive(Copy, Clone, Debug)]
pub enum GlobalNodeIdentifier {
    /// The scene root
    SceneRoot,
    /// The node id provided by the GLTF file
    NodeId(usize),
    /// Index in the final Object Array
    ObjectIndex(usize),
}

impl std::fmt::Display for GlobalNodeIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl RotationTransform {
    /// Get Rotation Transform as Euler Degree
    pub fn to_euler(self) -> Self {
        match self {
            RotationTransform::Quaternion(quaternion) => {
                RotationTransform::Euler(quaterions_to_euler(quaternion))
            }
            RotationTransform::Euler(_) => self,
        }
    }

    /// Get Rotation Transform as_quaternion
    pub fn to_quaternion(self) -> Self {
        match self {
            RotationTransform::Quaternion(_) => self,
            RotationTransform::Euler(euler) => RotationTransform::Quaternion(euler.into()),
        }
    }

    /// Get inner value as Euler Degree
    pub fn unwrap_euler(self) -> Euler<Deg<f32>> {
        match self {
            RotationTransform::Quaternion(quaternion) => quaterions_to_euler(quaternion),
            RotationTransform::Euler(euler) => euler,
        }
    }

    /// Get inner value as quaternion
    pub fn unwrap_quaternion(self) -> Quaternion<f32> {
        match self {
            RotationTransform::Quaternion(quat) => quat,
            RotationTransform::Euler(euler) => euler.into(),
        }
    }

    /// Reexport Slerp
    pub fn slerp(self, other: Vector4<f32>, amount: f32) -> RotationTransform {
        RotationTransform::Quaternion(
            self.to_quaternion()
                .unwrap_quaternion()
                .slerp(Quaternion::new(other.w, other.x, other.y, other.z), amount),
        )
    }
}

impl From<Vector4<f32>> for RotationTransform {
    fn from(value: Vector4<f32>) -> Self {
        RotationTransform::Quaternion(Quaternion::new(value.w, value.x, value.y, value.z))
    }
}

impl From<[f32; 4]> for RotationTransform {
    fn from(value: [f32; 4]) -> Self {
        RotationTransform::Quaternion(value.into())
    }
}

impl Mul for RotationTransform {
    type Output = RotationTransform;

    fn mul(self, rhs: Self) -> Self::Output {
        let lhs = match self {
            RotationTransform::Quaternion(quaternion) => quaternion,
            RotationTransform::Euler(euler) => euler.into(),
        };

        let rhs = match rhs {
            RotationTransform::Quaternion(quaternion) => quaternion,
            RotationTransform::Euler(euler) => euler.into(),
        };

        let res = lhs * rhs;

        match self {
            RotationTransform::Quaternion(_) => RotationTransform::Quaternion(res),
            RotationTransform::Euler(_) => RotationTransform::Euler(quaterions_to_euler(res)),
        }
    }
}

/// A decomposed tranform matrix
#[derive(Clone, Debug)]
pub struct DecomposedTransform {
    /// `[x, y, z]` vector.
    pub translation: Vector3<f32>,

    /// `[x, y, z, w]` quaternion, where `w` is the scalar.
    /// or in Euler Degree
    pub rotation: RotationTransform,

    /// `[x, y, z]` vector.
    pub scale: Vector3<f32>,
}

impl Add for DecomposedTransform {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        DecomposedTransform {
            translation: self.translation + rhs.translation,
            rotation: self.rotation * rhs.rotation,
            scale: self.scale.zip(rhs.scale, |lhs, rhs| lhs * rhs),
        }
    }
}

impl DecomposedTransform {
    /// is this transform the identity transform (does nothing)
    pub fn is_default(&self) -> bool {
        let default_rotation = match self.rotation {
            RotationTransform::Quaternion(quaternion) => quaternion.is_one(),
            RotationTransform::Euler(euler) => {
                euler.x.is_zero() && euler.y.is_zero() && euler.z.is_zero()
            }
        };

        self.translation.is_zero()
            && default_rotation
            && self.scale.eq(&Vector3::new(1.0, 1.0, 1.0))
    }

    /// Convert translation gltf coords to renpy coords
    pub fn to_renpy_coords(mut self, keep_rotation_format: bool) -> Self {
        self.as_renpy_coords(keep_rotation_format);
        self
    }

    /// Convert gltf transform coords to renpy coords in place
    pub fn as_renpy_coords(&mut self, keep_rotation_format: bool) {
        self.translation.y = self.translation.y.neg();

        // Rotation has to be changed since renpy uses clockwise ZYX euler angles while GLTF uses counterclockwise quaternions
        self.rotation = match self.rotation {
            RotationTransform::Quaternion(quaternion) => {
                // Apperntly you can just convert quaternion to XYZ by just mutiplying the real part by -1 but something keeps going wrong...
                let rot = Quaternion::new(
                    quaternion.s,
                    -quaternion.v.x,
                    quaternion.v.y,
                    -quaternion.v.z,
                );

                // I still have to convert euler to quaternion since renpy uses zyx and not xyz euler...
                // Except for animations because I have to spherical lerp lmao
                if keep_rotation_format {
                    RotationTransform::Quaternion(rot)
                } else {
                    RotationTransform::Euler(quaterions_to_zyx_euler(rot))
                }
            }
            RotationTransform::Euler(euler) => {
                let rot = euler.into();

                // Renpy uses ZYX (or something similar I think) euler angles instead of XYZ angles we were using so we need to do some simple conversion
                let mut rot = quaterions_to_zyx_euler(rot);
                // We have to negate the x and z angles for some reason? (I think it's because renpy uses clockwise instead of counterclockwise angles)
                rot.x.0 = rot.x.0.neg();
                rot.z.0 = rot.z.0.neg();

                // We use euler because renpy uses euler for everything so it make the python code much simpler
                RotationTransform::Euler(rot)
            }
        };
    }

    /// Converts a tranform from the gltf crate into this type
    pub fn convert_from_gltf(intial: Transform) -> Self {
        let (translation, rotation, scale) = intial.decomposed();
        DecomposedTransform {
            translation: Vector3::from(translation),
            rotation: RotationTransform::Quaternion(Quaternion::from(rotation)),
            scale: Vector3::from(scale),
        }
    }
}
impl From<DecomposedTransform> for Matrix4<f32> {
    fn from(value: DecomposedTransform) -> Self {
        let rot = value.rotation.to_quaternion();
        let rot = match rot {
            RotationTransform::Quaternion(quaternion) => quaternion,
            RotationTransform::Euler(_) => unreachable!("wtf??"),
        };

        Matrix4::from(
            Transform::Decomposed {
                translation: value.translation.into(),
                rotation: rot.into(),
                scale: value.scale.into(),
            }
            .matrix(),
        )
    }
}

impl Default for DecomposedTransform {
    fn default() -> Self {
        DecomposedTransform {
            translation: Vector3::zero(),
            rotation: RotationTransform::Quaternion(Quaternion::one()),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

/// GLTF Transform into a 4x4 Matrix
pub fn transform_to_matrix(transform: Transform) -> Matrix4<f32> {
    let tr = transform.matrix();
    Matrix4::new(
        tr[0][0], tr[0][1], tr[0][2], tr[0][3], tr[1][0], tr[1][1], tr[1][2], tr[1][3], tr[2][0],
        tr[2][1], tr[2][2], tr[2][3], tr[3][0], tr[3][1], tr[3][2], tr[3][3],
    )
}

// FIXME: Maybe create a macro for Extra Conversion?
#[macro_export]
/// Simple macro to get extras by duplicating boilerplate code
macro_rules! get_extras {
    ($provider: ident) => {
        $provider.extras().clone().and_then(|extra_data| {
            let extra_data = extra_data.get();
            if extra_data.is_empty() {
                None
            } else {
                gltf::json::deserialize::from_str::<gltf::json::Value>(extra_data)
                    .ok()
                    .and_then(|map| convert_extra(&map))
            }
        })
    };
}

/// Converts GLTF extra properties into a string hashmap
pub fn convert_extra(extra: &Value) -> Option<HashMap<String, String>> {
    if extra.is_object() {
        let mut extras: HashMap<String, String> = HashMap::new();
        let map = extra.as_object().unwrap();
        for (key, value) in map {
            extras.insert(key.clone(), convert_json_map_object(value));
        }

        return Some(extras);
    }

    None
}

/// Converts Rad Quaterions to Euler Degree Angles
pub fn quaterions_to_euler<T: BaseFloat>(quat: Quaternion<T>) -> Euler<Deg<T>> {
    let rotation = cgmath::Euler::from(quat);

    Euler {
        x: Into::<Deg<T>>::into(rotation.x),
        y: Into::<Deg<T>>::into(rotation.y),
        z: Into::<Deg<T>>::into(rotation.z),
    }
}

/// Converts ZYX Euler Degree Angles to Quaternions
/// Based off of renpy code
pub fn euler_zyx_to_quaterions<T: BaseFloat>(euler_angles: Euler<T>) -> Quaternion<T> {
    let half: T = num_traits::cast(0.5).unwrap();
    let three_sixty: T = num_traits::cast(360).unwrap();

    let (mut old_x, mut old_y, mut old_z) = (euler_angles.x, euler_angles.y, euler_angles.z);
    old_x %= three_sixty;
    old_y %= three_sixty;
    old_z %= three_sixty;

    let old_x_div_2 = Into::<Rad<T>>::into(Deg(old_x)) * half;
    let old_y_div_2 = Into::<Rad<T>>::into(Deg(old_y)) * half;
    let old_z_div_2 = Into::<Rad<T>>::into(Deg(old_z)) * half;

    let cx = old_x_div_2.cos();
    let sx = old_x_div_2.sin();
    let cy = old_y_div_2.cos();
    let sy = old_y_div_2.sin();
    let cz = old_z_div_2.cos();
    let sz = old_z_div_2.sin();

    let xi = sx * cy * cz - cx * sy * sz;
    let yj = cx * sy * cz + sx * cy * sz;
    let zk = cx * cy * sz - sx * sy * cz;
    let w = cx * cy * cz + sx * sy * sz;

    Quaternion::new(w, xi, yj, zk)
}

/// Converts Rad Quaterions to ZYX Euler Degree Angles that Renpy uses???
/// This is basically just copied from wikipedia and the cgmath crate and other random sources
/// https://en.wikipedia.org/wiki/Conversion_between_quaternions_and_Euler_angles#Quaternion_to_Euler_angles_(in_3-2-1_sequence)_conversion
pub fn quaterions_to_zyx_euler<T: BaseFloat>(quat: Quaternion<T>) -> Euler<Deg<T>> {
    let quat = quat.normalize();

    let two: T = num_traits::cast(2.0).unwrap();
    let one: T = num_traits::cast(1.0).unwrap();
    let zero: T = num_traits::cast(0.0).unwrap();

    // Deconstruct the quaternion values
    let (qw, qx, qy, qz) = (quat.s, quat.v.x, quat.v.y, quat.v.z);
    // Compute the values squared
    let (sqx, sqy, sqz) = (qx * qx, qy * qy, qz * qz);

    // Intermediate terms
    // Clamping the pitch angle to avoid exceeding the range [-1, 1] due to precision errors

    let sin_r_cos_p = (two * (qw * qx + qy * qz)).clamp(one.neg(), one);
    let cos_r_cos_p = one - two * (sqx + sqy);
    let sin_p = two * (qw * qy - qz * qx);

    let roll = T::atan2(sin_r_cos_p, cos_r_cos_p);
    let pitch = T::asin(sin_p);
    let yaw = if sin_r_cos_p.abs() >= one {
        zero
    } else {
        let sin_y_cos_p = two * (qw * qz + qx * qy);
        let cos_y_cos_p = one - two * (sqy + sqz);
        T::atan2(sin_y_cos_p, cos_y_cos_p)
    };

    Euler {
        x: Into::<Deg<T>>::into(Rad(roll)),
        y: Into::<Deg<T>>::into(Rad(pitch)),
        z: Into::<Deg<T>>::into(Rad(yaw)),
    }
}

/// Converts Rad Quaterions to ZYX Euler Degree Angles copied straight from renpy source code
pub fn quaterions_to_zyx_euler2<T: BaseFloat>(quat: Quaternion<T>) -> Euler<Deg<T>> {
    let quat = quat.normalize();

    let two: T = num_traits::cast(2).unwrap();
    let one: T = num_traits::cast(1).unwrap();
    let zero: T = num_traits::cast(0.0).unwrap();
    let pi: T = num_traits::cast(f64::consts::PI).unwrap();

    // Deconstruct the quaternion values
    let (qw, qx, qy, qz) = (quat.s, quat.v.x, quat.v.y, quat.v.z);

    let sinx_cosp = two * (qw * qx + qy * qz);
    let cosx_cosp = one - two * (qx * qx + qy * qy);
    let mut siny = two * (qw * qy - qz * qx);
    let sinz_cosp1 = two * (qx * qy - qw * qz);
    let cosz_cosp1 = one - two * (qx * qx + qz * qz);
    let sinz_cosp2 = two * (qw * qz + qx * qy);
    let cosz_cosp2 = one - two * (qy * qy + qz * qz);

    let (x, y, z);
    if siny >= one {
        x = zero;
        y = pi / two;
        z = sinz_cosp1.atan2(cosz_cosp1);
    } else if siny <= -one {
        x = zero;
        y = -pi / two;
        z = sinz_cosp1.atan2(cosz_cosp1);
    } else {
        x = sinx_cosp.atan2(cosx_cosp);
        if siny > one {
            siny = one;
        } else if siny < -one {
            siny = -one;
        }
        y = siny.asin();
        z = sinz_cosp2.atan2(cosz_cosp2);
    }
    Euler {
        x: Into::<Deg<T>>::into(Rad(x)),
        y: Into::<Deg<T>>::into(Rad(y)),
        z: Into::<Deg<T>>::into(Rad(z)),
    }
}

fn convert_json_map_object(value: &Value) -> String {
    match value {
        Value::Null => "".to_owned(),
        Value::Bool(val) => val.to_string(),
        Value::Number(val) => val.to_string(),
        Value::String(val) => format!("\"{}\"", val.to_owned()),
        Value::Array(_) => value.to_string(),
        Value::Object(_) => value.to_string(),
    }
}

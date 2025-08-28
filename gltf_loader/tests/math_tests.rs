use cgmath::{AbsDiffEq, Deg, Euler, Quaternion};
use gltf_loader::utils::{
    euler_zyx_to_quaterions, quaterions_to_zyx_euler, quaterions_to_zyx_euler2,
};

#[test]
fn quaterions_to_zyx_euler_test() {
    let quat = Quaternion::new(0.8660254, 0.0, 0.5, 0.0);
    let result = quaterions_to_zyx_euler(quat);
    let checked = Euler::new(Deg(0.0), Deg(60.0), Deg(0.0));
    assert!(result.abs_diff_eq(&checked, 1.0));

    let quat = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    let result = quaterions_to_zyx_euler(quat);
    let checked = Euler::new(Deg(0.0), Deg(0.0), Deg(0.0));
    assert!(result.abs_diff_eq(&checked, 1.0));

    let quat = Quaternion::new(0.5609855, 0.4304593, 0.092296, 0.7010574);
    let result = quaterions_to_zyx_euler(quat);
    let checked = Euler::new(Deg(45.0), Deg(-30.0), Deg(90.0));
    assert!(result.abs_diff_eq(&checked, 1.0));

    let quat = Quaternion::new(0.9601763, -0.1866394, -0.0396714, -0.2040918);
    let result = quaterions_to_zyx_euler(quat);
    let checked = Euler::new(Deg(-20.2589182), Deg(-8.7640775), Deg(-22.4312696));
    assert!(result.abs_diff_eq(&checked, 1.0));

    let quat = Quaternion::new(0.6762096, 0.206738, 0.206738, 0.6762096);
    let result = quaterions_to_zyx_euler(quat);
    let checked = Euler::new(Deg(34.0), Deg(0.0), Deg(90.0));
    assert!(result.abs_diff_eq(&checked, 1.0));

    let quat = Quaternion::new(-0.7294356, 0.5182009, -0.442959, -0.0563824);
    let result = quaterions_to_zyx_euler(quat);
    let checked = Euler::new(Deg(-84.2969062), Deg(44.8016636), Deg(-32.0785351));
    assert!(result.abs_diff_eq(&checked, 1.0));

    let quat = Euler::new(Deg(45.0), Deg(45.0), Deg(45.0));
    let result = quaterions_to_zyx_euler(quat.into());
    let checked = Euler::new(Deg(59.6388073), Deg(-8.4210585), Deg(59.6388073));
    assert!(result.abs_diff_eq(&checked, 1.0));
}

#[test]
fn quaterions_to_zyx_euler_test2() {
    let quat = Quaternion::new(0.8660254, 0.0, 0.5, 0.0);
    let result = quaterions_to_zyx_euler(quat);
    let result2 = quaterions_to_zyx_euler2(quat);
    assert!(result.abs_diff_eq(&result2, 1.0));

    let quat = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    let result = quaterions_to_zyx_euler(quat);
    let checked = quaterions_to_zyx_euler2(quat);
    assert!(result.abs_diff_eq(&checked, 1.0));

    let quat = Quaternion::new(0.5609855, 0.4304593, 0.092296, 0.7010574);
    let result = quaterions_to_zyx_euler(quat);
    let checked = quaterions_to_zyx_euler2(quat);
    assert!(result.abs_diff_eq(&checked, 1.0));

    let quat = Quaternion::new(0.9601763, -0.1866394, -0.0396714, -0.2040918);
    let result = quaterions_to_zyx_euler(quat);
    let checked = quaterions_to_zyx_euler2(quat);
    assert!(result.abs_diff_eq(&checked, 1.0));

    let quat = Quaternion::new(0.6762096, 0.206738, 0.206738, 0.6762096);
    let result = quaterions_to_zyx_euler(quat);
    let checked = quaterions_to_zyx_euler2(quat);
    assert!(result.abs_diff_eq(&checked, 1.0));

    let quat = Quaternion::new(-0.7294356, 0.5182009, -0.442959, -0.0563824);
    let result = quaterions_to_zyx_euler(quat);
    let checked = quaterions_to_zyx_euler2(quat);
    assert!(result.abs_diff_eq(&checked, 1.0));

    let quat = Euler::new(Deg(380.0), Deg(30.0), Deg(1.1));
    let result = quaterions_to_zyx_euler(quat.into());
    let checked = quaterions_to_zyx_euler2(quat.into());
    assert!(result.abs_diff_eq(&checked, 0.1));

    println!("{:?}, {:?}", result, checked);
}

#[test]
fn euler_zyx_to_quaterions_test() {
    let euler = Euler::new(380.0, 30.0, 1.1);
    let result = euler_zyx_to_quaterions(euler);
    let checked = Quaternion::new(0.9516388, 0.1652768, 0.2564853, -0.0358102);
    println!("{:?}, {:?}", result, checked);
    assert!(result.abs_diff_eq(&checked, 0.05));

    let euler = Euler::new(45.0, 45.0, 45.0);
    let result = euler_zyx_to_quaterions(euler);
    let checked = Quaternion::new(0.8446232, 0.1913417, 0.4619398, 0.1913417);
    println!("{:?}, {:?}", result, checked);
    assert!(result.abs_diff_eq(&checked, 0.05));

    let euler = Euler::new(80.0, 0.0, 0.0);
    let result = euler_zyx_to_quaterions(euler);
    let checked = Quaternion::new(0.7660444, 0.6427876, 0.0, 0.0);
    println!("{:?}, {:?}", result, checked);
    assert!(result.abs_diff_eq(&checked, 0.05));

    let euler = Euler::new(-84.2969062, 44.8016636, -32.0785351);
    let result = euler_zyx_to_quaterions(euler);
    let checked = Quaternion::new(0.7294356, -0.5182009, 0.4429589, 0.0563824);
    println!("{:?}, {:?}", result, checked);
    assert!(result.abs_diff_eq(&checked, 0.05));

    let euler = Euler::new(-20.258918, -8.76407, -22.43126);
    let result = euler_zyx_to_quaterions(euler);
    let checked = Quaternion::new(0.9396137, -0.1893143, -0.0238366, -0.2841091);
    println!("{:?}, {:?}", result, checked);
    assert!(result.abs_diff_eq(&checked, 0.1));

    let euler = Euler::new(0.0, 0.0, 0.0);
    let result = euler_zyx_to_quaterions(euler);
    let checked = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    println!("{:?}, {:?}", result, checked);
    assert!(result.abs_diff_eq(&checked, 0.05));
}

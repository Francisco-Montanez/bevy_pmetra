use bevy::{color::palettes::css, math::DVec3, prelude::*};
use bevy_pmetra::{
    math::get_rotation_from_normals,
    pmetra_core::extensions::shell::ShellCadExtension,
    prelude::*,
    re_exports::{
        anyhow::{anyhow, Result},
        truck_modeling::{builder, control_point::ControlPoint, Shell, Vector3, Vertex},
    },
};

use super::{CadShellIds, SimpleCubeAtCylinder};

pub fn build_cube_shell(params: &SimpleCubeAtCylinder) -> Result<CadShell> {
    let SimpleCubeAtCylinder {
        cube_side_length, ..
    } = params.clone();

    let mut tagged_elements = CadTaggedElements::default();

    let v0 = Vertex::new(
        (DVec3::new(-cube_side_length / 2., 0., cube_side_length / 2.))
            .to_array()
            .into(),
    );
    let v1 = Vertex::new(
        (DVec3::new(cube_side_length / 2., 0., cube_side_length / 2.))
            .to_array()
            .into(),
    );
    tagged_elements.insert(
        CadElementTag("VertexV0".into()),
        CadElement::Vertex(v0.clone()),
    );
    tagged_elements.insert(
        CadElementTag("VertexV1".into()),
        CadElement::Vertex(v1.clone()),
    );

    let edge = builder::tsweep(&v0, v1.point().to_vec() - v0.point().to_vec());
    let face = builder::tsweep(&edge, -Vector3::unit_z() * cube_side_length);
    tagged_elements.insert(
        CadElementTag("ProfileFace".into()),
        CadElement::Face(face.clone()),
    );
    let solid = builder::tsweep(&face, (DVec3::Y * cube_side_length).to_array().into());

    let shell = Shell::try_from_solid(&solid)?;

    Ok(CadShell {
        shell,
        tagged_elements,
    })
}

pub fn cube_mesh_builder(
    params: &SimpleCubeAtCylinder,
    shell_name: CadShellName,
    shells_by_name: &CadShellsByName,
    rot_y: f32,
) -> Result<CadMeshBuilder<SimpleCubeAtCylinder>> {
    let SimpleCubeAtCylinder {
        cylinder_height, ..
    } = &params;
    // spawn entity with generated mesh...
    let Some(cylinder_solid) = shells_by_name.get(&CadShellName(CadShellIds::Cylinder.to_string()))
    else {
        return Err(anyhow!("Could not get cylinder_solid!"));
    };
    let Some(CadElement::Vertex(cylinder_v0)) = cylinder_solid
        .tagged_elements
        .get(&CadElementTag("VertexV0".into()))
    else {
        return Err(anyhow!("Could not get cylinder VertexV0!"));
    };

    let rotation = get_rotation_from_normals(Vec3::Y, Vec3::X);
    let mut transform = Transform::from_rotation(rotation).with_translation(
        cylinder_v0.point().as_bevy_vec3() + Vec3::Y * (*cylinder_height as f32 / 2.),
    );
    transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(rot_y));

    let mesh_builder = CadMeshBuilder::new(params.clone(), shell_name.clone())? // builder
        .set_transform(transform)?
        .set_base_material(Color::from(css::BLUE).into())?;

    Ok(mesh_builder)
}

pub fn build_side_length_slider(
    params: &SimpleCubeAtCylinder,
    shells_by_name: &CadShellsByName,
) -> Result<CadSlider> {
    let SimpleCubeAtCylinder {
        cube_side_length, ..
    } = &params;

    let cad_shell = shells_by_name
        .get(&CadShellName(CadShellIds::Cube.to_string()))
        .ok_or_else(|| anyhow!("Could not get cube shell!"))?;
    let Some(CadElement::Face(face)) =
        cad_shell.get_element_by_tag(CadElementTag::new("ProfileFace"))
    else {
        return Err(anyhow!("Could not find face!"));
    };
    let face_boundaries = face.boundaries();
    let face_wire = face_boundaries.last().expect("No wire found!");
    let face_centroid = face_wire.get_centroid();

    let cube_builder = cube_mesh_builder(
        params,
        CadShellName(CadShellIds::Cube.to_string()),
        shells_by_name,
        -std::f32::consts::FRAC_PI_8,
    )?;
    let mesh_transform = cube_builder.transform;

    let local_slider_pos =
        face_centroid.as_vec3() - Vec3::X * (*cube_side_length as f32 / 2. + 0.1);
    let slider_pos = mesh_transform.transform_point(local_slider_pos);
    let mut slider_transform =
        Transform::from_translation(slider_pos).with_rotation(mesh_transform.rotation);
    slider_transform.rotate_y(std::f32::consts::FRAC_PI_2);

    Ok(CadSlider {
        drag_plane_normal: *mesh_transform.up(),
        transform: slider_transform,
        slider_type: CadSliderType::Linear {
            direction: *mesh_transform.local_x(),
            limit_min: None,
            limit_max: None,
        },
        ..default()
    })
}

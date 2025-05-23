use bevy::{math::DVec3, prelude::*};
use bevy_pmetra::{
    math::get_rotation_from_normals,
    pmetra_core::extensions::shell::ShellCadExtension,
    prelude::*,
    re_exports::{
        anyhow::{anyhow, Context, Result},
        truck_modeling::{
            builder, cgmath::AbsDiffEq, ParametricSurface3D, Point3, Shell, Vector3, Vertex, Wire,
        },
    },
};
use itertools::Itertools;

use super::{CadShellIds, TowerExtension};

/// Straight Beam Shell Builder.
///
/// The beam has a L-shaped cross section.
///
/// ```ignore
/// O -> x
/// |
/// z
///
/// ad = bc = enclosure_profile_width
/// ab = dc = enclosure_profile_depth
/// o : Located at origin(0,0,0)
///
///  a --------d
///  |         |           
///  |    o    | enclosure_profile_depth
///  |         |
///  b --------c
///     enclosure_profile_width
/// ```
///
///
///
///
pub fn build_cuboid_enclosure_shell(params: &TowerExtension) -> Result<CadShell> {
    let TowerExtension {
        tower_length,
        enclosure_profile_width,
        enclosure_profile_depth,
        ..
    } = params.clone();

    let mut tagged_elements = CadTaggedElements::default();

    // Create the L-Shaped Cross section of beam...

    // Create points...
    let _o = DVec3::ZERO;
    let a = DVec3::new(
        -enclosure_profile_width / 2.,
        0.,
        -enclosure_profile_depth / 2.,
    );
    let b = a + DVec3::new(0., 0., enclosure_profile_depth);
    let c = b + DVec3::new(enclosure_profile_width, 0., 0.);
    let d = c + DVec3::new(0., 0., -enclosure_profile_depth);

    // Create wire...
    let points = [a, b, c, d];
    let vertices = points
        .iter()
        .map(|p| Vertex::new(Point3::from(p.to_array())))
        .collect::<Vec<_>>();
    let mut wire = Wire::new();
    for (v0, v1) in vertices.iter().circular_tuple_windows() {
        let edge = builder::line(v0, v1);
        wire.push_back(edge);
    }

    // Checks for wire...
    debug_assert!(wire.is_closed());

    // Extrude wire and create shell...
    let face =
        builder::try_attach_plane(&[wire]).with_context(|| "Could not attach plane to wire")?;
    let solid = builder::tsweep(&face, Vector3::from((DVec3::Y * tower_length).to_array()));
    let shell = Shell::try_from_solid(&solid)?;

    // Add tags...
    let top_face = shell
        .face_iter()
        .find(|f| {
            let normal = f.oriented_surface().normal(0.5, 0.5);
            normal.abs_diff_eq(&Vector3::unit_y(), Point3::default_epsilon())
        })
        .ok_or_else(|| anyhow!("Could not find top face!"))?;
    tagged_elements.insert(
        CadElementTag("TopFace".into()),
        CadElement::Face(top_face.clone()),
    );
    let front_face = shell
        .face_iter()
        .find(|f| {
            let normal = f.oriented_surface().normal(0.5, 0.5);
            normal.abs_diff_eq(&Vector3::unit_z(), Point3::default_epsilon())
        })
        .ok_or_else(|| anyhow!("Could not find top face!"))?;
    tagged_elements.insert(
        CadElementTag("FrontFace".into()),
        CadElement::Face(front_face.clone()),
    );

    Ok(CadShell {
        shell,
        tagged_elements,
    })
}

pub fn cuboid_enclosure_mesh_builder(
    params: &TowerExtension,
    shell_name: CadShellName,
) -> Result<CadMeshBuilder<TowerExtension>> {
    // spawn entity with generated mesh...
    let transform = Transform::default();

    let mesh_builder = CadMeshBuilder::new(params.clone(), shell_name.clone())? // builder
        .set_transform(transform)?
        .set_base_material(Color::WHITE.with_alpha(0.0).into())?;

    Ok(mesh_builder)
}

pub fn build_tower_length_slider(
    _params: &TowerExtension,
    cad_shells_by_name: &CadShellsByName,
) -> Result<CadSlider> {
    let cad_shell = cad_shells_by_name
        .get(&CadShellName(CadShellIds::CuboidEnclosure.to_string()))
        .ok_or_else(|| anyhow!("Could not get CuboidEnclosure shell!"))?;

    let Some(CadElement::Face(top_face)) =
        cad_shell.get_element_by_tag(CadElementTag::new("TopFace"))
    else {
        return Err(anyhow!("Could not find TopFace!"));
    };
    let top_face_normal = top_face.oriented_surface().normal(0.5, 0.5).as_bevy_vec3();
    let top_face_boundaries = top_face.boundaries();
    let top_face_wire = top_face_boundaries.last().expect("No wire found!");
    let top_face_centroid = top_face_wire.get_centroid();

    let Some(CadElement::Face(front_face)) =
        cad_shell.get_element_by_tag(CadElementTag::new("FrontFace"))
    else {
        return Err(anyhow!("Could not find FrontFace!"));
    };
    let front_face_normal = front_face
        .oriented_surface()
        .normal(0.5, 0.5)
        .as_bevy_vec3();

    let slider_transform =
        Transform::from_translation(top_face_centroid.as_vec3() + top_face_normal * 0.1)
            .with_rotation(get_rotation_from_normals(Vec3::Z, front_face_normal));

    Ok(CadSlider {
        drag_plane_normal: front_face_normal,
        transform: slider_transform,
        slider_type: CadSliderType::Linear {
            direction: top_face_normal,
            limit_min: None,
            limit_max: None,
        },
        ..default()
    })
}

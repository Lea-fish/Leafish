use crate::inventory::Material;

pub fn to_id(mat: Material) -> u16 {
    crate::inventory::material::versions::mapping1_12_2::to_id(mat)
}

pub fn to_material(material_id: u16) -> Material {
    crate::inventory::material::versions::mapping1_12_2::to_material(material_id)
}

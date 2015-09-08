#![feature(convert, box_patterns)]
#[macro_use]
extern crate pyramid;
extern crate cgmath;

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use cgmath::*;

use pyramid::interface::*;
use pyramid::pon::*;
use pyramid::document::*;

pub struct TransformSubSystem {
    cached_transforms: HashMap<EntityId, Matrix4<f32>>
}

impl TransformSubSystem {
    pub fn new() -> TransformSubSystem {
        TransformSubSystem {
            cached_transforms: HashMap::new()
        }
    }
    fn get_entity_transform(&mut self, system: &mut ISystem, entity_id: &EntityId) -> Matrix4<f32> {
        {
            match self.cached_transforms.get(entity_id) {
                Some(v) => return v.clone(),
                None => {}
            };
        }

        let expr = {
            (&*system.get_property_expression(&entity_id, "transform").unwrap()).clone()
        };
        let mat = {
            let mat = self.pon_to_matrix(system, entity_id, &expr);
            system.set_property(entity_id, "transformed", mat.to_pon()).unwrap();
            mat
        };

        match self.cached_transforms.entry(*entity_id) {
            Entry::Occupied(_) => unreachable!(),
            Entry::Vacant(v) => v.insert(mat).clone()
        }
    }

    fn pon_to_matrix(&mut self, system: &mut ISystem, owner: &EntityId, pon: &Pon) -> Matrix4<f32> {
        match pon {
            &Pon::TypedPon(box TypedPon { ref type_name, ref data }) => {
                match type_name.as_str() {
                    "mul" => {
                        let arr = data.translate::<&Vec<Pon>>().unwrap();
                        let mut a = Matrix4::identity();
                        for b in arr {
                            let mat = self.pon_to_matrix(system, owner, b);
                            a = a * mat;
                        }
                        return a;
                    },
                    _ => system.resolve_pon_dependencies(owner, pon).unwrap().translate().unwrap()
                }
            },
            &Pon::DependencyReference(ref named_prop_ref) => {
                if &named_prop_ref.property_key == "transform" {
                    let prop_ref = system.resolve_named_prop_ref(owner, named_prop_ref).unwrap();
                    return self.get_entity_transform(system, &prop_ref.entity_id).clone();
                }
                system.resolve_pon_dependencies(owner, pon).unwrap().translate().unwrap()
            },
            _ => system.resolve_pon_dependencies(owner, pon).unwrap().translate().unwrap()
        }
    }

}


impl ISubSystem for TransformSubSystem {

    fn on_property_value_change(&mut self, system: &mut ISystem, prop_refs: &Vec<PropRef>) {
        for pr in prop_refs.iter().filter(|pr| pr.property_key == "transform") {
            self.cached_transforms.remove(&pr.entity_id);
        }
        for pr in prop_refs.iter().filter(|pr| pr.property_key == "transform") {
            self.get_entity_transform(system, &pr.entity_id);
        }
    }
}

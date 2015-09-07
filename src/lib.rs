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
    fn get_entity_transform(&mut self, system: &mut ISystem, entity_id: &EntityId) -> Result<Matrix4<f32>, DocError> {
        {
            match self.cached_transforms.get(entity_id) {
                Some(v) => return Ok(v.clone()),
                None => {}
            };
        }

        let expr = {
            (&*try!(system.get_property_expression(&entity_id, "transform"))).clone()
        };
        let mat = {
            let mat = try!(self.pon_to_matrix(system, entity_id, &expr));
            try!(system.set_property(entity_id, "transformed", mat.to_pon()));
            mat
        };

        match self.cached_transforms.entry(*entity_id) {
            Entry::Occupied(_) => unreachable!(),
            Entry::Vacant(v) => Ok(v.insert(mat).clone())
        }
    }

    fn pon_to_matrix(&mut self, system: &mut ISystem, owner: &EntityId, pon: &Pon) -> Result<Matrix4<f32>, DocError> {
        match pon {
            &Pon::TypedPon(box TypedPon { ref type_name, ref data }) => {
                match type_name.as_str() {
                    "mul" => {
                        let arr = try!(data.translate::<&Vec<Pon>>());
                        let mut a = Matrix4::identity();
                        for b in arr {
                            let mat = try!(self.pon_to_matrix(system, owner, b));
                            a = a * mat;
                        }
                        return Ok(a);
                    },
                    _ => Ok(try!(try!(system.resolve_pon_dependencies(owner, pon)).translate()))
                }
            },
            &Pon::DependencyReference(ref named_prop_ref) => {
                if &named_prop_ref.property_key == "transform" {
                    let prop_ref = try!(system.resolve_named_prop_ref(owner, named_prop_ref));
                    return match self.get_entity_transform(system, &prop_ref.entity_id) {
                        Ok(mat) => Ok(mat.clone()),
                        Err(err) => Err(err)
                    }
                }
                Ok(try!(try!(system.resolve_pon_dependencies(owner, pon)).translate()))
            },
            _ => Ok(try!(try!(system.resolve_pon_dependencies(owner, pon)).translate()))
        }
    }

}


impl ISubSystem for TransformSubSystem {

    fn on_property_value_change(&mut self, system: &mut ISystem, prop_refs: &Vec<PropRef>) {
        for pr in prop_refs.iter().filter(|pr| pr.property_key == "transform") {
            self.cached_transforms.remove(&pr.entity_id);
        }
        for pr in prop_refs.iter().filter(|pr| pr.property_key == "transform") {
            self.get_entity_transform(system, &pr.entity_id).unwrap();
        }
    }
}

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
use pyramid::system::*;

pub struct TransformSubSystem {
    cached_transforms: HashMap<EntityId, Matrix4<f32>>
}

impl TransformSubSystem {
    pub fn new() -> TransformSubSystem {
        TransformSubSystem {
            cached_transforms: HashMap::new()
        }
    }
    fn get_entity_transform(&mut self, document: &mut Document, entity_id: &EntityId) -> Matrix4<f32> {
        {
            match self.cached_transforms.get(entity_id) {
                Some(v) => return v.clone(),
                None => {}
            };
        }

        let expr = {
            match document.get_property(&entity_id, "transform") {
                Ok(expression) => expression.clone(),
                Err(err) => {
                    println!("Couldn't get property 'transform': {:?}", err);
                    return Matrix4::identity();
                }
            }
        };
        let mat = {
            let mat = self.pon_to_matrix(document, entity_id, &expr);
            document.set_property(entity_id, "transformed", mat.to_pon()).unwrap();
            mat
        };

        match self.cached_transforms.entry(*entity_id) {
            Entry::Occupied(_) => unreachable!(),
            Entry::Vacant(v) => v.insert(mat).clone()
        }
    }

    fn pon_to_matrix(&mut self, document: &mut Document, owner: &EntityId, pon: &Pon) -> Matrix4<f32> {
        let resolved_pon_dependency = |document: &mut Document| {
            let r = pon.as_resolved(|pon| {
                pon.translate::<Matrix4<f32>>(&mut TranslateContext::from_doc(document))
            });
            match r {
                Ok(mat) => mat,
                Err(err) => {
                    println!("Unable to resolve pon dependency: {}", err.to_string());
                    Matrix4::identity()
                }
            }
        };
        match pon {
            &Pon::TypedPon(box TypedPon { ref type_name, ref data }) => {
                match type_name.as_str() {
                    "mul" => {
                        data.as_array(|arr| {
                            let mut a = Matrix4::identity();
                            for b in arr {
                                let mat = self.pon_to_matrix(document, owner, b);
                                a = a * mat;
                            }
                            return Ok(a);
                        }).unwrap()
                    },
                    _ => resolved_pon_dependency(document)
                }
            },
            &Pon::DependencyReference(ref named_prop_ref, _) => {
                if &named_prop_ref.property_key == "transform" {
                    let prop_ref = document.resolve_named_prop_ref(owner, named_prop_ref).unwrap();
                    return self.get_entity_transform(document, &prop_ref.entity_id).clone();
                }
                resolved_pon_dependency(document)
            },
            _ => resolved_pon_dependency(document)
        }
    }

}


impl ISubSystem for TransformSubSystem {

    fn on_property_value_change(&mut self, system: &mut System, prop_refs: &Vec<PropRef>) {
        for pr in prop_refs.iter().filter(|pr| pr.property_key == "transform") {
            self.cached_transforms.remove(&pr.entity_id);
        }
        for pr in prop_refs.iter().filter(|pr| pr.property_key == "transform") {
            self.get_entity_transform(system.document_mut(), &pr.entity_id);
        }
    }
}

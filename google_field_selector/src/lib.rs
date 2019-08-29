pub use google_field_selector_derive::FieldSelector;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, LinkedList, VecDeque};

/// This is the purpose of the crate. Given a type that implements FieldSelector
/// this will return a string that can be used in the `fields` parameter of
/// google API's to request a partial response that contains the fields of T.
///
/// See [Google
/// Docs](https://developers.google.com/discovery/v1/performance#partial-response)
/// for more details.
pub fn to_string<T: FieldSelector>() -> String {
    fn append_fields(parents: &mut Vec<&str>, fields: &[Field], output: &mut String) {
        let mut iter = fields.iter();
        let mut next = iter.next();
        while let Some(field) = next {
            append_field(parents, &field, output);
            next = iter.next();
            if next.is_some() {
                output.push_str(",");
            }
        }
    }

    fn append_field(parents: &mut Vec<&str>, field: &Field, output: &mut String) {
        let append_parents = |output: &mut String| {
            for &parent in parents.iter() {
                output.push_str(parent);
                output.push_str("/");
            }
        };

        match field {
            Field::Glob => {
                append_parents(output);
                output.push_str("*");
            }
            Field::Named {
                field_name,
                field_type,
            } => match field_type {
                FieldType::Leaf => {
                    append_parents(output);
                    output.push_str(field_name);
                }
                FieldType::Container(inner_field_type) => {
                    append_parents(output);
                    output.push_str(field_name);
                    match &**inner_field_type {
                        FieldType::Leaf | FieldType::Container(_) => {}
                        FieldType::Struct(fields) => {
                            output.push_str("(");
                            append_fields(&mut Vec::new(), fields, output);
                            output.push_str(")");
                        }
                    }
                }
                FieldType::Struct(fields) => {
                    parents.push(field_name);
                    append_fields(parents, fields, output);
                    parents.pop();
                }
            },
        }
    }

    let mut output = String::new();
    append_fields(&mut Vec::new(), &T::fields(), &mut output);
    output
}

pub enum Field {
    Glob,
    Named {
        field_name: &'static str,
        field_type: FieldType,
    },
}

pub enum FieldType {
    Leaf,
    Struct(Vec<Field>),
    Container(Box<FieldType>),
}

pub trait ToFieldType {
    fn field_type() -> FieldType;
}

/// FieldSelector provides a google api compatible field selector. This trait
/// will typically be generated from a procedural macro using
/// #[derive(FieldSelector)]
pub trait FieldSelector {
    fn fields() -> Vec<Field>;
}

macro_rules! leaf_field_type {
    ($t:ty) => {
        impl ToFieldType for $t {
            fn field_type() -> FieldType {
                FieldType::Leaf
            }
        }
    };
}

leaf_field_type!(bool);
leaf_field_type!(char);
leaf_field_type!(i8);
leaf_field_type!(i16);
leaf_field_type!(i32);
leaf_field_type!(i64);
leaf_field_type!(i128);
leaf_field_type!(isize);
leaf_field_type!(u8);
leaf_field_type!(u16);
leaf_field_type!(u32);
leaf_field_type!(u64);
leaf_field_type!(u128);
leaf_field_type!(usize);
leaf_field_type!(f32);
leaf_field_type!(f64);
leaf_field_type!(String);

// For field selection we treat Options as invisible, proxying to the inner type.
impl<T> ToFieldType for Option<T>
where
    T: ToFieldType,
{
    fn field_type() -> FieldType {
        T::field_type()
    }
}

// implement ToFieldType for std::collections types.
// Vec, VecDeque, HashSet, BTreeSet, LinkedList, all act as containers of other elements.

impl<T> ToFieldType for Vec<T>
where
    T: ToFieldType,
{
    fn field_type() -> FieldType {
        FieldType::Container(Box::new(T::field_type()))
    }
}

impl<T> ToFieldType for VecDeque<T>
where
    T: ToFieldType,
{
    fn field_type() -> FieldType {
        FieldType::Container(Box::new(T::field_type()))
    }
}

impl<T, H> ToFieldType for HashSet<T, H>
where
    T: ToFieldType,
{
    fn field_type() -> FieldType {
        FieldType::Container(Box::new(T::field_type()))
    }
}

impl<T> ToFieldType for BTreeSet<T>
where
    T: ToFieldType,
{
    fn field_type() -> FieldType {
        FieldType::Container(Box::new(T::field_type()))
    }
}

impl<T> ToFieldType for LinkedList<T>
where
    T: ToFieldType,
{
    fn field_type() -> FieldType {
        FieldType::Container(Box::new(T::field_type()))
    }
}

// HashMap and BTreeMap are not considered containers for the purposes of
// selections. The google api does not provide a mechanism to specify fields of
// key/value pairs.
impl<K, V, H> ToFieldType for HashMap<K, V, H> {
    fn field_type() -> FieldType {
        FieldType::Leaf
    }
}

impl<K, V> ToFieldType for BTreeMap<K, V> {
    fn field_type() -> FieldType {
        FieldType::Leaf
    }
}

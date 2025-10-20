use crate::{Accessor, Data, Ident, InvokeRef, MacroInvokeRef, Type, ValueRef};

#[derive(Debug, Clone)]
pub(crate) enum ValueNode {
    Tuple(Vec<ValueRef>),
    Str(String),
    Reference(ValueRef),
    ReferenceMut(ValueRef),
    Dereference(ValueRef),
    Binding {
        name: Ident,
        ty: Type,
    },
    DataStructure {
        name: String,
        data: Data<ValueRef>,
    },
    Invoke(InvokeRef),
    Destructure {
        parent: ValueRef,
        accessor: Accessor,
        ty: Type,
    },
    MacroInvocation(MacroInvokeRef),
}

impl ValueNode {
    pub fn get_type_name(&self) -> Self {
        match self {
            ValueNode::Tuple(types) => {
                let types: String = types.iter().fold(String::new(), |mut acc, v| {
                    match &v.node().get_type_name() {
                        ValueNode::Str(name) => {
                            acc.push_str(name);
                            acc.push_str(", ");
                            acc
                        }
                        _ => unreachable!(),
                    }
                });
                let types = format!("({})", types.trim_end_matches(", "));
                ValueNode::Str(types)
            }
            ValueNode::Str(_) => ValueNode::Str(String::from("str")),
            ValueNode::DataStructure { name, .. } => ValueNode::Str(name.clone()),
            ValueNode::Reference(v) => v.node().get_type_name(),
            ValueNode::ReferenceMut(v) => v.node().get_type_name(),
            ValueNode::Binding { ty, .. } => ValueNode::Str(ty.0.get_name()),
            ValueNode::Destructure {
                parent,
                accessor,
                ty,
            } => ValueNode::Str(ty.0.get_name()),
            node => panic!("ValueNode::get_type_name"),
        }
    }
}

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum VarKind {
    Fluid,
    Rigid,
}

#[derive(Debug, Clone)]
pub struct Var {
    pub value: String,
    pub kind: VarKind,
}

/// Store a variable in the map.
/// Returns an error string if rigid and blocked.
pub fn store_variable(
    vars: &mut HashMap<String, Var>,
    kind: VarKind,
    name: &str,
    value: &str
) -> Result<String, String> {
    if let Some(existing) = vars.get(name) {
        if existing.kind == VarKind::Rigid {
            return Err(format!("cannot overwrite rigid variable '{}'", name));
        }
    }

    vars.insert(name.to_string(), Var {
        value: value.to_string(),
        kind,
    });

    Ok(value.to_string())
}

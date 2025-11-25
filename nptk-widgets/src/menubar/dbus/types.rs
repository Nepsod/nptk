//! Common types for DBus menu implementation.

use std::collections::HashMap;
use zbus::zvariant::{OwnedValue, Structure, Value};

/// A snapshot of the menu structure for DBus transmission.
#[derive(Clone)]
pub struct MenuSnapshot {
    pub entries: Vec<RemoteMenuNode>,
}

/// A node in the remote menu tree.
#[derive(Clone)]
pub struct RemoteMenuNode {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub is_separator: bool,
    pub shortcut: Option<String>,
    pub children: Vec<RemoteMenuNode>,
}

/// Internal menu state for DBusMenu protocol.
#[derive(Default)]
pub struct MenuState {
    pub revision: u32,
    pub entries: Vec<RemoteMenuNode>,
}

impl MenuState {
    pub fn replace(&mut self, snapshot: MenuSnapshot) {
        self.entries = snapshot.entries;
        self.revision = (self.revision.wrapping_add(1)).max(1);
    }

    pub fn layout_with(&self, parent_id: i32, depth: i32, _properties: Vec<&str>) -> SubMenuLayout {
        // - parent_id = 0 → top-level entries
        // - depth: -1 = full recursion, 0 = only this node, >0 recurse that many levels
        let submenus: Vec<OwnedValue> = if parent_id == 0 {
            self.entries
                .iter()
                .map(|n| build_owned_value_recursive(n, depth))
                .collect()
        } else if let Some(node) = find_node_by_id(&self.entries, parent_id) {
            node.children
                .iter()
                .map(|n| build_owned_value_recursive(n, depth))
                .collect()
        } else {
            Vec::new()
        };

        SubMenuLayout {
            id: parent_id,
            fields: {
                let mut root_fields: HashMap<String, OwnedValue> = HashMap::new();
                if parent_id == 0 {
                    // Root node should be minimal - only children-display property.
                    // libdbusmenu-qt treats any item-like properties (label, enabled, visible, type)
                    // as making it a menu item rather than a container.
                    root_fields.insert("children-display".into(), owned_value("menubar"));
                }
                root_fields
            },
            submenus,
        }
    }
}

/// Submenu layout structure for DBusMenu protocol.
#[derive(Clone, serde::Serialize, zbus::zvariant::Type)]
pub struct SubMenuLayout {
    pub id: i32,
    pub fields: HashMap<String, OwnedValue>,
    pub submenus: Vec<OwnedValue>,
}

/// Find a node by ID in the menu tree.
pub fn find_node_by_id<'a>(roots: &'a [RemoteMenuNode], id: i32) -> Option<&'a RemoteMenuNode> {
    for n in roots {
        if n.id == id {
            return Some(n);
        }
        if let Some(found) = find_node_by_id(&n.children, id) {
            return Some(found);
        }
    }
    None
}

/// Build an owned value recursively from a menu node.
pub fn build_owned_value_recursive(node: &RemoteMenuNode, depth: i32) -> OwnedValue {
    let mut fields: HashMap<String, OwnedValue> = HashMap::new();
    if !node.is_separator {
        let label = node.label.replace('_', "__");
        fields.insert("label".into(), owned_value(label));
    }
    fields.insert("enabled".into(), OwnedValue::from(node.enabled));
    fields.insert("visible".into(), OwnedValue::from(true));
    if node.is_separator {
        fields.insert("type".into(), owned_value("separator"));
    }
    if !node.children.is_empty() {
        fields.insert("children-display".into(), owned_value("submenu"));
        // DO NOT add type: "menu" - type is only for item types like "separator",
        // not for indicating that an item has children. children-display is sufficient.
    }
    if let Some(shortcut) = &node.shortcut {
        if let Some(seq) = encode_shortcut(shortcut) {
            fields.insert("shortcut".into(), owned_value(seq));
        }
    }

    // depth semantics:
    // -1 → recurse fully; 0 → no children; N≥1 → include children and recurse with N-1
    let recurse_children = depth < 0 || depth >= 1;
    let next_depth = if depth < 0 { -1 } else { depth.saturating_sub(1) };
    let children: Vec<OwnedValue> = if recurse_children {
        node.children
            .iter()
            .map(|c| build_owned_value_recursive(c, next_depth))
            .collect()
    } else {
        Vec::new()
    };

    owned_value(Structure::from((node.id, fields, children)))
}

/// Get properties map for a node.
pub fn node_properties_map(node: &RemoteMenuNode) -> HashMap<String, OwnedValue> {
    let mut props: HashMap<String, OwnedValue> = HashMap::new();
    if !node.is_separator {
        let label = node.label.replace('_', "__");
        props.insert("label".into(), owned_value(label));
    }
    props.insert("enabled".into(), OwnedValue::from(node.enabled));
    props.insert("visible".into(), OwnedValue::from(true));
    if node.is_separator {
        props.insert("type".into(), owned_value("separator"));
    }
    if !node.children.is_empty() {
        props.insert("children-display".into(), owned_value("submenu"));
        // DO NOT add type: "menu" - type is only for item types like "separator",
        // not for indicating that an item has children. children-display is sufficient.
    }
    if let Some(shortcut) = &node.shortcut {
        if let Some(seq) = encode_shortcut(shortcut) {
            props.insert("shortcut".into(), owned_value(seq));
        }
    }
    props
}

/// Get a specific property value for a node.
pub fn node_property_value(node: &RemoteMenuNode, name: &str) -> Option<OwnedValue> {
    match name {
        "label" if !node.is_separator => Some(owned_value(node.label.replace('_', "__"))),
        "enabled" => Some(OwnedValue::from(node.enabled)),
        "visible" => Some(OwnedValue::from(true)),
        "type" if node.is_separator => Some(owned_value("separator")),
        "children-display" if !node.children.is_empty() => Some(owned_value("submenu")),
        "shortcut" => node.shortcut.as_ref().and_then(|s| encode_shortcut(s.as_str())).map(owned_value),
        _ => None,
    }
}

/// Flatten properties updates for all nodes in the tree.
pub fn flatten_properties_updates(
    roots: &[RemoteMenuNode],
) -> Vec<(i32, HashMap<String, OwnedValue>)> {
    fn recurse<'a>(
        node: &'a RemoteMenuNode,
        acc: &mut Vec<(i32, HashMap<String, OwnedValue>)>,
    ) {
        acc.push((node.id, node_properties_map(node)));
        for c in &node.children {
            recurse(c, acc);
        }
    }

    let mut out = Vec::new();
    for n in roots {
        recurse(n, &mut out);
    }
    out
}

/// Build properties index for all nodes.
pub fn properties_index(roots: &[RemoteMenuNode]) -> HashMap<i32, HashMap<String, OwnedValue>> {
    let mut index: HashMap<i32, HashMap<String, OwnedValue>> = HashMap::new();
    fn recurse(node: &RemoteMenuNode, index: &mut HashMap<i32, HashMap<String, OwnedValue>>) {
        index.insert(node.id, node_properties_map(node));
        for c in &node.children {
            recurse(c, index);
        }
    }
    for n in roots {
        recurse(n, &mut index);
    }
    index
}

/// Convert a value to OwnedValue.
pub fn owned_value<T>(value: T) -> OwnedValue
where
    Value<'static>: From<T>,
{
    OwnedValue::try_from(Value::from(value)).expect("value conversion")
}

/// Encode "Ctrl+Shift+N" into [["Ctrl","Shift","N"]] per DBusMenu spec.
pub fn encode_shortcut(s: &str) -> Option<Vec<Vec<String>>> {
    let parts: Vec<String> = s
        .split('+')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(vec![parts])
    }
}


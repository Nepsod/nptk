//! DBusMenu interface implementation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;

use super::bridge::Command;
use super::types::{MenuState, SubMenuLayout};
use super::types::{find_node_by_id, node_properties_map, node_property_value, owned_value};

/// DBusMenu interface implementation.
pub struct MenuObject {
    pub state: Arc<Mutex<MenuState>>,
    pub evt_tx: Sender<super::BridgeEvent>,
    pub cmd_tx: Sender<Command>,
}

#[interface(name = "com.canonical.dbusmenu")]
impl MenuObject {
    #[zbus(name = "AboutToShow")]
    async fn about_to_show(&self, _id: i32) -> bool {
        log::info!("DBusMenu.AboutToShow id={}", _id);
        // Return true if this node has (or may have) children to show
        let has_children = if _id == 0 {
            !self.state.lock().unwrap().entries.is_empty()
        } else {
            self
                .state
                .lock()
                .unwrap()
                .entries
                .iter()
                .any(|n| n.id == _id && !n.children.is_empty())
        };
        if has_children {
            // Ask the bridge loop to emit LayoutUpdated for this parent
            let _ = self.cmd_tx.send(Command::RequestLayout(_id));
        }
        has_children
    }

    #[zbus(name = "Event")]
    async fn event(
        &self,
        id: i32,
        event_id: &str,
        _data: OwnedValue,
        _timestamp: u32,
    ) {
        log::info!("DBusMenu.Event id={} event_id={}", id, event_id);
        if event_id == "clicked" {
            let _ = self.evt_tx.send(super::BridgeEvent::Activated(id));
        } else if event_id == "opened" {
            let _ = self.cmd_tx.send(Command::RequestLayout(id));
        } else if event_id == "about-to-show" {
            let _ = self.cmd_tx.send(Command::RequestLayout(id));
        }
    }

    #[zbus(name = "GetLayout")]
    async fn get_layout(
        &self,
        parent_id: i32,
        depth: i32,
        properties: Vec<&str>,
    ) -> (u32, SubMenuLayout) {
        let st = self.state.lock().unwrap();
        let props_debug = properties.clone();
        let layout = st.layout_with(parent_id, depth, properties);
        log::info!(
            "DBusMenu.GetLayout parent_id={} depth={} props={:?} revision={} entries_count={} submenus_count={}",
            parent_id,
            depth,
            props_debug,
            st.revision,
            st.entries.len(),
            layout.submenus.len()
        );
        (st.revision, layout)
    }

    #[zbus(name = "GetGroupProperties")]
    async fn get_group_properties(
        &self,
        ids: Vec<i32>,
        properties: Vec<String>,
    ) -> (u32, Vec<(i32, HashMap<String, OwnedValue>)>) {
        log::info!("DBusMenu.GetGroupProperties ids={:?} props={:?}", ids, properties);
        let st = self.state.lock().unwrap();
        let mut out: Vec<(i32, HashMap<String, OwnedValue>)> = Vec::new();
        for id in ids {
            if id == 0 {
                let mut m: HashMap<String, OwnedValue> = HashMap::new();
                let want_all = properties.is_empty();
                // Root node should be minimal - only children-display property.
                // libdbusmenu-qt treats any item-like properties (label, enabled, visible, type)
                // as making it a menu item rather than a container.
                if want_all || properties.iter().any(|p| p == "children-display") {
                    m.insert("children-display".into(), owned_value("menubar"));
                }
                // Explicitly do NOT include label, enabled, visible, or type for root node
                out.push((id, m));
                continue;
            }
            if let Some(node) = find_node_by_id(&st.entries, id) {
                if properties.is_empty() {
                    let map = node_properties_map(node);
                    out.push((id, map));
                } else {
                    let mut map: HashMap<String, OwnedValue> = HashMap::new();
                    for p in properties.iter() {
                        if let Some(v) = node_property_value(node, p.as_str()) {
                            map.insert(p.clone(), v);
                        }
                    }
                    out.push((id, map));
                }
            }
        }
        (st.revision, out)
    }

    #[zbus(name = "GetProperty")]
    async fn get_property(&self, _id: i32, _name: &str) -> OwnedValue {
        log::info!("DBusMenu.GetProperty id={} name={}", _id, _name);
        // Root node is a pure container - only children-display property exists
        if _id == 0 {
            return match _name {
                "children-display" => owned_value("menubar"),
                // Root node has no item-like properties. Return empty string for all others
                // to ensure it's treated as a pure container, not a menu item.
                _ => owned_value(String::new()),
            };
        }
        if let Some(node) = find_node_by_id(&self.state.lock().unwrap().entries, _id) {
            if let Some(v) = node_property_value(node, _name) {
                return v;
            }
        }
        owned_value(String::new())
    }

    #[zbus(signal)]
    #[zbus(name = "LayoutUpdated")]
    pub async fn layout_updated(
        emitter: &SignalEmitter<'_>,
        revision: u32,
        parent: i32,
    ) -> zbus::Result<()>;

    #[zbus(property)]
    #[zbus(name = "Status")]
    fn status(&self) -> &str {
        "normal"
    }

    #[zbus(property)]
    #[zbus(name = "Version")]
    fn version(&self) -> u32 {
        4
    }

    #[zbus(signal)]
    #[zbus(name = "ItemsPropertiesUpdated")]
    pub async fn items_properties_updated(
        emitter: &SignalEmitter<'_>,
        updated: Vec<(i32, HashMap<String, OwnedValue>)>,
        removed: Vec<(i32, Vec<String>)>,
    ) -> zbus::Result<()>;
}


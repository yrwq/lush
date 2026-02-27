use serde::Serialize;
use zbus::blocking::{Connection as BlockingConnection, Proxy as BlockingProxy};
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

use super::TrayItemSnapshot;

pub(super) fn call_item_method<B>(
    item: &TrayItemSnapshot,
    method: &str,
    body: &B,
) -> Result<(), String>
where
    B: zbus::zvariant::DynamicType + Serialize,
{
    let conn = BlockingConnection::session().map_err(|e| format!("dbus connect: {}", e))?;
    let proxy = BlockingProxy::new(
        &conn,
        item.service.as_str(),
        item.path.as_str(),
        item.iface.as_str(),
    )
    .map_err(|e| format!("dbus proxy: {}", e))?;

    proxy
        .call::<_, _, ()>(method, body)
        .map_err(|e| format!("{} failed: {}", method, e))?;
    Ok(())
}

pub(super) fn extract_object_path_string(value: &OwnedValue) -> Option<String> {
    if let Ok(path) = <&ObjectPath<'_>>::try_from(value) {
        return Some(path.to_string());
    }
    if let Ok(path) = OwnedObjectPath::try_from(value.try_clone().ok()?) {
        return Some(path.to_string());
    }

    let cloned = value.try_clone().ok()?;
    let dynamic = Value::try_from(cloned).ok()?;
    if let Ok(path) = dynamic.downcast_ref::<ObjectPath<'_>>() {
        return Some(path.to_string());
    }
    if let Ok(text) = String::try_from(dynamic.try_to_owned().ok()?) {
        let s = text.trim();
        if !s.is_empty() {
            return Some(s.to_string());
        }
    }
    None
}

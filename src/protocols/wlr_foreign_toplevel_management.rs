pub mod wlr_foreign_toplevel_management_unstable_v1 {
    #![allow(unused_imports)]

    use wayland_client;
    use wayland_client::protocol::*;

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!(
            "protocols/wlr-foreign-toplevel-management-unstable-v1.xml"
        );
    }
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!(
        "protocols/wlr-foreign-toplevel-management-unstable-v1.xml"
    );
}

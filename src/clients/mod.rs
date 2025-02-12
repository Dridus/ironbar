use std::sync::Arc;

#[cfg(feature = "clipboard")]
pub mod clipboard;
#[cfg(feature = "workspaces")]
pub mod compositor;
#[cfg(feature = "music")]
pub mod music;
#[cfg(feature = "tray")]
pub mod system_tray;
#[cfg(feature = "upower")]
pub mod upower;
pub mod wayland;

/// Singleton wrapper consisting of
/// all the singleton client types used by modules.
#[derive(Debug, Default)]
pub struct Clients {
    wayland: Option<Arc<wayland::Client>>,
    #[cfg(feature = "workspaces")]
    workspaces: Option<Arc<dyn compositor::WorkspaceClient>>,
    #[cfg(feature = "clipboard")]
    clipboard: Option<Arc<clipboard::Client>>,
    #[cfg(feature = "music")]
    music: std::collections::HashMap<music::ClientType, Arc<dyn music::MusicClient>>,
    #[cfg(feature = "tray")]
    tray: Option<Arc<system_tray::TrayEventReceiver>>,
    #[cfg(feature = "upower")]
    upower: Option<Arc<zbus::fdo::PropertiesProxy<'static>>>,
}

impl Clients {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub fn wayland(&mut self) -> Arc<wayland::Client> {
        self.wayland
            .get_or_insert_with(|| Arc::new(wayland::Client::new()))
            .clone()
    }

    #[cfg(feature = "clipboard")]
    pub fn clipboard(&mut self) -> Arc<clipboard::Client> {
        let wayland = self.wayland();

        self.clipboard
            .get_or_insert_with(|| Arc::new(clipboard::Client::new(wayland)))
            .clone()
    }

    #[cfg(feature = "workspaces")]
    pub fn workspaces(&mut self) -> Arc<dyn compositor::WorkspaceClient> {
        // TODO: Error handling here isn't great - should throw a user-friendly error & exit
        self.workspaces
            .get_or_insert_with(|| {
                compositor::Compositor::create_workspace_client().expect("to be valid compositor")
            })
            .clone()
    }

    #[cfg(feature = "music")]
    pub fn music(&mut self, client_type: music::ClientType) -> Arc<dyn music::MusicClient> {
        self.music
            .entry(client_type.clone())
            .or_insert_with(|| music::create_client(client_type))
            .clone()
    }

    #[cfg(feature = "tray")]
    pub fn tray(&mut self) -> Arc<system_tray::TrayEventReceiver> {
        self.tray
            .get_or_insert_with(|| {
                Arc::new(crate::await_sync(async {
                    system_tray::create_client().await
                }))
            })
            .clone()
    }

    #[cfg(feature = "upower")]
    pub fn upower(&mut self) -> Arc<zbus::fdo::PropertiesProxy<'static>> {
        self.upower
            .get_or_insert_with(|| {
                crate::await_sync(async { upower::create_display_proxy().await })
            })
            .clone()
    }
}

/// Types implementing this trait
/// indicate that they provide a singleton client instance of type `T`.
pub trait ProvidesClient<T: ?Sized> {
    /// Returns a singleton client instance of type `T`.
    fn provide(&self) -> Arc<T>;
}

/// Generates a `ProvidesClient` impl block on `WidgetContext`
/// for the provided `$ty` (first argument) client type.
///
/// The implementation calls `$method` (second argument)
/// on the `Clients` struct to obtain the client instance.
///
/// # Example
/// `register_client!(Client, clipboard);`
#[macro_export]
macro_rules! register_client {
    ($ty:ty, $method:ident) => {
        impl<TSend, TReceive> $crate::clients::ProvidesClient<$ty>
            for $crate::modules::WidgetContext<TSend, TReceive>
        where
            TSend: Clone,
        {
            fn provide(&self) -> Arc<$ty> {
                self.ironbar.clients.borrow_mut().$method()
            }
        }
    };
}

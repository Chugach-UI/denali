use std::{
    env,
    os::{fd::FromRawFd, unix::net::UnixStream},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use denali_utils::proxy::{IdManager, Proxy, SharedIdManager};

pub struct DisplayConnection {
    socket: UnixStream,
    id_manager: SharedIdManager,
    display: dwl::display::Display,
}

impl DisplayConnection {
    pub fn connect() -> Result<Self, String> {
        let socket = if let Some(wayland_socket) = env::var_os("WAYLAND_SOCKET") {
            let wayland_socket = wayland_socket.to_string_lossy().parse::<i32>().unwrap();
            Self::connect_socket(wayland_socket)?
        } else {
            let wayland_display = env::var_os("WAYLAND_DISPLAY").unwrap_or("wayland-0".into());
            Self::connect_display(wayland_display.to_string_lossy().into_owned())?
        };

        let id_manager = Arc::new(Mutex::new(IdManager::new()));

        let display =
            dwl::display::Display::from(Proxy::new_with_manager(1, id_manager.clone()).unwrap());

        Ok(Self {
            socket,
            id_manager,
            display,
        })
    }

    pub fn display(&self) -> &dwl::display::Display {
        &self.display
    }

    fn connect_socket(wayland_socket: i32) -> Result<UnixStream, String> {
        unsafe { Ok(UnixStream::from_raw_fd(wayland_socket)) }
    }

    fn connect_display(wayland_display: String) -> Result<UnixStream, String> {
        let wayland_display = PathBuf::from(wayland_display);
        let path = if !wayland_display.is_absolute() {
            let xdg_runtime_dir = PathBuf::from(env::var_os("XDG_RUNTIME_DIR").unwrap());
            xdg_runtime_dir.join(wayland_display)
        } else {
            wayland_display
        };

        UnixStream::connect(path).map_err(|_| "Failed to connect to socket".into())
    }
}

// TEMP: Temporary type definitions to appease the compiler until codegen does this
mod dwl {
    pub mod display {
        pub struct Display(denali_utils::proxy::Proxy);
        impl Display {
            pub fn sync(&self) {
                todo!()
            }

            pub fn get_registry(&mut self) -> super::registry::Registry {
                self.0.create_object(self.0.version()).unwrap()
            }
        }
        impl From<denali_utils::proxy::Proxy> for Display {
            fn from(value: denali_utils::proxy::Proxy) -> Self {
                Self(value)
            }
        }
        impl denali_utils::Interface for Display {
            const INTERFACE: &'static str = "wl_display";

            const MAX_VERSION: u32 = 1;
        }
    }

    pub mod registry {
        pub struct Registry(denali_utils::proxy::Proxy);
        impl Registry {
            pub fn bind<T: denali_utils::Interface + From<denali_utils::proxy::Proxy>>(
                &mut self,
                name: u32,
                version: u32,
            ) -> T {
                _ = name;
                _ = version;
                self.0.create_object(version).unwrap()
            }
        }
        impl From<denali_utils::proxy::Proxy> for Registry {
            fn from(value: denali_utils::proxy::Proxy) -> Self {
                Self(value)
            }
        }
        impl denali_utils::Interface for Registry {
            const INTERFACE: &'static str = "wl_registry";

            const MAX_VERSION: u32 = 1;
        }
    }
}

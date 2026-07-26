//! 浏览器启动器:选项配置(后端无关)+ Camoufox 自动下载分发 + 进程启动(仅 Camoufox 后端)。

#[cfg(feature = "camoufox")]
pub mod fetch;
pub mod options;
#[cfg(feature = "camoufox")]
pub mod process;

#[cfg(feature = "camoufox")]
pub use fetch::{cache_root, ensure_camoufox, platform_tag};
pub use options::{BrowserOptions, Fingerprint, Geolocation, OsType, Proxy};
#[cfg(feature = "camoufox")]
pub use process::{Launched, launch};

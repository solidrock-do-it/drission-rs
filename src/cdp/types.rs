//! CDP 后端的后端无关**值类型**(与 Camoufox 后端同名同形,prelude 按 feature 出统一名):
//! [`LoadMode`] / [`GetOptions`] / [`ImageFormat`] / [`ShotOpts`] / [`PageRect`] / [`Cookie`] /
//! [`CookieParam`]。这些是纯数据 + builder,供 [`ChromiumTab`](crate::cdp::ChromiumTab) 用。

use std::time::Duration;

/// 页面加载等待模式(对齐 camoufox `LoadMode`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadMode {
    /// 等待 `load` 事件(整页资源加载完成)。默认。
    #[default]
    Normal,
    /// 等待 `DOMContentLoaded`(DOM 就绪即可)。
    Eager,
    /// 不等待,导航下发后立即返回。
    None,
}

impl LoadMode {
    /// 该模式要等待的事件名(CDP `Page.loadEventFired` / `Page.domContentEventFired`)。
    pub(crate) fn cdp_event(self) -> Option<&'static str> {
        match self {
            LoadMode::Normal => Some("Page.loadEventFired"),
            LoadMode::Eager => Some("Page.domContentEventFired"),
            LoadMode::None => None,
        }
    }
}

/// [`ChromiumTab::get_with`](crate::cdp::ChromiumTab) 的可选参数(对齐 camoufox `GetOptions`)。
#[derive(Debug, Clone)]
pub struct GetOptions {
    /// 失败重试次数(总尝试 = `retry + 1`)。默认 0。
    pub retry: u32,
    /// 重试间隔。默认 1s。
    pub interval: Duration,
    /// 本次导航超时;`None` 用标签默认超时。
    pub timeout: Option<Duration>,
    /// 本次加载模式;`None` 用 `Normal`。
    pub load_mode: Option<LoadMode>,
    /// 可选 referer。
    pub referer: Option<String>,
}

impl Default for GetOptions {
    fn default() -> Self {
        Self {
            retry: 0,
            interval: Duration::from_secs(1),
            timeout: None,
            load_mode: None,
            referer: None,
        }
    }
}

impl GetOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn retry(mut self, n: u32) -> Self {
        self.retry = n;
        self
    }
    pub fn interval(mut self, secs: f64) -> Self {
        self.interval = Duration::from_secs_f64(secs.max(0.0));
        self
    }
    pub fn timeout(mut self, d: Duration) -> Self {
        self.timeout = Some(d);
        self
    }
    pub fn load_mode(mut self, m: LoadMode) -> Self {
        self.load_mode = Some(m);
        self
    }
    pub fn referer(mut self, r: impl Into<String>) -> Self {
        self.referer = Some(r.into());
        self
    }
}

/// 截图图片格式(对齐 camoufox `ImageFormat`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageFormat {
    /// PNG(无损,默认)。
    #[default]
    Png,
    /// JPEG(有损,可配合 `quality`)。
    Jpeg,
}

impl ImageFormat {
    /// 对应 `Page.captureScreenshot` 的 `format`。
    pub(crate) fn cdp_format(self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpeg",
        }
    }
}

/// 截图选项(对齐 camoufox `ShotOpts`)。
#[derive(Debug, Clone, Default)]
pub struct ShotOpts {
    /// 是否整页截图(`true` 截整个文档,`false` 截可视视口)。
    pub full_page: bool,
    /// 指定矩形区域 `((left, top), (right, bottom))`(页面坐标,含滚动)。设置后忽略 `full_page`。
    pub region: Option<((f64, f64), (f64, f64))>,
    /// 图片格式(默认 PNG)。
    pub format: ImageFormat,
    /// JPEG 质量 0–100(仅 `Jpeg` 有效)。
    pub quality: Option<u8>,
}

impl ShotOpts {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn full_page(mut self, yes: bool) -> Self {
        self.full_page = yes;
        self
    }
    pub fn region(mut self, left_top: (f64, f64), right_bottom: (f64, f64)) -> Self {
        self.region = Some((left_top, right_bottom));
        self
    }
    pub fn format(mut self, format: ImageFormat) -> Self {
        self.format = format;
        self
    }
    pub fn quality(mut self, q: u8) -> Self {
        self.quality = Some(q);
        self
    }
}

/// 页面尺寸/滚动信息(由 [`ChromiumTab::rect`](crate::cdp::ChromiumTab) 返回,对齐 camoufox `PageRect`)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageRect {
    /// 可视视口宽(`innerWidth`)。
    pub window_width: f64,
    /// 可视视口高(`innerHeight`)。
    pub window_height: f64,
    /// 整个文档内容宽(`scrollWidth`)。
    pub page_width: f64,
    /// 整个文档内容高(`scrollHeight`)。
    pub page_height: f64,
    /// 横向滚动位置(`scrollX`)。
    pub scroll_x: f64,
    /// 纵向滚动位置(`scrollY`)。
    pub scroll_y: f64,
    /// 设备像素比(`devicePixelRatio`)。
    pub device_pixel_ratio: f64,
}

/// 一条 cookie(由 [`ChromiumTab::cookies`](crate::cdp::ChromiumTab) 返回,对齐 camoufox `Cookie`)。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: f64,
    pub http_only: bool,
    pub secure: bool,
}

/// 设置 cookie 的参数(对齐 camoufox `CookieParam`)。至少需要 `name`/`value`,及 `url` 或 `domain` 其一。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CookieParam {
    pub name: String,
    pub value: String,
    pub url: Option<String>,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub secure: Option<bool>,
    pub http_only: Option<bool>,
    pub expires: Option<f64>,
}

/// JS 对话框信息(由 [`ChromiumTab::handle_next_dialog`](crate::cdp::ChromiumTab) 返回,对齐 camoufox `DialogInfo`)。
#[derive(Debug, Clone, Default)]
pub struct DialogInfo {
    /// 对话框文本。
    pub message: String,
    /// 类型(alert/confirm/prompt/beforeunload)。
    pub dialog_type: String,
    /// prompt 的默认值。
    pub default_prompt: String,
}

/// 一次下载的信息(由 [`ChromiumTab::wait_download`](crate::cdp::ChromiumTab) 返回,对齐 camoufox `DownloadInfo`)。
#[derive(Debug, Clone, Default)]
pub struct DownloadInfo {
    /// 下载来源 URL。
    pub url: String,
    /// 建议文件名。
    pub suggested_filename: String,
    /// 落盘完整路径(若已知下载目录)。
    pub path: String,
    /// 状态(`completed`/`canceled`/`inProgress`)。
    pub state: String,
}

impl CookieParam {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            ..Default::default()
        }
    }
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_mode_maps_to_cdp_wait_event() {
        assert_eq!(LoadMode::Normal.cdp_event(), Some("Page.loadEventFired"));
        assert_eq!(
            LoadMode::Eager.cdp_event(),
            Some("Page.domContentEventFired")
        );
        assert_eq!(LoadMode::None.cdp_event(), None);
        assert_eq!(LoadMode::default(), LoadMode::Normal);
    }

    #[test]
    fn image_format_maps_to_cdp_format() {
        assert_eq!(ImageFormat::Png.cdp_format(), "png");
        assert_eq!(ImageFormat::Jpeg.cdp_format(), "jpeg");
        assert_eq!(ImageFormat::default(), ImageFormat::Png);
    }

    #[test]
    fn get_options_builder_sets_fields() {
        let o = GetOptions::new()
            .retry(3)
            .interval(2.5)
            .timeout(Duration::from_secs(9))
            .load_mode(LoadMode::Eager)
            .referer("https://ref.example");
        assert_eq!(o.retry, 3);
        assert_eq!(o.interval, Duration::from_secs_f64(2.5));
        assert_eq!(o.timeout, Some(Duration::from_secs(9)));
        assert_eq!(o.load_mode, Some(LoadMode::Eager));
        assert_eq!(o.referer.as_deref(), Some("https://ref.example"));
    }

    #[test]
    fn get_options_default_and_negative_interval_clamps() {
        let d = GetOptions::default();
        assert_eq!(d.retry, 0);
        assert_eq!(d.interval, Duration::from_secs(1));
        assert!(d.timeout.is_none() && d.load_mode.is_none() && d.referer.is_none());
        // 负间隔被夹到 0,不会 panic 于 Duration::from_secs_f64。
        assert_eq!(GetOptions::new().interval(-5.0).interval, Duration::ZERO);
    }

    #[test]
    fn shot_opts_builder_sets_fields() {
        let s = ShotOpts::new()
            .full_page(true)
            .format(ImageFormat::Jpeg)
            .quality(80);
        assert!(s.full_page);
        assert_eq!(s.format, ImageFormat::Jpeg);
        assert_eq!(s.quality, Some(80));
        assert!(s.region.is_none());

        let r = ShotOpts::new().region((1.0, 2.0), (3.0, 4.0));
        assert_eq!(r.region, Some(((1.0, 2.0), (3.0, 4.0))));
    }

    #[test]
    fn cookie_param_builder_sets_identity_fields() {
        let c = CookieParam::new("sid", "abc")
            .url("https://x.example")
            .domain(".example.com");
        assert_eq!(c.name, "sid");
        assert_eq!(c.value, "abc");
        assert_eq!(c.url.as_deref(), Some("https://x.example"));
        assert_eq!(c.domain.as_deref(), Some(".example.com"));
        // 未设置的可选字段保持 None。
        assert!(c.path.is_none() && c.secure.is_none() && c.expires.is_none());
    }
}

//! 通用**滑块验证码**求解:cdp `ChromiumTab` 后端适配(薄委托层)。
//!
//! 缺口算法 / 类型 / 配置等**后端无关核心**在 [`crate::slider`];本文件只做两件事:
//! ① 为 [`ChromiumTab`] 实现 [`SliderTab`] 原语(跑 JS / 截图 / 点元素 / 鼠标),
//! ② 提供与 camoufox 侧完全一致的公开方法(`tab.slider_gap` / `tab.solve_slider` / 极验·顶象便捷方法),
//! 每个一行委托到核心。这样 cdp 后端也能用整套滑块能力。

use serde_json::Value;

use super::ChromiumTab;
use crate::Result;
use crate::slider::{SliderConfig, SliderGap, SliderResult, SliderTab};

impl SliderTab for ChromiumTab {
    fn sl_run_js(&self, js: &str) -> impl std::future::Future<Output = Result<Value>> {
        self.run_js(js)
    }
    async fn sl_ele_screenshot(&self, css_selector: &str) -> Result<Vec<u8>> {
        self.ele(&format!("css:{css_selector}"))
            .await?
            .screenshot_bytes()
            .await
    }
    async fn sl_ele_click(&self, css_selector: &str) {
        if let Ok(e) = self.ele(&format!("css:{css_selector}")).await {
            let _ = e.click().await;
        }
    }
    fn sl_mouse_move(&self, x: f64, y: f64) -> impl std::future::Future<Output = Result<()>> {
        self.mouse_move(x, y)
    }
    fn sl_mouse_down(&self, x: f64, y: f64) -> impl std::future::Future<Output = Result<()>> {
        self.mouse_down(x, y)
    }
    fn sl_mouse_drag(&self, x: f64, y: f64) -> impl std::future::Future<Output = Result<()>> {
        self.mouse_drag(x, y)
    }
    fn sl_mouse_drag_fast(&self, x: f64, y: f64) -> Result<()> {
        self.mouse_drag_fast(x, y)
    }
    fn sl_mouse_up(&self, x: f64, y: f64) -> impl std::future::Future<Output = Result<()>> {
        self.mouse_up(x, y)
    }
}

impl ChromiumTab {
    /// **纯视觉**:按 [`SliderConfig`] 读图、自动选缺口算法,算出拼图需要水平移动的距离([`SliderGap`])。
    /// 要求验证图已显示。读图失败 / 无有效结果返回 `Err`。
    pub async fn slider_gap(&self, cfg: &SliderConfig) -> Result<SliderGap> {
        crate::slider::slider_gap(self, cfg).await
    }

    /// **一把梭**:弹出→匹配→闭环拟人拖动→判定→非通过换图重试。返回 [`SliderResult`]。
    pub async fn solve_slider(&self, cfg: &SliderConfig) -> Result<SliderResult> {
        crate::slider::solve_slider(self, cfg).await
    }

    /// 极验 v4 滑块缺口(预设 [`SliderConfig::geetest_v4`] 的 [`slider_gap`](Self::slider_gap))。
    pub async fn geetest_slide_gap(&self) -> Result<SliderGap> {
        crate::slider::geetest_slide_gap(self).await
    }

    /// 一把梭求解极验 v4 滑块(预设 [`SliderConfig::geetest_v4`] 的 [`solve_slider`](Self::solve_slider))。
    /// 要调尝试次数等用 `solve_slider(&SliderConfig::geetest_v4().max_attempts(8))`。
    pub async fn solve_geetest_slide(&self) -> Result<SliderResult> {
        crate::slider::solve_geetest_slide(self).await
    }

    /// 顶象滑块缺口(预设 [`SliderConfig::dingxiang`] 的 [`slider_gap`](Self::slider_gap),`index`=实例后缀)。
    pub async fn dingxiang_slide_gap(&self, index: u32) -> Result<SliderGap> {
        crate::slider::dingxiang_slide_gap(self, index).await
    }

    /// 一把梭求解顶象滑块(预设 [`SliderConfig::dingxiang`])。弹出式传 `open` 触发按钮(如 `#btn-popup`)。
    /// 注:顶象 demo 对自动化拖动有轨迹/IP 行为风控会弹回(与缺口算法无关);本方法只保证**缺口找得准 +
    /// 拖到位**。要自定义尝试次数等改用 `solve_slider(&SliderConfig::dingxiang(i)...)`。
    pub async fn solve_dingxiang_slide(
        &self,
        index: u32,
        open: Option<&str>,
    ) -> Result<SliderResult> {
        crate::slider::solve_dingxiang_slide(self, index, open).await
    }
}

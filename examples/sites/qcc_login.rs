//! 企查查账号密码登录案例(CDP / Chromium,默认有头)。
//!
//! 流程:打开首页 → 登录弹窗 → 切到“密码登录” → 从环境变量读取账号密码 → 勾选协议 →
//! 可信点击“立即登录” → 识别极验 v4 的滑块 / 点选分支 → 在可见浏览器中等待用户完成验证 →
//! 判断登录成功、站点错误或超时。验证码由当前账户持有人在浏览器窗口内完成;持久 profile 会保留登录态。
//!
//! 只检查当前页面选择器(不提交账号密码):
//! `cargo run --example qcc_login`
//!
//! 执行登录:
//! `QCC_USERNAME='用户名' QCC_PASSWORD='密码' cargo run --example qcc_login`
//!
//! 可选环境变量:
//! - `HL=1`:无头运行(默认 `HL=0`;出现交互验证时请使用有头模式)
//! - `QCC_PROFILE_DIR=...`:持久浏览器 profile(默认 `target/qcc-login-profile`)
//! - `QCC_WAIT_SECS=180`:等待验证和登录结果的秒数
//! - `QCC_URL=https://www.qcc.com/`:覆盖目标 URL
//! - `CONNECT=http://127.0.0.1:9222`:接管已经开启 CDP 的 Chromium

use std::path::PathBuf;
use std::time::{Duration, Instant};

use drission::prelude::*;
use tokio::time::sleep;

const DEFAULT_URL: &str = "https://www.qcc.com/";
const LOGIN_ENTRY: &str = ".qcc-header-login-btn";
const LOGIN_MODE_SWITCH: &str = ".qcc-login-type-change";
const PASSWORD_TAB: &str =
    "xpath://div[contains(@class,'qcc-login-phone-tabs-item')]/a[normalize-space()='密码登录']";
const USERNAME_INPUT: &str = "css:input[placeholder='请输入手机号码/用户名']";
const PASSWORD_INPUT: &str = "css:input[placeholder='请输入密码']";
const CONSENT_INPUT: &str = ".qcc-login-phone-tip input[type='checkbox']";
const CONSENT_LABEL: &str = ".qcc-login-phone-tip .qccd-checkbox-wrapper";
const LOGIN_SUBMIT: &str = "xpath://button[normalize-space()='立即登录']";

/// 仅观察页面可见状态,不读取 Cookie、localStorage 或输入框内容。
const PAGE_STATE_JS: &str = r#"
(() => {
  const shown = (e) => {
    if (!e) return false;
    const r = e.getBoundingClientRect();
    const s = getComputedStyle(e);
    return r.width > 2 && r.height > 2 && s.display !== 'none' &&
      s.visibility !== 'hidden' && Number(s.opacity || 1) > 0;
  };
  const firstShown = (selector) =>
    Array.from(document.querySelectorAll(selector)).find(shown) || null;

  const challenge = firstShown(
    '.geetest_box_wrap, .geetest_box, .geetest_captcha, [class*="geetest_box_wrap"]'
  );
  if (challenge) {
    const text = (challenge.innerText || '').toLowerCase();
    const point = firstShown(
      '.geetest_click, .geetest_icon, [class*="geetest_click"], [class*="geetest_icon"]'
    );
    const slider = firstShown(
      '.geetest_slider, .geetest_slide, [class*="geetest_slider"], [class*="geetest_slide"]'
    );
    if (point || /click|select|点选|依次|图中/.test(text)) {
      return { kind: 'challenge-point', message: '检测到点选验证' };
    }
    if (slider || /slide|drag|拖动|滑块/.test(text)) {
      return { kind: 'challenge-slider', message: '检测到滑块验证' };
    }
    return { kind: 'challenge-other', message: '检测到交互验证' };
  }

  const message = firstShown(
    '.qccd-message-notice, .qccd-message-custom-content, [role="alert"]'
  );
  const messageText = message ? (message.innerText || message.textContent || '').trim() : '';
  if (messageText) return { kind: 'rejected', message: messageText.slice(0, 240) };

  const loginModal = firstShown('.qcc-login');
  const loginText = loginModal ? (loginModal.innerText || '').replace(/\s+/g, ' ').trim() : '';
  const loginErrors = [
    '账号和密码不匹配', '账号或密码', '密码错误', '账号不存在',
    '用户不存在', '手机号未注册', '登录失败'
  ];
  if (loginText && loginErrors.some(word => loginText.includes(word))) {
    return { kind: 'rejected', message: loginText.slice(0, 240) };
  }

  const loginEntry = firstShown('.qcc-header-login-btn');
  if (!loginModal && !loginEntry) {
    return { kind: 'authenticated', message: '登录弹窗和登录入口均已消失' };
  }
  if (!loginModal && loginEntry) {
    return { kind: 'signed-out', message: '页面仍显示登录入口' };
  }
  return { kind: 'waiting', message: '' };
})()
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
enum PageState {
    Challenge(&'static str),
    Authenticated,
    Rejected(String),
    SignedOut,
    Waiting,
}

#[tokio::main]
async fn main() -> drission::Result<()> {
    let headless = std::env::var("HL").map(|v| v != "0").unwrap_or(false);
    let profile_dir = std::env::var_os("QCC_PROFILE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/qcc-login-profile"));

    let (browser, owns_browser) = if let Ok(url) = std::env::var("CONNECT") {
        println!("[*] 接管浏览器 {url}");
        (ChromiumBrowser::connect(&url).await?, false)
    } else {
        println!(
            "[*] 启动 Chromium(headless={headless}, profile={})",
            profile_dir.display()
        );
        (
            ChromiumBrowser::launch(
                ChromiumOptions::new()
                    .headless(headless)
                    .user_data_dir(profile_dir),
            )
            .await?,
            true,
        )
    };

    // 无论流程成功或报错都结束本例启动的浏览器;接管模式仅释放连接,保留原浏览器。
    let result = run_case(&browser, headless).await;
    let quit_result = if owns_browser {
        browser.quit().await
    } else {
        Ok(())
    };
    result?;
    quit_result?;
    Ok(())
}

async fn run_case(browser: &ChromiumBrowser, headless: bool) -> drission::Result<()> {
    let url = std::env::var("QCC_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
    let wait_secs = std::env::var("QCC_WAIT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(180);

    println!("[*] 打开 {url}");
    let tab = browser.new_tab(Some(&url)).await?;
    tab.wait().doc_loaded(Some(Duration::from_secs(30))).await?;
    if !tab
        .wait()
        .ele_displayed(".qcc-header", Some(Duration::from_secs(20)))
        .await?
    {
        return Err(Error::msg("首页头部在 20 秒内没有显示"));
    }

    if page_state(&tab).await? == PageState::Authenticated {
        println!("[ok] 持久 profile 已有登录态");
        return Ok(());
    }

    open_password_form(&tab).await?;

    let (username, password) = match (std::env::var("QCC_USERNAME"), std::env::var("QCC_PASSWORD"))
    {
        (Ok(username), Ok(password)) if !username.is_empty() && !password.is_empty() => {
            (username, password)
        }
        _ => {
            println!("[ok] 登录表单选择器检查通过;本次为 probe 模式,未提交表单");
            println!(
                "[i] 执行登录:QCC_USERNAME='用户名' QCC_PASSWORD='密码' cargo run --example qcc_login"
            );
            return Ok(());
        }
    };

    println!("[*] 填写账号密码(值不会写入日志)");
    tab.ele(USERNAME_INPUT)
        .await?
        .input_human(&username)
        .await?;
    tab.ele(PASSWORD_INPUT)
        .await?
        .input_human(&password)
        .await?;

    let consented = tab
        .run_js(&format!(
            "Boolean(document.querySelector({selector:?})?.checked)",
            selector = CONSENT_INPUT
        ))
        .await?
        .as_bool()
        .unwrap_or(false);
    if !consented {
        tab.ele(CONSENT_LABEL).await?.click().await?;
    }

    println!("[*] 可信点击“立即登录”");
    tab.ele(LOGIN_SUBMIT).await?.click().await?;
    wait_for_login(&tab, headless, Duration::from_secs(wait_secs)).await
}

async fn open_password_form(tab: &ChromiumTab) -> drission::Result<()> {
    // 页面可能因为持久 profile 或上一轮 probe 已经停在密码表单,先走快速路径。
    if tab
        .wait()
        .ele_displayed(USERNAME_INPUT, Some(Duration::from_millis(500)))
        .await?
    {
        return Ok(());
    }

    tab.ele(LOGIN_ENTRY).await?.click().await?;
    if !tab
        .wait()
        .ele_displayed(".qcc-login", Some(Duration::from_secs(8)))
        .await?
    {
        return Err(Error::msg("登录弹窗在 8 秒内没有显示"));
    }

    if tab
        .wait()
        .ele_displayed(PASSWORD_TAB, Some(Duration::from_millis(500)))
        .await?
    {
        // 已在短信/密码面板,但当前选中“验证码登录”。
        tab.ele(PASSWORD_TAB).await?.click().await?;
    } else if !tab
        .wait()
        .ele_displayed(USERNAME_INPUT, Some(Duration::from_millis(500)))
        .await?
    {
        // 首页通常先显示扫码 / 微信面板,右上角切换控件进入短信/密码面板。
        if !tab
            .wait()
            .ele_displayed(LOGIN_MODE_SWITCH, Some(Duration::from_secs(8)))
            .await?
        {
            print_login_diagnostic(tab).await;
            return Err(Error::msg("登录方式切换控件在 8 秒内没有显示"));
        }
        tab.ele(LOGIN_MODE_SWITCH).await?.click().await?;

        // 新 profile 通常默认落在“验证码登录”;历史 profile 也可能直接记住“密码登录”。
        if !tab
            .wait()
            .ele_displayed(USERNAME_INPUT, Some(Duration::from_millis(500)))
            .await?
        {
            if !tab
                .wait()
                .ele_displayed(PASSWORD_TAB, Some(Duration::from_secs(8)))
                .await?
            {
                print_login_diagnostic(tab).await;
                return Err(Error::msg("密码登录页签在 8 秒内没有显示"));
            }
            tab.ele(PASSWORD_TAB).await?.click().await?;
        }
    }

    if !tab
        .wait()
        .ele_displayed(USERNAME_INPUT, Some(Duration::from_secs(8)))
        .await?
    {
        print_login_diagnostic(tab).await;
        return Err(Error::msg("密码登录表单在 8 秒内没有显示"));
    }
    if !tab
        .wait()
        .ele_displayed(PASSWORD_INPUT, Some(Duration::from_secs(2)))
        .await?
    {
        return Err(Error::msg("密码输入框没有显示"));
    }

    println!("[ok] 已打开账号密码登录表单");
    Ok(())
}

async fn print_login_diagnostic(tab: &ChromiumTab) {
    let js = r#"(() => Array.from(document.querySelectorAll('.qcc-login input, .qcc-login button, .qcc-login [class]'))
      .filter(e => { const r=e.getBoundingClientRect(); return r.width>2 && r.height>2; })
      .map(e => ({
        tag:e.tagName.toLowerCase(), class:String(e.className || ''), type:e.getAttribute('type'),
        placeholder:e.getAttribute('placeholder'), text:(e.innerText || '').trim().slice(0, 60)
      })).slice(0, 40))()"#;
    if let Ok(value) = tab.run_js(js).await {
        println!("[diag] 当前可见登录控件:{value}");
    }
}

async fn wait_for_login(
    tab: &ChromiumTab,
    headless: bool,
    timeout: Duration,
) -> drission::Result<()> {
    let deadline = Instant::now() + timeout;
    let mut announced_challenge: Option<&'static str> = None;
    let mut signed_out_since: Option<Instant> = None;

    loop {
        match page_state(tab).await? {
            PageState::Challenge(kind) => {
                signed_out_since = None;
                if announced_challenge != Some(kind) {
                    println!("[*] {kind};请在浏览器窗口中完成,脚本将继续等待登录结果");
                    announced_challenge = Some(kind);
                }
                if headless {
                    return Err(Error::msg(
                        "当前出现点选/滑块交互验证;请使用默认有头模式或设置 HL=0 运行",
                    ));
                }
            }
            PageState::Authenticated => {
                println!("[ok] 登录成功;登录态已保存在持久 profile");
                return Ok(());
            }
            PageState::Rejected(message) => {
                return Err(Error::msg(format!("站点返回登录提示:{message}")));
            }
            PageState::SignedOut => match signed_out_since {
                Some(since) if since.elapsed() >= Duration::from_secs(2) => {
                    return Err(Error::msg("验证结束后页面仍处于未登录状态"));
                }
                Some(_) => {}
                None => signed_out_since = Some(Instant::now()),
            },
            PageState::Waiting => signed_out_since = None,
        }

        if Instant::now() >= deadline {
            return Err(Error::msg(format!(
                "等待登录结果超时({} 秒);可通过 QCC_WAIT_SECS 调整",
                timeout.as_secs()
            )));
        }
        sleep(Duration::from_millis(400)).await;
    }
}

async fn page_state(tab: &ChromiumTab) -> drission::Result<PageState> {
    let value = tab.run_js(PAGE_STATE_JS).await?;
    let kind = value["kind"].as_str().unwrap_or("waiting");
    let message = value["message"].as_str().unwrap_or_default().to_string();
    Ok(match kind {
        "challenge-slider" => PageState::Challenge("检测到极验滑块验证"),
        "challenge-point" => PageState::Challenge("检测到极验点选验证"),
        "challenge-other" => PageState::Challenge("检测到交互验证"),
        "authenticated" => PageState::Authenticated,
        "rejected" => PageState::Rejected(message),
        "signed-out" => PageState::SignedOut,
        _ => PageState::Waiting,
    })
}

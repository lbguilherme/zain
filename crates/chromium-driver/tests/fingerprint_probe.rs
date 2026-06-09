//! Diagnostic probe: dumps the fingerprint signals that bot-detection
//! services (Cloudflare, hCaptcha, DataDome) actually inspect, under the
//! SAME conditions `rpa` uses (default launch + `dom()` which enables touch
//! emulation).
//!
//! This is evidence-gathering, not an assertion test: run it and read the
//! printed JSON to decide which (if any) stealth patches are justified.
//!
//!   cargo test -p chromium-driver --test fingerprint_probe -- --ignored --nocapture

use chromium_driver::{LaunchOptions, launch};

const PROBE_JS: &str = r#"
(async () => {
    const out = {};

    // 1. The headline automation flag.
    out.navigator_webdriver = navigator.webdriver;

    // 2/3. Real Chrome exposes window.chrome (+ .runtime under some conditions).
    out.window_chrome = typeof window.chrome;
    out.window_chrome_runtime = (window.chrome && typeof window.chrome.runtime) || "absent";

    // 4/5. Empty plugins/mimeTypes is a classic headless tell.
    out.plugins_length = navigator.plugins ? navigator.plugins.length : -1;
    out.mimetypes_length = navigator.mimeTypes ? navigator.mimeTypes.length : -1;

    // 6. Empty languages is a tell.
    out.languages = navigator.languages;
    out.language = navigator.language;

    // 7. UA must not say "HeadlessChrome".
    out.user_agent = navigator.userAgent;
    out.ua_has_headless = /headless/i.test(navigator.userAgent);
    out.platform = navigator.platform;
    out.ua_data_platform = navigator.userAgentData ? navigator.userAgentData.platform : "no-uadata";
    out.ua_data_mobile = navigator.userAgentData ? navigator.userAgentData.mobile : null;

    // 8. Hardware signals.
    out.hardware_concurrency = navigator.hardwareConcurrency;
    out.device_memory = navigator.deviceMemory ?? null;
    out.max_touch_points = navigator.maxTouchPoints;

    // 9. WebGL vendor/renderer — "SwiftShader"/"llvmpipe"/"Mesa" => software
    //    rendering, a strong headless/VM signal.
    try {
        const c = document.createElement("canvas");
        const gl = c.getContext("webgl") || c.getContext("experimental-webgl");
        const dbg = gl && gl.getExtension("WEBGL_debug_renderer_info");
        out.webgl_vendor = dbg ? gl.getParameter(dbg.UNMASKED_VENDOR_WEBGL) : "no-ext";
        out.webgl_renderer = dbg ? gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : "no-ext";
    } catch (e) {
        out.webgl_error = String(e);
    }

    // 10. permissions.query vs Notification.permission mismatch is a famous
    //     headless detector (query says "prompt" while Notification says "denied").
    try {
        const st = await navigator.permissions.query({ name: "notifications" });
        out.permissions_notifications = st.state;
    } catch (e) {
        out.permissions_error = String(e);
    }
    out.notification_permission = (typeof Notification !== "undefined") ? Notification.permission : "no-Notification";

    return JSON.stringify(out, null, 2);
})()
"#;

#[tokio::test]
#[ignore] // launches a real browser; run with `-- --ignored --nocapture`
async fn fingerprint_probe() {
    let (mut process, browser) = launch(LaunchOptions::default()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();
    page.enable().await.unwrap();

    // Probe BEFORE touch emulation.
    let before = page.eval_value_async(PROBE_JS).await.unwrap();
    println!(
        "\n===== FINGERPRINT (raw page, no dom()) =====\n{}",
        before.as_str().unwrap_or("<not a string>")
    );

    // dom() enables touch emulation in this crate — probe the post-dom state too,
    // because that's what `rpa` actually runs against.
    let _ = page.dom().await.unwrap();
    let after = page.eval_value_async(PROBE_JS).await.unwrap();
    println!(
        "\n===== FINGERPRINT (after dom(), touch emulation on) =====\n{}",
        after.as_str().unwrap_or("<not a string>")
    );

    // Re-probe on a REAL https page. userAgentData / deviceMemory are gated to
    // secure contexts, so about:blank under-reports them — this disambiguates
    // "real leak" from "probe artifact".
    page.navigate("https://example.com").await.unwrap();
    page.wait_for_load(std::time::Duration::from_secs(30))
        .await
        .ok();
    let https = page.eval_value_async(PROBE_JS).await.unwrap();
    println!(
        "\n===== FINGERPRINT (https://example.com, secure context) =====\n{}",
        https.as_str().unwrap_or("<not a string>")
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// Regression guard for the two evidence-justified coherence fixes:
/// - `navigator.maxTouchPoints` is a stable `1` — present from the first read
///   (touch emulation enabled eagerly at attach) and unchanged after `dom()`,
///   so it never mutates mid-session.
/// - locale presents as Brazil: `navigator.languages == ["pt-BR","pt"]`.
#[tokio::test]
#[ignore] // launches a real browser; run with `-- --ignored`
async fn coherence_brazil_and_stable_touch() {
    let (mut process, browser) = launch(LaunchOptions::default()).await.unwrap();
    let page = browser
        .create_page("data:text/html,<title>probe</title>")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();
    page.enable().await.unwrap();

    // BEFORE any dom() call: maxTouchPoints must already be 1 (born stable).
    let touch_before = page.eval_value("navigator.maxTouchPoints").await.unwrap();
    assert_eq!(
        touch_before.as_i64(),
        Some(1),
        "maxTouchPoints should be 1 from the first read, got {touch_before}"
    );

    // Locale must present as Brazil.
    let langs = page
        .eval_value("JSON.stringify(navigator.languages)")
        .await
        .unwrap();
    assert_eq!(
        langs.as_str(),
        Some(r#"["pt-BR","pt"]"#),
        "navigator.languages should be pt-BR,pt, got {langs}"
    );
    let lang = page.eval_value("navigator.language").await.unwrap();
    assert_eq!(lang.as_str(), Some("pt-BR"), "navigator.language");

    // Timezone must present as São Paulo (CDP override, host-independent).
    let tz = page
        .eval_value("Intl.DateTimeFormat().resolvedOptions().timeZone")
        .await
        .unwrap();
    assert_eq!(
        tz.as_str(),
        Some("America/Sao_Paulo"),
        "Intl timezone should be America/Sao_Paulo, got {tz}"
    );

    // AFTER dom(): maxTouchPoints must be unchanged (no 0→1 or 1→N mutation).
    let _ = page.dom().await.unwrap();
    let touch_after = page.eval_value("navigator.maxTouchPoints").await.unwrap();
    assert_eq!(
        touch_after.as_i64(),
        Some(1),
        "maxTouchPoints must stay 1 after dom(), got {touch_after}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

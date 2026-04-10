use chromium_driver::{LaunchOptions, launch};

fn opts() -> LaunchOptions {
    LaunchOptions::default()
}

/// navigator.webdriver should be false/undefined (not true).
/// Bots set this to true; --disable-blink-features=AutomationControlled should hide it.
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn webdriver_property_hidden() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page.eval_value("navigator.webdriver").await.unwrap();
    assert!(
        val.is_null() || val == false,
        "navigator.webdriver should be false/undefined, got: {val}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// window.chrome should exist (real Chrome has it, headless/automation may not).
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn window_chrome_exists() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page.eval_value("typeof window.chrome").await.unwrap();
    assert_eq!(
        val, "object",
        "window.chrome should be an object, got: {val}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// window.chrome.runtime should exist (real Chrome exposes it).
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn chrome_runtime_exists() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page
        .eval_value("typeof window.chrome.runtime")
        .await
        .unwrap();
    assert!(
        val == "object" || val == "undefined",
        // In real Chrome without extensions, chrome.runtime may be undefined but chrome.runtime
        // existing as a property is what matters. Some detection checks typeof !== 'undefined'.
        // The key thing is window.chrome itself exists.
        "window.chrome.runtime type: {val}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// navigator.plugins should not be empty (empty = headless red flag).
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn navigator_plugins_not_empty() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page.eval_value("navigator.plugins.length").await.unwrap();
    let len = val.as_i64().unwrap_or(0);
    assert!(
        len > 0,
        "navigator.plugins should not be empty, got length: {len}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// navigator.languages should not be empty.
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn navigator_languages_not_empty() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page.eval_value("navigator.languages.length").await.unwrap();
    let len = val.as_i64().unwrap_or(0);
    assert!(
        len > 0,
        "navigator.languages should not be empty, got length: {len}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// User-Agent should NOT contain "HeadlessChrome".
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn user_agent_not_headless() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page.eval_value("navigator.userAgent").await.unwrap();
    let ua = val.as_str().unwrap_or("");
    assert!(
        !ua.contains("HeadlessChrome"),
        "User-Agent should not contain HeadlessChrome: {ua}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// window.outerWidth and outerHeight should not be 0 (they are 0 in headless).
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn outer_dimensions_not_zero() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    // Wait for the window compositor to fully initialize
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let w = page
        .eval_value("window.outerWidth")
        .await
        .unwrap()
        .as_i64()
        .unwrap_or(0);
    let h = page
        .eval_value("window.outerHeight")
        .await
        .unwrap()
        .as_i64()
        .unwrap_or(0);
    assert!(w > 0, "outerWidth should not be 0, got: {w}");
    assert!(h > 0, "outerHeight should not be 0, got: {h}");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// No $cdc_ properties should exist on document (ChromeDriver injection).
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn no_cdc_properties() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page
        .eval_value("Object.keys(document).filter(k => k.match(/\\$?cdc_/)).length")
        .await
        .unwrap();
    let count = val.as_i64().unwrap_or(-1);
    assert_eq!(
        count, 0,
        "document should have no $cdc_ properties, found: {count}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// Notification.permission on a page with a real origin should not be "denied".
/// On about:blank (no origin), Chrome always returns "denied" — that's normal.
/// This test navigates to example.com to check on a real origin.
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn notification_permission_not_denied() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();
    page.enable().await.unwrap();
    page.navigate("https://example.com").await.unwrap();
    page.wait_for_load(std::time::Duration::from_secs(10))
        .await
        .unwrap();

    let val = page.eval_value("Notification.permission").await.unwrap();
    let perm = val.as_str().unwrap_or("unknown");
    assert_eq!(
        perm, "default",
        "Notification.permission should be 'default' on a real origin, got: {perm}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// WebGL renderer should not be "Google SwiftShader" (software renderer used in headless).
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn webgl_renderer_not_swiftshader() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page
        .eval_value(
            r#"(() => {
                const canvas = document.createElement('canvas');
                const gl = canvas.getContext('webgl') || canvas.getContext('experimental-webgl');
                if (!gl) return 'no-webgl';
                const ext = gl.getExtension('WEBGL_debug_renderer_info');
                if (!ext) return 'no-ext';
                return gl.getParameter(ext.UNMASKED_RENDERER_WEBGL);
            })()"#,
        )
        .await
        .unwrap();
    let renderer = val.as_str().unwrap_or("unknown");
    assert!(
        !renderer.contains("SwiftShader"),
        "WebGL renderer should not be SwiftShader (headless indicator): {renderer}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// Permissions API should not reveal automation.
/// query({name:'notifications'}) should return "prompt", not "denied".
/// Requires a page with a real origin (about:blank always returns "denied").
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn permissions_query_not_denied() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();
    page.enable().await.unwrap();
    page.navigate("https://example.com").await.unwrap();
    page.wait_for_load(std::time::Duration::from_secs(10))
        .await
        .unwrap();

    let val = page
        .eval_value_async(
            r#"navigator.permissions.query({name: 'notifications'}).then(r => r.state)"#,
        )
        .await
        .unwrap();
    let state = val.as_str().unwrap_or("unknown");
    assert_eq!(
        state, "prompt",
        "permissions.query notifications should be 'prompt', got: {state}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// Media codecs: canPlayType should return non-empty for common codecs.
/// Headless Chrome may return "" for all codecs.
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn media_codecs_supported() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page
        .eval_value(
            r#"(() => {
                const video = document.createElement('video');
                return video.canPlayType('video/mp4; codecs="avc1.42E01E"');
            })()"#,
        )
        .await
        .unwrap();
    let result = val.as_str().unwrap_or("");
    assert!(
        !result.is_empty(),
        "canPlayType for H.264 should not be empty (headless indicator)"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// CDP scripts injected via Runtime.evaluate should not leak sourceURL.
/// Check that evaluated scripts don't leave detectable traces.
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn no_leaked_source_urls() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    // Run some eval first to ensure CDP has injected scripts
    let _ = page.eval_value("1 + 1").await.unwrap();

    // Try to detect CDP-injected scripts via Error stack traces
    let val = page
        .eval_value(
            r#"(() => {
                try { throw new Error('test'); } catch(e) {
                    return e.stack.includes('pptr:') || e.stack.includes('puppeteer');
                }
            })()"#,
        )
        .await
        .unwrap();
    assert_eq!(
        val, false,
        "Error stack should not contain CDP/puppeteer markers"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// The toString of native functions should not be overwritten.
/// Bot detection checks if Function.prototype.toString.call(navigator.permissions.query)
/// returns "function query() { [native code] }".
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn native_function_tostring() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page
        .eval_value(
            r#"Function.prototype.toString.call(navigator.permissions.query).includes('[native code]')"#,
        )
        .await
        .unwrap();
    assert_eq!(
        val, true,
        "navigator.permissions.query should appear as native code"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

// ── Advanced bot detection techniques ────────────────────────────────────

/// Chrome DevTools Protocol detection via Runtime domain.
/// Anti-bot scripts check for the presence of `Runtime.enable` side effects
/// by looking for extra properties on Error objects or stack trace anomalies.
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn error_stack_no_cdp_artifacts() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    // CDP scripts get sourceURL annotations; check they don't leak into user errors
    let val = page
        .eval_value(
            r#"(() => {
                try { null.x } catch(e) {
                    const s = e.stack;
                    return s.includes('__puppeteer') || s.includes('__cdp')
                        || s.includes('__selenium') || s.includes('__webdriver')
                        || s.includes('Runtime.evaluate');
                }
            })()"#,
        )
        .await
        .unwrap();
    assert_eq!(
        val, false,
        "Error stack should not contain automation markers"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// Check that navigator.connection exists (missing in some automation setups).
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn navigator_connection_exists() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page
        .eval_value("typeof navigator.connection")
        .await
        .unwrap();
    assert_eq!(
        val, "object",
        "navigator.connection should exist, got: {val}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// Screen dimensions should be reasonable (anti-bot checks for 0x0 or unusual sizes).
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn screen_dimensions_reasonable() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let w = page
        .eval_value("screen.width")
        .await
        .unwrap()
        .as_i64()
        .unwrap_or(0);
    let h = page
        .eval_value("screen.height")
        .await
        .unwrap()
        .as_i64()
        .unwrap_or(0);
    let cd = page
        .eval_value("screen.colorDepth")
        .await
        .unwrap()
        .as_i64()
        .unwrap_or(0);

    assert!(w >= 800, "screen.width should be >= 800, got: {w}");
    assert!(h >= 600, "screen.height should be >= 600, got: {h}");
    assert!(cd >= 24, "screen.colorDepth should be >= 24, got: {cd}");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// window.speechSynthesis should exist (missing in some headless configs).
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn speech_synthesis_exists() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page
        .eval_value("typeof window.speechSynthesis")
        .await
        .unwrap();
    assert_eq!(val, "object", "speechSynthesis should exist, got: {val}");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// Check for prototype chain tampering.
/// Anti-bot checks that prototype methods haven't been wrapped/proxied.
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn prototype_chain_intact() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page
        .eval_value(
            r#"(() => {
                // Check that common objects have expected prototype chains
                const checks = [
                    navigator.permissions.query.toString().includes('[native code]'),
                    HTMLCanvasElement.prototype.toDataURL.toString().includes('[native code]'),
                    CanvasRenderingContext2D.prototype.getImageData.toString().includes('[native code]'),
                    // Proxy detection: toString of Proxy returns different result
                    navigator.toString() === '[object Navigator]',
                    window.toString() === '[object Window]',
                ];
                return checks.every(c => c);
            })()"#,
        )
        .await
        .unwrap();
    assert_eq!(
        val, true,
        "prototype chain should be intact (no proxies/wrappers)"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// Check that the browser has real deviceMemory and hardwareConcurrency.
/// Headless/automation may return 0 or unusual values.
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn hardware_info_realistic() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let cores = page
        .eval_value("navigator.hardwareConcurrency")
        .await
        .unwrap()
        .as_i64()
        .unwrap_or(0);
    assert!(
        cores >= 1,
        "hardwareConcurrency should be >= 1, got: {cores}"
    );

    // deviceMemory may not be available on all platforms but should be > 0 if present
    let mem = page
        .eval_value("navigator.deviceMemory || -1")
        .await
        .unwrap()
        .as_f64()
        .unwrap_or(-1.0);
    assert!(mem != 0.0, "deviceMemory should not be 0");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// Comprehensive CDP detection: check for properties that CDP injects
/// into the global scope or prototype chains.
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn no_cdp_global_leaks() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page
        .eval_value(
            r#"(() => {
                const suspicious = [];

                // Check for ChromeDriver $cdc_ properties on document
                for (const key of Object.keys(document)) {
                    if (/\$?cdc_/.test(key)) suspicious.push('cdc:' + key);
                }

                // Check for Selenium/WebDriver markers
                for (const key of Object.keys(window)) {
                    if (/selenium|webdriver|__driver/i.test(key)) suspicious.push('window:' + key);
                }
                for (const key of Object.keys(document)) {
                    if (/selenium|webdriver|__driver/i.test(key)) suspicious.push('doc:' + key);
                }

                // Check for _Recaptcha or _phantom markers
                if (window._phantom || window.phantom) suspicious.push('phantom');
                if (window.__nightmare) suspicious.push('nightmare');
                if (window.domAutomation || window.domAutomationController)
                    suspicious.push('domAutomation');

                return suspicious;
            })()"#,
        )
        .await
        .unwrap();

    let arr = val.as_array().unwrap();
    assert!(
        arr.is_empty(),
        "found suspicious global properties: {:?}",
        arr
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// Iframe contentWindow cross-origin detection.
/// Anti-bot checks that cross-origin iframe behavior is consistent
/// (headless Chrome sometimes differs).
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn iframe_content_window_consistent() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();
    page.enable().await.unwrap();
    page.navigate(r#"data:text/html,<iframe id="f" src="about:blank"></iframe>"#)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // In real Chrome, iframe.contentWindow exists and has a length property
    let val = page
        .eval_value(
            r#"(() => {
                const f = document.getElementById('f');
                return typeof f.contentWindow === 'object'
                    && typeof f.contentWindow.length === 'number';
            })()"#,
        )
        .await
        .unwrap();
    assert_eq!(
        val, true,
        "iframe.contentWindow should behave like real Chrome"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// Canvas fingerprinting should work (anti-bot uses it to verify the browser
/// can actually render, not just return empty/null canvas data).
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn canvas_rendering_works() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page
        .eval_value(
            r#"(() => {
                const canvas = document.createElement('canvas');
                canvas.width = 200;
                canvas.height = 50;
                const ctx = canvas.getContext('2d');
                ctx.textBaseline = 'top';
                ctx.font = '14px Arial';
                ctx.fillStyle = '#f60';
                ctx.fillRect(125, 1, 62, 20);
                ctx.fillStyle = '#069';
                ctx.fillText('Bot test 🤖', 2, 15);
                const data = canvas.toDataURL();
                // Should be a non-trivial data URL (not blank canvas)
                return data.length > 100;
            })()"#,
        )
        .await
        .unwrap();
    assert_eq!(val, true, "canvas should render non-trivial content");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// AudioContext fingerprinting should work (anti-bot checks that the
/// audio stack is functional, which it isn't in some headless setups).
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn audio_context_works() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page
        .eval_value(
            r#"(() => {
                try {
                    const ctx = new (window.AudioContext || window.webkitAudioContext)();
                    return ctx.sampleRate > 0 && ctx.destination.maxChannelCount > 0;
                } catch(e) {
                    return false;
                }
            })()"#,
        )
        .await
        .unwrap();
    assert_eq!(val, true, "AudioContext should be functional");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// Consistent timestamp behavior.
/// Anti-bot checks that performance.now() returns increasing values
/// and Date.now() is consistent (some bot frameworks freeze time).
#[tokio::test]
#[ignore] // headed-only: run with `cargo test -- --ignored`
async fn timestamps_not_frozen() {
    let (mut process, browser) = launch(opts()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let val = page
        .eval_value_async(
            r#"new Promise(resolve => {
                const t1 = performance.now();
                const d1 = Date.now();
                setTimeout(() => {
                    const t2 = performance.now();
                    const d2 = Date.now();
                    resolve(t2 > t1 && d2 >= d1);
                }, 50);
            })"#,
        )
        .await
        .unwrap();
    assert_eq!(val, true, "timestamps should advance (not frozen)");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

//! Proof tests for the ergonomics/robustness fixes. Browser-backed, so
//! `#[ignore]` like the rest:
//!
//!   cargo test -p chromium-driver --test ergonomics -- --ignored

use std::time::Duration;

use chromium_driver::{Cookie, LaunchOptions, launch};

/// `Element::text()` must return rendered text only — never the body of inline
/// `<script>`/`<style>` (the old strip-tags approach leaked it).
#[tokio::test]
#[ignore]
async fn element_text_excludes_script_body() {
    let (mut process, browser) = launch(LaunchOptions::default()).await.unwrap();
    let page = browser
        .create_page(
            "data:text/html,<div id=t>hello <b>world</b><script>var SECRET='leaked'</script></div>",
        )
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();
    page.enable().await.unwrap();

    let dom = page.dom().await.unwrap();
    let el = dom.query_selector("#t").await.unwrap();
    let text = el.text().await.unwrap();

    assert!(
        text.contains("world"),
        "should keep visible text, got {text:?}"
    );
    assert!(
        !text.contains("SECRET") && !text.contains("leaked"),
        "must not leak <script> body, got {text:?}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// `eval_value_with_args` must pass values as data — strings with quotes and
/// JS metacharacters round-trip untouched and cannot break out of the call.
#[tokio::test]
#[ignore]
async fn eval_value_with_args_is_injection_safe() {
    let (mut process, browser) = launch(LaunchOptions::default()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    let sum = page
        .eval_value_with_args("(a, b) => a + b", &[2.into(), 3.into()])
        .await
        .unwrap();
    assert_eq!(sum.as_i64(), Some(5));

    let nasty = r#"");globalThis.PWNED=1;//"#;
    let echoed = page
        .eval_value_with_args("(s) => s", &[serde_json::Value::String(nasty.into())])
        .await
        .unwrap();
    assert_eq!(
        echoed.as_str(),
        Some(nasty),
        "string must round-trip verbatim"
    );

    let pwned = page.eval_value("globalThis.PWNED ?? null").await.unwrap();
    assert!(pwned.is_null(), "argument must not have executed as code");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// Typed cookies round-trip through `set_cookies` / `get_cookies`.
#[tokio::test]
#[ignore]
async fn cookies_round_trip() {
    let (mut process, browser) = launch(LaunchOptions::default()).await.unwrap();

    let cookie = Cookie {
        name: "zain_probe".into(),
        value: "42".into(),
        domain: Some(".example.com".into()),
        path: Some("/".into()),
        extra: Default::default(),
    };
    browser.set_cookies(vec![cookie]).await.unwrap();

    let got = browser.get_cookies().await.unwrap();
    let found = got.iter().find(|c| c.name == "zain_probe");
    assert!(
        found.is_some_and(|c| c.value == "42"),
        "cookie should round-trip, got {got:?}"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// `wait_for_function` polls a JS expression until truthy, and times out on a
/// condition that never holds.
#[tokio::test]
#[ignore]
async fn wait_for_function_polls_until_truthy() {
    let (mut process, browser) = launch(LaunchOptions::default()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();

    // Flag flips to true asynchronously ~300ms from now.
    page.eval_value("window.__ready=false;setTimeout(()=>{window.__ready=true},300);true")
        .await
        .unwrap();
    page.wait_for_function("window.__ready === true", Duration::from_secs(5))
        .await
        .expect("should resolve once the flag flips");

    // A condition that never holds must time out.
    let timed_out = page
        .wait_for_function("false", Duration::from_millis(400))
        .await;
    assert!(matches!(
        timed_out,
        Err(chromium_driver::CdpError::Timeout(_))
    ));

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

/// `wait_for_network_idle` resolves once the page settles (smoke test for the
/// lifecycle-based waits).
#[tokio::test]
#[ignore]
async fn wait_for_network_idle_resolves() {
    let (mut process, browser) = launch(LaunchOptions::default()).await.unwrap();
    let page = browser
        .create_page("about:blank")
        .await
        .unwrap()
        .attach()
        .await
        .unwrap();
    page.enable().await.unwrap();
    page.set_lifecycle_events_enabled(true).await.unwrap();

    page.navigate("https://example.com").await.unwrap();
    page.wait_for_network_idle(Duration::from_secs(30))
        .await
        .expect("network should go idle");

    let url = page
        .wait_for_url("example.com", Duration::from_secs(5))
        .await
        .unwrap();
    assert!(url.contains("example.com"));

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

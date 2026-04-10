use std::time::Duration;

use chromium_driver::{LaunchOptions, launch};

fn opts() -> LaunchOptions {
    LaunchOptions::default()
}

/// HTML page that embeds an iframe with known content.
const PAGE_WITH_IFRAME: &str = r#"data:text/html,
<html>
<body>
  <h1 id="main-title">Main Page</h1>
  <iframe id="my-frame" srcdoc="
    <html><body>
      <div id='inner-title'>Inside Iframe</div>
      <input id='inner-input' type='text' value=''>
    </body></html>
  "></iframe>
</body>
</html>"#;

/// HTML page with two iframes.
const PAGE_WITH_TWO_IFRAMES: &str = r#"data:text/html,
<html>
<body>
  <iframe id="frame-a" srcdoc="<div id='content'>Frame A</div>"></iframe>
  <iframe id="frame-b" srcdoc="<div id='content'>Frame B</div>"></iframe>
</body>
</html>"#;

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn get_frames_lists_main_and_iframe() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.navigate(PAGE_WITH_IFRAME).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let frames = page.get_frames().await.unwrap();
    assert!(
        frames.len() >= 2,
        "expected main frame + iframe, got {}",
        frames.len()
    );

    // Main frame has no parent
    let main = frames.iter().find(|f| f.parent_id.is_none()).unwrap();
    assert!(main.url.contains("data:text/html"));

    // Child frame has a parent
    let child = frames.iter().find(|f| f.parent_id.is_some()).unwrap();
    assert_eq!(child.parent_id.as_ref(), Some(&main.id));

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn frame_dom_query_inside_iframe() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.navigate(PAGE_WITH_IFRAME).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Find the child frame
    let frames = page.get_frames().await.unwrap();
    let child = frames.iter().find(|f| f.parent_id.is_some()).unwrap();

    // Enter the frame
    let frame = page.frame(&child.id).await.unwrap();
    let dom = frame.dom().await.unwrap();

    // Query inside the iframe
    let el = dom.query_selector("#inner-title").await.unwrap();
    let text = el.text().await.unwrap();
    assert_eq!(text, "Inside Iframe");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn frame_eval_runs_in_iframe_context() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.navigate(PAGE_WITH_IFRAME).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let frames = page.get_frames().await.unwrap();
    let child = frames.iter().find(|f| f.parent_id.is_some()).unwrap();

    let frame = page.frame(&child.id).await.unwrap();

    // Eval in iframe context — should see iframe's DOM
    let title = frame
        .eval_value("document.getElementById('inner-title').textContent")
        .await
        .unwrap();
    assert_eq!(title.as_str().unwrap(), "Inside Iframe");

    // Main page elements should NOT be visible from iframe context
    let main_title = frame
        .eval_value("document.getElementById('main-title')")
        .await
        .unwrap();
    assert!(
        main_title.is_null(),
        "main-title should not exist in iframe"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn two_iframes_isolated_dom() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.navigate(PAGE_WITH_TWO_IFRAMES).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let frames = page.get_frames().await.unwrap();
    let children: Vec<_> = frames.iter().filter(|f| f.parent_id.is_some()).collect();
    assert_eq!(children.len(), 2, "expected 2 child frames");

    // Query each frame sequentially — each dom() call refreshes the DOM tree,
    // so we must finish with one frame before entering the next.
    let frame_a = page.frame(&children[0].id).await.unwrap();
    let text_a = {
        let dom = frame_a.dom().await.unwrap();
        dom.query_selector("#content")
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    };

    let frame_b = page.frame(&children[1].id).await.unwrap();
    let text_b = {
        let dom = frame_b.dom().await.unwrap();
        dom.query_selector("#content")
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    };

    // Both have #content but with different text
    assert_ne!(text_a, text_b);
    assert!(text_a == "Frame A" || text_a == "Frame B");
    assert!(text_b == "Frame A" || text_b == "Frame B");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn wait_for_frame_by_url() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();

    // Navigate to page that creates an iframe after a delay using srcdoc
    // (data: URLs as src create cross-origin frames that CDP can't access)
    page.navigate(r#"data:text/html,<script>setTimeout(()=>{const f=document.createElement('iframe');f.srcdoc='<div id="delayed-content">loaded</div>';f.id='late-frame';document.body.appendChild(f)},300)</script>"#)
        .await
        .unwrap();

    // Wait for child frame to appear (srcdoc frames show as about:srcdoc)
    let frame_info = page
        .wait_for_frame("about:srcdoc", Duration::from_secs(5))
        .await
        .unwrap();

    let frame = page.frame(&frame_info.id).await.unwrap();

    // Eval works in the dynamically added frame
    let text = frame
        .eval_value("document.getElementById('delayed-content')?.textContent")
        .await
        .unwrap();
    assert_eq!(text.as_str().unwrap(), "loaded");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

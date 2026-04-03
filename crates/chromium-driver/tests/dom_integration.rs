use std::time::Duration;

use chromium_driver::dom::Dom;
use chromium_driver::{LaunchOptions, launch};

fn opts() -> LaunchOptions {
    LaunchOptions {
        headless: true,
        ..Default::default()
    }
}

#[tokio::test]
async fn query_selector_and_text() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.navigate("data:text/html,<div id='hello'><span class='msg'>Hello World</span></div>")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let dom = Dom::enable(page.cdp()).await.unwrap();

    let el = dom.query_selector("#hello").await.unwrap();
    let text = el.text().await.unwrap();
    assert!(text.contains("Hello World"), "got: {text}");

    let span = el.query_selector("span.msg").await.unwrap();
    let span_text = span.text().await.unwrap();
    assert_eq!(span_text, "Hello World");

    dom.disable().await.unwrap();
    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn query_selector_all() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.navigate("data:text/html,<ul><li>A</li><li>B</li><li>C</li></ul>")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let dom = Dom::enable(page.cdp()).await.unwrap();

    let items = dom.query_selector_all("li").await.unwrap();
    assert_eq!(items.len(), 3);

    let mut texts = Vec::new();
    for item in &items {
        texts.push(item.text().await.unwrap());
    }
    assert_eq!(texts, vec!["A", "B", "C"]);

    dom.disable().await.unwrap();
    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn try_query_selector_not_found() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.navigate("data:text/html,<div>hi</div>").await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let dom = Dom::enable(page.cdp()).await.unwrap();

    let result = dom.try_query_selector("#nonexistent").await.unwrap();
    assert!(result.is_none());

    dom.disable().await.unwrap();
    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn element_attributes() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.navigate(
        r#"data:text/html,<a id="link" href="https://example.com" class="btn primary">Click</a>"#,
    )
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let dom = Dom::enable(page.cdp()).await.unwrap();

    let el = dom.query_selector("#link").await.unwrap();
    assert_eq!(
        el.attribute("href").await.unwrap().unwrap(),
        "https://example.com"
    );
    assert_eq!(el.attribute("class").await.unwrap().unwrap(), "btn primary");
    assert!(el.attribute("data-nope").await.unwrap().is_none());

    let attrs = el.attributes().await.unwrap();
    assert_eq!(attrs.len(), 3); // id, href, class

    dom.disable().await.unwrap();
    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn element_box_model_and_screenshot() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.navigate(
        "data:text/html,<div id='box' style='width:100px;height:100px;background:red'></div>",
    )
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let dom = Dom::enable(page.cdp()).await.unwrap();

    let el = dom.query_selector("#box").await.unwrap();

    let bm = el.box_model().await.unwrap();
    assert_eq!(bm.width, 100);
    assert_eq!(bm.height, 100);

    let png = el.screenshot_png().await.unwrap();
    assert!(!png.is_empty());
    // PNG magic bytes
    assert_eq!(&png[..4], &[0x89, 0x50, 0x4E, 0x47]);

    dom.disable().await.unwrap();
    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn wait_for_element() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();

    // Navigate to page that creates an element after a delay
    page.navigate(r#"data:text/html,<script>setTimeout(()=>{let d=document.createElement('div');d.id='delayed';d.textContent='appeared';document.body.appendChild(d)},500)</script>"#)
        .await
        .unwrap();

    let dom = Dom::enable(page.cdp()).await.unwrap();

    let el = dom
        .wait_for("#delayed", Duration::from_secs(5))
        .await
        .unwrap();
    let text = el.text().await.unwrap();
    assert_eq!(text, "appeared");

    dom.disable().await.unwrap();
    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn click_and_type() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.navigate(r#"data:text/html,<input id="inp" type="text" value=""><div id="out"></div><script>document.getElementById('inp').addEventListener('input',e=>{document.getElementById('out').textContent=e.target.value})</script>"#)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let dom = Dom::enable(page.cdp()).await.unwrap();

    let input = dom.query_selector("#inp").await.unwrap();
    input.click().await.unwrap();
    input.type_text("hello").await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let out = dom.query_selector("#out").await.unwrap();
    let text = out.text().await.unwrap();
    assert_eq!(text, "hello", "typed text should appear in output div");

    dom.disable().await.unwrap();
    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn press_key_enter() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.navigate(r#"data:text/html,<input id="inp"><div id="out"></div><script>document.getElementById('inp').addEventListener('keydown',e=>{if(e.key==='Enter')document.getElementById('out').textContent='enter_pressed'})</script>"#)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let dom = Dom::enable(page.cdp()).await.unwrap();

    let input = dom.query_selector("#inp").await.unwrap();
    input.click().await.unwrap();
    input.press_key("Enter").await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let out = dom.query_selector("#out").await.unwrap();
    let text = out.text().await.unwrap();
    assert_eq!(text, "enter_pressed");

    dom.disable().await.unwrap();
    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn type_accented_chars() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.navigate(r#"data:text/html,<input id="inp" type="text"><div id="out"></div><script>document.getElementById('inp').addEventListener('input',e=>{document.getElementById('out').textContent=e.target.value})</script>"#)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let dom = Dom::enable(page.cdp()).await.unwrap();

    let input = dom.query_selector("#inp").await.unwrap();
    input.click().await.unwrap();
    input.type_text("ação").await.unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    let out = dom.query_selector("#out").await.unwrap();
    let text = out.text().await.unwrap();
    assert_eq!(text, "ação", "accented chars via dead keys, got: {text}");

    dom.disable().await.unwrap();
    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn type_emoji_via_paste() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.navigate(r#"data:text/html,<input id="inp" type="text"><div id="out"></div><script>document.getElementById('inp').addEventListener('input',e=>{document.getElementById('out').textContent=e.target.value})</script>"#)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let dom = Dom::enable(page.cdp()).await.unwrap();

    let input = dom.query_selector("#inp").await.unwrap();
    input.click().await.unwrap();
    input.type_text("hi 🎉").await.unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    let out = dom.query_selector("#out").await.unwrap();
    let text = out.text().await.unwrap();
    assert_eq!(text, "hi 🎉", "emoji via paste, got: {text}");

    dom.disable().await.unwrap();
    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

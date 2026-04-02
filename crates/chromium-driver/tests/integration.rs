use chromium_driver::cdp::target::{SetDiscoverTargetsParams, TargetCommands};
use chromium_driver::page::PageEvent;
use chromium_driver::{launch, LaunchOptions};

fn opts() -> LaunchOptions {
    LaunchOptions {
        headless: true,
        ..Default::default()
    }
}

#[tokio::test]
async fn launch_and_get_version() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let version = browser.get_version().await.unwrap();
    assert!(!version.product.is_empty());
    assert!(!version.protocol_version.is_empty());
    println!("Browser: {}", version.product);
    println!("Protocol: {}", version.protocol_version);

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn create_page_and_navigate() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();

    page.enable().await.unwrap();

    let nav = page.navigate("data:text/html,<h1>hello</h1>").await.unwrap();
    assert!(!nav.frame_id.0.is_empty());
    assert!(nav.error_text.is_none());

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn navigation_history() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();

    page.set_lifecycle_events_enabled(true).await.unwrap();

    page.navigate("data:text/html,<h1>page1</h1>").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    page.navigate("data:text/html,<h1>page2</h1>").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let history = page.get_navigation_history().await.unwrap();
    assert!(history.entries.len() >= 2);
    assert_eq!(history.current_index, (history.entries.len() - 1) as i64);

    // Navigate back
    let prev = &history.entries[history.current_index as usize - 1];
    page.navigate_to_history_entry(prev.id).await.unwrap();

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn reload_page() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();

    page.navigate("data:text/html,<h1>reload-me</h1>").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    page.reload(false).await.unwrap();
    page.wait_for_load(std::time::Duration::from_secs(5)).await.unwrap();
    page.reload(true).await.unwrap(); // ignore cache

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn multiple_targets() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let t1 = browser.create_page("data:text/html,<h1>tab1</h1>").await.unwrap();
    let t2 = browser.create_page("data:text/html,<h1>tab2</h1>").await.unwrap();

    let targets = browser.get_targets().await.unwrap();
    let page_targets: Vec<_> = targets.iter().filter(|t| t.target_type == "page").collect();
    assert!(
        page_targets.len() >= 2,
        "expected >= 2 page targets, got {}",
        page_targets.len()
    );

    // Attach to both and navigate
    let p1 = t1.attach().await.unwrap();
    let p2 = t2.attach().await.unwrap();

    p1.enable().await.unwrap();
    p2.enable().await.unwrap();

    let nav1 = p1.navigate("data:text/html,<h1>navigated-tab1</h1>").await.unwrap();
    let nav2 = p2.navigate("data:text/html,<h1>navigated-tab2</h1>").await.unwrap();

    assert!(!nav1.frame_id.0.is_empty());
    assert!(!nav2.frame_id.0.is_empty());
    assert_ne!(nav1.frame_id.0, nav2.frame_id.0);

    // Close one target
    let closed = t1.close().await.unwrap();
    assert!(closed);

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn activate_target() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let t1 = browser.create_page("about:blank").await.unwrap();
    let t2 = browser.create_page("about:blank").await.unwrap();

    t1.activate().await.unwrap();
    t2.activate().await.unwrap();

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn discover_targets_raw() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    browser.cdp().target_set_discover_targets(&SetDiscoverTargetsParams { discover: true, filter: None }).await.unwrap();

    browser.create_page("about:blank").await.unwrap();

    let targets = browser.get_targets().await.unwrap();
    assert!(!targets.is_empty());

    browser.cdp().target_set_discover_targets(&SetDiscoverTargetsParams { discover: false, filter: None }).await.unwrap();

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn lifecycle_events_typed() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.set_lifecycle_events_enabled(true).await.unwrap();

    let mut events = page.events();

    page.navigate("data:text/html,<h1>lifecycle</h1>").await.unwrap();

    // Collect typed events
    let mut received = Vec::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(2);
    loop {
        tokio::select! {
            evt = events.recv() => {
                if let Some(evt) = evt {
                    received.push(evt);
                } else {
                    break;
                }
            }
            _ = tokio::time::sleep_until(deadline) => break,
        }
    }

    assert!(!received.is_empty(), "should have received at least one event");

    // Verify we got typed variants, not just Other
    let has_lifecycle = received.iter().any(|e| matches!(e, PageEvent::LifecycleEvent(_)));
    let has_frame_navigated = received.iter().any(|e| matches!(e, PageEvent::FrameNavigated(_)));
    assert!(has_lifecycle, "expected at least one LifecycleEvent, got: {:?}", received);
    assert!(has_frame_navigated, "expected at least one FrameNavigated, got: {:?}", received);

    println!("Received {} typed events", received.len());
    for evt in &received {
        match evt {
            PageEvent::LoadEventFired(e) => println!("  LoadEventFired @ {:?}", e.timestamp),
            PageEvent::DomContentEventFired(e) => {
                println!("  DomContentEventFired @ {:?}", e.timestamp)
            }
            PageEvent::FrameNavigated(e) => println!("  FrameNavigated -> {}", e.frame.url),
            PageEvent::LifecycleEvent(e) => println!("  LifecycleEvent: {}", e.name),
            PageEvent::Other { method, .. } => println!("  Other: {}", method),
        }
    }

    // Disable and verify no more Page events
    page.disable().await.unwrap();

    page.navigate("data:text/html,<h1>after-disable</h1>").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let mut after_disable = Vec::new();
    while let Some(evt) = events.try_recv() {
        if matches!(
            evt,
            PageEvent::LoadEventFired(_)
                | PageEvent::DomContentEventFired(_)
                | PageEvent::FrameNavigated(_)
                | PageEvent::LifecycleEvent(_)
        ) {
            after_disable.push(evt);
        }
    }
    assert!(
        after_disable.is_empty(),
        "should not receive Page events after disable, got: {:?}",
        after_disable
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn browser_events_typed() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    browser.cdp().target_set_discover_targets(&SetDiscoverTargetsParams { discover: true, filter: None }).await.unwrap();

    let mut events = browser.events();

    // Create a target — should trigger TargetCreated
    let target = browser.create_page("about:blank").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let mut received = Vec::new();
    while let Some(evt) = events.try_recv() {
        received.push(evt);
    }

    let has_created = received
        .iter()
        .any(|e| matches!(e, chromium_driver::BrowserEvent::TargetCreated(_)));
    assert!(has_created, "expected TargetCreated event, got: {:?}", received);

    // Close target — should trigger TargetDestroyed
    target.close().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    while let Some(evt) = events.try_recv() {
        received.push(evt);
    }

    let has_destroyed = received
        .iter()
        .any(|e| matches!(e, chromium_driver::BrowserEvent::TargetDestroyed(_)));
    assert!(has_destroyed, "expected TargetDestroyed event, got: {:?}", received);

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn connect_to_running_browser() {
    let (mut process, _browser) = launch(opts()).await.unwrap();

    let browser2 = chromium_driver::connect(process.ws_url()).await.unwrap();

    let version = browser2.get_version().await.unwrap();
    assert!(!version.product.is_empty());

    let target = browser2
        .create_page("data:text/html,<h1>second-conn</h1>")
        .await
        .unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();

    let nav = page.navigate("data:text/html,<h1>works</h1>").await.unwrap();
    assert!(nav.error_text.is_none());

    browser2.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
async fn discovery_http_endpoints() {
    let (mut process, _browser) = launch(opts()).await.unwrap();

    let version = chromium_driver::discovery::get_version("127.0.0.1", process.debug_port)
        .await
        .unwrap();
    assert!(!version.browser.is_empty());
    assert!(version.web_socket_debugger_url.starts_with("ws://"));

    let targets = chromium_driver::discovery::list_targets("127.0.0.1", process.debug_port)
        .await
        .unwrap();
    assert!(!targets.is_empty());

    process.kill().await.unwrap();
}

#[tokio::test]
async fn detach_from_target() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();

    page.navigate("data:text/html,<h1>before-detach</h1>").await.unwrap();
    
    drop(page);
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let targets = browser.get_targets().await.unwrap();
    let still_exists = targets.iter().any(|t| t.target_id == *target.id());
    assert!(still_exists, "target should still exist after detach");

    let page2 = target.attach().await.unwrap();
    page2.enable().await.unwrap();
    let nav = page2
        .navigate("data:text/html,<h1>after-reattach</h1>")
        .await
        .unwrap();
    assert!(nav.error_text.is_none());

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

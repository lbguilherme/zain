use chromium_driver::cdp::browser::{DownloadBehavior, PermissionSetting};
use chromium_driver::cdp::target::{SetDiscoverTargetsParams, TargetCommands};
use chromium_driver::page::PageEvent;
use chromium_driver::{LaunchOptions, launch};

fn opts() -> LaunchOptions {
    LaunchOptions::default()
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
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
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn create_page_and_navigate() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();

    page.enable().await.unwrap();

    let nav = page
        .navigate("data:text/html,<h1>hello</h1>")
        .await
        .unwrap();
    assert!(!nav.frame_id.0.is_empty());
    assert!(nav.error_text.is_none());

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn navigation_history() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();

    page.set_lifecycle_events_enabled(true).await.unwrap();

    page.navigate("data:text/html,<h1>page1</h1>")
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    page.navigate("data:text/html,<h1>page2</h1>")
        .await
        .unwrap();
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
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn reload_page() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();

    page.navigate("data:text/html,<h1>reload-me</h1>")
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    page.reload(false).await.unwrap();
    page.wait_for_load(std::time::Duration::from_secs(5))
        .await
        .unwrap();
    page.reload(true).await.unwrap(); // ignore cache

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn multiple_targets() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let t1 = browser
        .create_page("data:text/html,<h1>tab1</h1>")
        .await
        .unwrap();
    let t2 = browser
        .create_page("data:text/html,<h1>tab2</h1>")
        .await
        .unwrap();

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

    let nav1 = p1
        .navigate("data:text/html,<h1>navigated-tab1</h1>")
        .await
        .unwrap();
    let nav2 = p2
        .navigate("data:text/html,<h1>navigated-tab2</h1>")
        .await
        .unwrap();

    assert!(!nav1.frame_id.0.is_empty());
    assert!(!nav2.frame_id.0.is_empty());
    assert_ne!(nav1.frame_id.0, nav2.frame_id.0);

    // Drop one target — closed via RAII
    let t1_id = t1.id().clone();
    drop(p1);
    drop(t1);

    // Poll until the spawned close completes (may be delayed under load).
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let targets = browser.get_targets().await.unwrap();
        if !targets.iter().any(|t| t.target_id == t1_id) {
            break;
        }
    }
    let targets = browser.get_targets().await.unwrap();
    let t1_exists = targets.iter().any(|t| t.target_id == t1_id);
    assert!(!t1_exists, "t1 should be closed after drop");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
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
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn discover_targets_raw() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    browser
        .cdp()
        .target_set_discover_targets(&SetDiscoverTargetsParams {
            discover: true,
            filter: None,
        })
        .await
        .unwrap();

    browser.create_page("about:blank").await.unwrap();

    let targets = browser.get_targets().await.unwrap();
    assert!(!targets.is_empty());

    browser
        .cdp()
        .target_set_discover_targets(&SetDiscoverTargetsParams {
            discover: false,
            filter: None,
        })
        .await
        .unwrap();

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn lifecycle_events_typed() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();
    page.set_lifecycle_events_enabled(true).await.unwrap();

    let mut events = page.events();

    page.navigate("data:text/html,<h1>lifecycle</h1>")
        .await
        .unwrap();

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

    assert!(
        !received.is_empty(),
        "should have received at least one event"
    );

    // Verify we got typed variants, not just Other
    let has_lifecycle = received
        .iter()
        .any(|e| matches!(e, PageEvent::LifecycleEvent(_)));
    let has_frame_navigated = received
        .iter()
        .any(|e| matches!(e, PageEvent::FrameNavigated(_)));
    assert!(
        has_lifecycle,
        "expected at least one LifecycleEvent, got: {:?}",
        received
    );
    assert!(
        has_frame_navigated,
        "expected at least one FrameNavigated, got: {:?}",
        received
    );

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

    page.navigate("data:text/html,<h1>after-disable</h1>")
        .await
        .unwrap();
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
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn browser_events_typed() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    browser
        .cdp()
        .target_set_discover_targets(&SetDiscoverTargetsParams {
            discover: true,
            filter: None,
        })
        .await
        .unwrap();

    let mut events = browser.events();

    let timeout = std::time::Duration::from_secs(5);

    // Create a target — should trigger TargetCreated
    let target = browser.create_page("about:blank").await.unwrap();

    events
        .wait_for(
            |e| matches!(e, chromium_driver::BrowserEvent::TargetCreated(_)),
            timeout,
        )
        .await
        .expect("expected TargetCreated event");

    // Drop target — should trigger TargetDestroyed via RAII
    drop(target);

    events
        .wait_for(
            |e| matches!(e, chromium_driver::BrowserEvent::TargetDestroyed(_)),
            timeout,
        )
        .await
        .expect("expected TargetDestroyed event");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn discovery_http_version() {
    let (mut process, _browser) = launch(opts()).await.unwrap();

    let version = chromium_driver::discovery::get_version("127.0.0.1", process.debug_port)
        .await
        .unwrap();
    assert!(!version.browser.is_empty());
    assert!(version.web_socket_debugger_url.starts_with("ws://"));

    process.kill().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn detach_from_target() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();

    page.navigate("data:text/html,<h1>before-detach</h1>")
        .await
        .unwrap();

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

// ── Browser fluent API tests ───────────────────────────────────────────────

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn set_and_reset_permissions() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    browser
        .set_permission("geolocation", PermissionSetting::Granted)
        .await
        .unwrap();

    browser
        .set_permission("notifications", PermissionSetting::Denied)
        .await
        .unwrap();

    browser
        .set_permission_for_origin(
            "clipboard-read",
            PermissionSetting::Granted,
            "https://example.com",
        )
        .await
        .unwrap();

    browser.reset_permissions().await.unwrap();

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn set_download_behavior_fluent() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    browser
        .set_download_behavior(DownloadBehavior::Deny, None)
        .await
        .unwrap();

    browser
        .set_download_behavior(DownloadBehavior::Allow, Some("/tmp"))
        .await
        .unwrap();

    browser
        .set_download_behavior(DownloadBehavior::Default, None)
        .await
        .unwrap();

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn get_target_info_from_browser() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser
        .create_page("data:text/html,<h1>info-test</h1>")
        .await
        .unwrap();

    let info = browser.get_target_info(target.id()).await.unwrap();
    assert_eq!(info.target_id, *target.id());
    assert_eq!(info.target_type, "page");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn page_target_info() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser
        .create_page("data:text/html,<h1>target-info</h1>")
        .await
        .unwrap();

    let info = target.info().await.unwrap();
    assert_eq!(info.target_type, "page");
    assert!(info.url.contains("target-info"));

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn window_bounds() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();

    let (window_id, bounds) = browser.get_window_for_target(target.id()).await.unwrap();
    assert!(bounds.width.unwrap_or(0) > 0);
    assert!(bounds.height.unwrap_or(0) > 0);

    browser
        .set_window_bounds(
            window_id,
            chromium_driver::cdp::browser::Bounds {
                width: Some(1024),
                height: Some(768),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let (_, new_bounds) = browser.get_window_for_target(target.id()).await.unwrap();
    assert_eq!(new_bounds.width, Some(1024));
    assert_eq!(new_bounds.height, Some(768));

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn browser_context_create_and_use() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let ctx = browser.create_context().await.unwrap();
    assert!(!ctx.id().0.is_empty());

    let page_target = ctx
        .create_page("data:text/html,<h1>incognito</h1>")
        .await
        .unwrap();

    let info = page_target.info().await.unwrap();
    assert_eq!(info.browser_context_id.as_ref(), Some(ctx.id()));
    assert_eq!(info.target_type, "page");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn browser_context_drop_disposes() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target_id;
    {
        let ctx = browser.create_context().await.unwrap();
        let page = ctx
            .create_page("data:text/html,<h1>drop-test</h1>")
            .await
            .unwrap();
        target_id = page.id().clone();
    }
    // ctx dropped here — dispose fires via tokio::spawn

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let targets = browser.get_targets().await.unwrap();
    let still_exists = targets.iter().any(|t| t.target_id == target_id);
    assert!(!still_exists, "page should be closed after context drop");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn browser_context_isolation() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let ctx1 = browser.create_context().await.unwrap();
    let ctx2 = browser.create_context().await.unwrap();

    assert_ne!(ctx1.id(), ctx2.id());

    let p1 = ctx1.create_page("about:blank").await.unwrap();
    let p2 = ctx2.create_page("about:blank").await.unwrap();

    let info1 = p1.info().await.unwrap();
    let info2 = p2.info().await.unwrap();
    assert_eq!(info1.browser_context_id.as_ref(), Some(ctx1.id()));
    assert_eq!(info2.browser_context_id.as_ref(), Some(ctx2.id()));

    // Drop one context + its page doesn't affect the other
    drop(p1);
    drop(ctx1);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let info2_after = p2.info().await.unwrap();
    assert_eq!(info2_after.target_type, "page");

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

#[tokio::test]
#[ignore] // launches a browser; run with `cargo test -- --ignored`
async fn browser_context_kept_alive_by_page() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target_id;
    let page_target;
    {
        let ctx = browser.create_context().await.unwrap();
        page_target = ctx
            .create_page("data:text/html,<h1>survive</h1>")
            .await
            .unwrap();
        target_id = page_target.id().clone();
        // ctx dropped here, but page_target holds Arc to BrowserContextInner
    }

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // Page should still be alive because it keeps the context alive
    let info = page_target.info().await.unwrap();
    assert_eq!(info.target_type, "page");
    assert!(info.url.contains("survive"));

    // Attach and use the page — proves it's fully functional
    let page = page_target.attach().await.unwrap();
    page.enable().await.unwrap();
    let nav = page
        .navigate("data:text/html,<h1>still-works</h1>")
        .await
        .unwrap();
    assert!(nav.error_text.is_none());

    // Now drop everything — context should be disposed, page gone
    drop(page);
    drop(page_target);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let targets = browser.get_targets().await.unwrap();
    let still_exists = targets.iter().any(|t| t.target_id == target_id);
    assert!(
        !still_exists,
        "page should be gone after last reference dropped"
    );

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

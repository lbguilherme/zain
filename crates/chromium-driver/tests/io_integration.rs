use chromium_driver::cdp::io::{IoCommands, ReadParams, StreamHandle};
use chromium_driver::cdp::runtime::{EvaluateParams, RuntimeCommands};
use chromium_driver::{LaunchOptions, launch};

fn opts() -> LaunchOptions {
    LaunchOptions {
        headless: true,
        ..Default::default()
    }
}

#[tokio::test]
async fn create_blob_and_read_via_io() {
    let (mut process, browser) = launch(opts()).await.unwrap();

    let target = browser.create_page("about:blank").await.unwrap();
    let page = target.attach().await.unwrap();
    page.enable().await.unwrap();

    // Navigate to a real page so we have a JS context.
    page.navigate("data:text/html,<h1>io-test</h1>")
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let cdp = page.cdp();

    // Create a Blob in JS and return its RemoteObject (not by value).
    let blob_result = cdp
        .runtime_evaluate(&EvaluateParams {
            expression: "new Blob(['hello from blob!'], { type: 'text/plain' })".to_owned(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(
        blob_result.exception_details.is_none(),
        "JS exception: {:?}",
        blob_result.exception_details
    );
    let blob_object_id = blob_result
        .result
        .object_id
        .expect("Blob should have an objectId");

    // Resolve the Blob to a UUID via IO.resolveBlob.
    let resolve_ret = cdp.io_resolve_blob(&blob_object_id).await.unwrap();
    assert!(!resolve_ret.uuid.is_empty(), "UUID should not be empty");

    // Build a blob stream handle from the UUID.
    let handle = StreamHandle(format!("blob:{}", resolve_ret.uuid));

    // Read the blob content via IO.read.
    let read_ret = cdp
        .io_read(&ReadParams {
            handle: handle.clone(),
            offset: None,
            size: None,
        })
        .await
        .unwrap();

    let content = if read_ret.base64_encoded.unwrap_or(false) {
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &read_ret.data)
                .expect("valid base64");
        String::from_utf8(bytes).expect("valid utf8")
    } else {
        read_ret.data
    };

    assert_eq!(content, "hello from blob!");
    assert!(read_ret.eof, "should be EOF after reading entire blob");

    // Close the stream.
    cdp.io_close(&handle).await.unwrap();

    // Release the JS object.
    cdp.runtime_release_object(&blob_object_id.0).await.unwrap();

    browser.close().await.unwrap();
    process.wait().await.unwrap();
}

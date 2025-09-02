#![warn(rust_2018_idioms)]

use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{Duration, Instant};
use tokio_test::io::Builder;

#[tokio::test]
async fn read() {
    let mut mock = Builder::new().read(b"hello ").read(b"world!").build();

    let mut buf = [0; 256];

    let n = mock.read(&mut buf).await.expect("read 1");
    assert_eq!(&buf[..n], b"hello ");

    let n = mock.read(&mut buf).await.expect("read 2");
    assert_eq!(&buf[..n], b"world!");
}

#[tokio::test]
async fn read_error() {
    let error = io::Error::new(io::ErrorKind::Other, "cruel");
    let mut mock = Builder::new()
        .read(b"hello ")
        .read_error(error)
        .read(b"world!")
        .build();
    let mut buf = [0; 256];

    let n = mock.read(&mut buf).await.expect("read 1");
    assert_eq!(&buf[..n], b"hello ");

    match mock.read(&mut buf).await {
        Err(error) => {
            assert_eq!(error.kind(), io::ErrorKind::Other);
            assert_eq!("cruel", format!("{error}"));
        }
        Ok(_) => panic!("error not received"),
    }

    let n = mock.read(&mut buf).await.expect("read 1");
    assert_eq!(&buf[..n], b"world!");
}

#[tokio::test]
async fn write() {
    let mut mock = Builder::new().write(b"hello ").write(b"world!").build();

    mock.write_all(b"hello ").await.expect("write 1");
    mock.write_all(b"world!").await.expect("write 2");
}

#[tokio::test]
async fn write_with_handle() {
    let (mut mock, mut handle) = Builder::new().build_with_handle();
    handle.write(b"hello ");
    handle.write(b"world!");

    mock.write_all(b"hello ").await.expect("write 1");
    mock.write_all(b"world!").await.expect("write 2");
}

#[tokio::test]
async fn read_with_handle() {
    let (mut mock, mut handle) = Builder::new().build_with_handle();
    handle.read(b"hello ");
    handle.read(b"world!");

    let mut buf = vec![0; 6];
    mock.read_exact(&mut buf).await.expect("read 1");
    assert_eq!(&buf[..], b"hello ");
    mock.read_exact(&mut buf).await.expect("read 2");
    assert_eq!(&buf[..], b"world!");
}

#[tokio::test]
async fn write_error() {
    let error = io::Error::new(io::ErrorKind::Other, "cruel");
    let mut mock = Builder::new()
        .write(b"hello ")
        .write_error(error)
        .write(b"world!")
        .build();
    mock.write_all(b"hello ").await.expect("write 1");

    match mock.write_all(b"whoa").await {
        Err(error) => {
            assert_eq!(error.kind(), io::ErrorKind::Other);
            assert_eq!("cruel", format!("{error}"));
        }
        Ok(_) => panic!("error not received"),
    }

    mock.write_all(b"world!").await.expect("write 2");
}

#[tokio::test]
#[should_panic]
async fn mock_panics_read_data_left() {
    use tokio_test::io::Builder;
    Builder::new().read(b"read").build();
}

#[tokio::test]
#[should_panic]
async fn mock_panics_write_data_left() {
    use tokio_test::io::Builder;
    Builder::new().write(b"write").build();
}

#[tokio::test(start_paused = true)]
async fn wait() {
    const FIRST_WAIT: Duration = Duration::from_secs(1);

    let mut mock = Builder::new()
        .wait(FIRST_WAIT)
        .read(b"hello ")
        .read(b"world!")
        .build();

    let mut buf = [0; 256];

    let start = Instant::now(); // record the time the read call takes
                                //
    let n = mock.read(&mut buf).await.expect("read 1");
    assert_eq!(&buf[..n], b"hello ");
    println!("time elapsed after first read {:?}", start.elapsed());

    let n = mock.read(&mut buf).await.expect("read 2");
    assert_eq!(&buf[..n], b"world!");
    println!("time elapsed after second read {:?}", start.elapsed());

    // make sure the .wait() instruction worked
    assert!(
        start.elapsed() >= FIRST_WAIT,
        "consuming the whole mock only took {}ms",
        start.elapsed().as_millis()
    );
}

#[tokio::test(start_paused = true)]
async fn multiple_wait() {
    const FIRST_WAIT: Duration = Duration::from_secs(1);
    const SECOND_WAIT: Duration = Duration::from_secs(1);

    let mut mock = Builder::new()
        .wait(FIRST_WAIT)
        .read(b"hello ")
        .wait(SECOND_WAIT)
        .read(b"world!")
        .build();

    let mut buf = [0; 256];

    let start = Instant::now(); // record the time it takes to consume the mock

    let n = mock.read(&mut buf).await.expect("read 1");
    assert_eq!(&buf[..n], b"hello ");
    println!("time elapsed after first read {:?}", start.elapsed());

    let n = mock.read(&mut buf).await.expect("read 2");
    assert_eq!(&buf[..n], b"world!");
    println!("time elapsed after second read {:?}", start.elapsed());

    // make sure the .wait() instruction worked
    assert!(
        start.elapsed() >= FIRST_WAIT + SECOND_WAIT,
        "consuming the whole mock only took {}ms",
        start.elapsed().as_millis()
    );
}

#[tokio::test]
async fn shutdown() {
    let mut mock = Builder::new().write(b"hello").shutdown().build();

    let _ = mock.write(b"hello").await.unwrap();
    mock.shutdown().await.unwrap();
}

#[tokio::test]
async fn shutdown_error() {
    let mut mock = Builder::new()
        .write(b"data")
        .shutdown_error(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "forced close",
        ))
        .build();

    mock.write_all(b"data").await.unwrap();
    let result = mock.shutdown().await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::ConnectionAborted);
}

#[tokio::test]
async fn read_write_shutdown() {
    let mut mock = Builder::new()
        .write(b"hello world")
        .read(b"goodbye world")
        .shutdown()
        .build();

    mock.write_all(b"hello world").await.unwrap();

    let mut response = [0; 256];
    let n = mock.read(&mut response).await.unwrap();
    assert_eq!(&response[..n], b"goodbye world");

    mock.shutdown().await.unwrap();
}

#[tokio::test]
async fn wait_shutdown() {
    let mut mock = Builder::new()
        .write(b"hellohello")
        .wait(Duration::from_millis(50))
        .shutdown()
        .build();

    let start = std::time::Instant::now();

    mock.write_all(b"hellohello").await.unwrap();
    mock.shutdown().await.unwrap();

    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(45)); // Allow some timing variance
}

#[tokio::test]
async fn shutdown_with_handle() {
    let (mut mock, mut handle) = Builder::new().write(b"initial data").build_with_handle();
    handle.shutdown();

    mock.write_all(b"initial data").await.unwrap();
    mock.shutdown().await.unwrap();
}

#[tokio::test]
async fn shutdown_error_with_handle() {
    let (mut mock, mut handle) = Builder::new().write(b"data").build_with_handle();

    handle.shutdown_error(io::Error::new(io::ErrorKind::BrokenPipe, "pipe broken"));

    mock.write_all(b"data").await.unwrap();

    let result = mock.shutdown().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
}

#[tokio::test]
#[should_panic(expected = "unexpected shutdown")]
async fn unexpected_shutdown_expecting_write() {
    let mut mock = Builder::new().write(b"expected data").build();

    // Try to shutdown before writing expected data
    mock.shutdown().await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "unexpected shutdown")]
async fn unexpected_shutdown_expecting_read() {
    let mut mock = Builder::new().read(b"some data").build();

    mock.shutdown().await.unwrap();
}

#[tokio::test]
async fn multiple_operations_with_shutdown() {
    let mut mock = Builder::new()
        .write(b"ping")
        .read(b"pong")
        .write(b"bing")
        .read(b"bong")
        .shutdown()
        .build();

    mock.write_all(b"ping").await.unwrap();
    let mut buf = [0; 4];
    mock.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"pong");

    mock.write_all(b"bing").await.unwrap();
    let mut buf = [0; 4];
    mock.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"bong");

    mock.shutdown().await.unwrap();
}

#[tokio::test]
async fn shutdown_after_read_error() {
    let mut mock = Builder::new()
        .write(b"request")
        .read_error(io::Error::new(
            io::ErrorKind::ConnectionReset,
            "connection lost",
        ))
        .shutdown()
        .build();

    mock.write_all(b"request").await.unwrap();

    let mut buf = [0; 10];
    let read_result = mock.read(&mut buf).await;
    assert!(read_result.is_err());
    assert_eq!(
        read_result.unwrap_err().kind(),
        io::ErrorKind::ConnectionReset
    );

    // But we should still be able to shutdown
    mock.shutdown().await.unwrap();
}

#[tokio::test]
async fn shutdown_after_write_error() {
    let mut mock = Builder::new()
        .write_error(io::Error::new(io::ErrorKind::NotConnected, "not connected"))
        .shutdown()
        .build();

    let write_result = mock.write_all(b"data").await;
    assert!(write_result.is_err());
    assert_eq!(
        write_result.unwrap_err().kind(),
        io::ErrorKind::NotConnected
    );

    mock.shutdown().await.unwrap();
}

#[tokio::test]
async fn partial_write_then_shutdown() {
    let mut mock = Builder::new().write(b"hel").write(b"lo").shutdown().build();

    // Write all data at once - should match both write actions
    mock.write_all(b"hello").await.unwrap();
    mock.shutdown().await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "write after shutdown")]
async fn invalid_builder_seq() {
    let mut _mock = Builder::new()
        .write(b"hel")
        .write(b"lo")
        .shutdown()
        .write(b"bye")
        .build();
}

#[tokio::test]
#[should_panic(expected = "write after shutdown")]
async fn invalid_handle_call() {
    let (mut mock, mut handle) = Builder::new()
        .write(b"hel")
        .write(b"lo")
        .shutdown()
        .build_with_handle();

    let _ = mock.write(b"hello").await;
    handle.write(b" world");
}

#[tokio::test]
async fn cancel_with_handle() {
    tokio::select! {
        biased;
        _ = async {
                let payload = b"writethisplease";

                let (mut a,mut handle) = tokio_test::io::Builder::new()
                    .write(payload)
                    .build_with_handle();

                let _  = a.write(payload).await;
                handle.shutdown();
                let mut buf = [0;32];

                // `read` will return Poll::Pending because the handle is live
                // This will cancel this future and drop Mock, which should NOT panic.
                let _ = a.read(&mut buf).await;
        } => {},
        _ = tokio::task::yield_now() => {}
    }
}

#[tokio::test]
#[should_panic(expected = "There is still data left to write. (1 actions remain)")]
async fn cancel_panic_with_handle() {
    tokio::select! {
        biased;
        _ = async {
                let payload = b"writethisplease";

                let (mut a,mut handle) = tokio_test::io::Builder::new()
                    .write(payload)
                    .build_with_handle();

                let _  = a.write(payload).await;
                handle.write(payload);

                let mut buf = [0;32];
                let _ = a.read(&mut buf).await;
        } => {},
        _ = tokio::task::yield_now() => {}
    }
}

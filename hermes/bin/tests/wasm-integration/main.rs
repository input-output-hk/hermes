//! Integration tests for Hermes WASM components
#![allow(clippy::all, unused, missing_docs)]

// SEE: https://docs.rs/libtest-mimic/latest/libtest_mimic/index.html

/// A parameter identifier specifying the directory for placing test WebAssembly
/// components.
const ENV_MODULE_DIR: &str = "TEST_WASM_MODULE_DIR";
/// A parameter identifier specifying the number of tests to run.
const ENV_N_TEST: &str = "N_TEST";
/// A parameter identifier specifying the number of benchmarks to run.
const ENV_N_BENCH: &str = "N_BENCH";
/// A standard value assigned to `ENV_MODULE_DIR` when it's not specified.
const DEFAULT_ENV_MODULE_DIR: &str = "../../wasm/test-components";
/// The default value for the number of tests to run when not specified.
const DEFAULT_ENV_N_TEST: &str = "32";
/// The default value for the number of benchmarks to run when not specified.
const DEFAULT_ENV_N_BENCH: &str = "32";

use std::{
    convert::Infallible,
    env,
    error::Error,
    ffi::OsStr,
    fs,
    net::SocketAddr,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use hermes::{
    runtime_extensions::hermes::integration_test::event::{execute_event, EventType},
    wasm::module::Module,
};
use http_body_util::Full;
use hyper::{body::Bytes, server::conn::http1, service::service_fn, Request, Response};
use hyper_util::rt::TokioIo;
use libtest_mimic::{Arguments, Failed, Measurement, Trial};
use tokio::{
    net::TcpListener,
    runtime::Builder,
    sync::{oneshot, Notify},
};
use tracing::{level_filters::LevelFilter, subscriber::SetGlobalDefaultError};
use tracing_subscriber::{fmt::time, FmtSubscriber};

/// Init the logger
#[allow(dead_code)]
fn init_logger() -> Result<(), SetGlobalDefaultError> {
    let subscriber = FmtSubscriber::builder()
        .json()
        .with_level(true)
        .with_thread_names(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_timer(time::UtcTime::rfc_3339())
        .with_max_level(LevelFilter::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
}

/// Initialize the IPFS node
fn init_ipfs() -> anyhow::Result<()> {
    let base_dir = temp_dir::TempDir::new()?;
    // disable bootstrapping the IPFS node to default addresses for testing
    let default_bootstrap = false;
    hermes::ipfs::bootstrap(base_dir.path(), default_bootstrap)
}

async fn hello(_: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

async fn spin_up_server(shutdown: Arc<AtomicBool>) -> anyhow::Result<()> {
    println!("ABC1");
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    loop {
        println!("ABC2");
        // Check shutdown flag before accepting new connections
        if shutdown.load(Ordering::Relaxed) {
            println!("Shutdown signal received, stopping server...");
            return Ok(());
        }
        println!("ABC3");

        // Use tokio::select! to wait for either a new connection or shutdown signal
        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = result?;
                let io = TokioIo::new(stream);

                // Spawn a new task for each connection
                tokio::task::spawn(async move {
                    println!("New connection accepted");
                    if let Err(err) = http1::Builder::new()
                        .serve_connection(io, service_fn(hello))
                        .await
                    {
                        eprintln!("Error serving connection: {:?}", err);
                    }
                });
            },
            _ = async {
                // Check shutdown flag periodically
                while !shutdown.load(Ordering::Relaxed) {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            } => {
                println!("Shutdown signal received during accept");
                return Ok(())
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // This is necessary otherwise the logging functions inside hermes are silent during the
    // test run.
    // init_logger()?;
    // This causes issues with normal test runs, so comment out for now.
    // info!("Starting Hermes WASM integration tests");

    init_ipfs()?;

    let args = Arguments::from_args();
    let tests = collect_tests()?;
    libtest_mimic::run(&args, tests).exit();
}

/// Collect all the tests to run from a specified directory
fn visit_dir(path: &Path, tests: &mut Vec<Trial>) -> Result<(), Box<dyn Error>> {
    let args = Arguments::from_args();

    let n_test: u32 = env::var(ENV_N_TEST)
        .unwrap_or_else(|_| DEFAULT_ENV_N_TEST.to_owned())
        .parse()?;
    let n_bench: u32 = env::var(ENV_N_BENCH)
        .unwrap_or_else(|_| DEFAULT_ENV_N_BENCH.to_owned())
        .parse()?;

    let raw_entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_e) => return Ok(()), // If the directory of modules can not be found, just skip it.
    };

    let entries: Vec<_> = raw_entries
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.file_type().map(|file_type| (file_type, entry.path())))
        .collect::<Result<_, _>>()?;

    let wasm_file_paths: Vec<_> = entries
        .iter()
        .filter_map(|(file_type, path)| {
            (file_type.is_file() && path.extension() == Some(OsStr::new("wasm"))).then_some(path)
        })
        .collect();

    let dir_paths: Vec<_> = entries
        .iter()
        .filter_map(|(file_type, path)| file_type.is_dir().then_some(path))
        .collect();

    // process `.wasm` files
    for file_path in wasm_file_paths {
        let name = file_path.strip_prefix(path)?.display().to_string();

        // Execute the wasm tests to get their name
        // Load WASM module in the executor.
        let wasm_buf = fs::read(file_path)?;
        let mut module = Module::from_bytes(&wasm_buf)?;

        let mut collect = |event_type: EventType, n: u32| -> Result<(), Box<dyn Error>> {
            // Collect the cases in a loop until no more cases.
            for i in 0..n {
                match execute_event(&mut module, i, false, event_type)? {
                    Some(result) => {
                        let path_string = file_path.to_string_lossy().to_string();

                        let test = match event_type {
                            EventType::Test => {
                                Trial::test(result.name, move || {
                                    let shutdown = Arc::new(AtomicBool::new(false));
                                    let cloned_shutdown = Arc::clone(&shutdown);
                                    println!("1");
                                    // TODO: If test needs server
                                    println!("2");
                                    let runtime =
                                        Builder::new_multi_thread().enable_all().build().unwrap();

                                    // Spawn the server in the background
                                    let server_handle = runtime.spawn(async move {
                                        println!("3");
                                        spin_up_server(Arc::clone(&cloned_shutdown)).await
                                    });

                                    // Give the server a moment to start up
                                    std::thread::sleep(std::time::Duration::from_millis(100));

                                    println!("5");
                                    let result = execute_test(i, path_string, event_type);
                                    println!("6");

                                    // Signal shutdown
                                    std::thread::sleep(std::time::Duration::from_millis(5000));
                                    shutdown.store(true, Ordering::SeqCst);
                                    println!("7");

                                    // Wait for server to shut down gracefully
                                    runtime.block_on(server_handle).unwrap();

                                    result
                                })
                            },
                            EventType::Bench => {
                                Trial::bench(result.name, move |test_mode| {
                                    execute_bench(test_mode, i, path_string, event_type)
                                })
                            },
                        }
                        .with_kind(name.clone());

                        tests.push(test);
                    },
                    _ => {
                        break;
                    },
                }
            }

            Ok(())
        };

        if args.test || args.list {
            collect(EventType::Test, n_test)?;
        }
        if args.bench || args.list {
            collect(EventType::Bench, n_bench)?;
        }
    }

    // process items inside directories
    for path in dir_paths {
        visit_dir(path, tests)?;
    }

    Ok(())
}

/// Creates as many tests as required for each `.wasm` file in the current directory or
/// sub-directories of the current directory.
fn collect_tests() -> Result<Vec<Trial>, Box<dyn Error>> {
    // Read wasm components to be test in a directory.
    let mut tests = Vec::new();
    let test_module_dir =
        env::var(ENV_MODULE_DIR).unwrap_or_else(|_| DEFAULT_ENV_MODULE_DIR.to_owned());
    let path = Path::new(&test_module_dir);

    visit_dir(path, &mut tests)?;

    Ok(tests)
}

/// Executes a test for a wasm component.
fn execute(test_case: u32, path: String, event_type: EventType) -> Result<(), Failed> {
    let wasm_buf = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;

    let mut module = Module::from_bytes(&wasm_buf)?;

    match execute_event(&mut module, test_case, true, event_type)? {
        Some(result) => {
            if result.status {
                Ok(())
            } else {
                Err(Failed::from("Failed"))
            }
        },
        _ => Err(Failed::from("result unexpected")),
    }
}

/// Executes a test for a wasm component.
fn execute_test(test_case: u32, path: String, event_type: EventType) -> Result<(), Failed> {
    execute(test_case, path, event_type)
}

/// Executes a test for a wasm component.
fn execute_bench(
    test_mode: bool, test_case: u32, path: String, event_type: EventType,
) -> Result<Option<Measurement>, Failed> {
    if test_mode {
        Ok(None)
    } else {
        let start_time = Instant::now();

        execute(test_case, path, event_type)?;

        let elapsed_time = start_time.elapsed().as_nanos();

        Ok(Some(Measurement {
            avg: u64::try_from(elapsed_time)?,
            variance: 0,
        }))
    }
}

//! # PyO3 Asyncio Testing Utilities
//!
//! This module provides some utilities for parsing test arguments as well as running and filtering
//! a sequence of tests.
//!
//! As mentioned [here](crate#pythons-event-loop), PyO3 Asyncio tests cannot use the default test
//! harness since it doesn't allow Python to gain control over the main thread. Instead, we have to
//! provide our own test harness in order to create integration tests.
//!
//! Running `pyo3-asyncio` code in doc tests _is_ supported however since each doc test has its own
//! `main` function. When writing doc tests, you may use the
//! [`#[pyo3_asyncio::async_std::main]`](crate::async_std::main) or
//! [`#[pyo3_asyncio::tokio::main]`](crate::tokio::main) macros on the test's main function to run
//! your test.
//!
//! If you don't want to write doc tests, you're unfortunately stuck with integration tests since
//! lib tests do not offer the same level of flexibility for the `main` fn. That being said,
//! overriding the default test harness can be quite different from what you're used to doing for
//! integration tests, so these next sections will walk you through this process.
//!
//! ## Main Test File
//! First, we need to create the test's main file. Although these tests are considered integration
//! tests, we cannot put them in the `tests` directory since that is a special directory owned by
//! Cargo. Instead, we put our tests in a `pytests` directory.
//!
//! > The name `pytests` is just a convention. You can name this folder anything you want in your own
//! > projects.
//!
//! We'll also want to provide the test's main function. Most of the functionality that the test harness needs is packed in the [`pyo3_asyncio::testing::main`](https://docs.rs/pyo3-asyncio/latest/pyo3_asyncio/testing/fn.main.html) function. This function will parse the test's CLI arguments, collect and pass the functions marked with [`#[pyo3_asyncio::async_std::test]`](https://docs.rs/pyo3-asyncio/latest/pyo3_asyncio/async_std/attr.test.html) or [`#[pyo3_asyncio::tokio::test]`](https://docs.rs/pyo3-asyncio/latest/pyo3_asyncio/tokio/attr.test.html) and pass them into the test harness for running and filtering.
//!
//! `pytests/test_example.rs` for the `tokio` runtime:
//! ```rust
//! # #[cfg(all(feature = "tokio-runtime", feature = "attributes"))]
//! #[pyo3_asyncio::tokio::main]
//! async fn main() -> pyo3::PyResult<()> {
//!     pyo3_asyncio::testing::main().await
//! }
//! # #[cfg(not(all(feature = "tokio-runtime", feature = "attributes")))]
//! # fn main() {}
//! ```
//!
//! `pytests/test_example.rs` for the `async-std` runtime:
//! ```rust
//! # #[cfg(all(feature = "async-std-runtime", feature = "attributes"))]
//! #[pyo3_asyncio::async_std::main]
//! async fn main() -> pyo3::PyResult<()> {
//!     pyo3_asyncio::testing::main().await
//! }
//! # #[cfg(not(all(feature = "async-std-runtime", feature = "attributes")))]
//! # fn main() {}
//! ```
//!
//! ## Cargo Configuration
//! Next, we need to add our test file to the Cargo manifest by adding the following section to the
//! `Cargo.toml`
//!
//! ```toml
//! [[test]]
//! name = "test_example"
//! path = "pytests/test_example.rs"
//! harness = false
//! ```
//!
//! Also add the `testing` and `attributes` features to the `pyo3-asyncio` dependency and select your preferred runtime:
//!
//! ```toml
//! pyo3-asyncio = { version = "0.13", features = ["testing", "attributes", "async-std-runtime"] }
//! ```
//!
//! At this point, you should be able to run the test via `cargo test`
//!
//! ### Adding Tests to the PyO3 Asyncio Test Harness
//!
//! We can add tests anywhere in the test crate with the runtime's corresponding `#[test]` attribute:
//!
//! For `async-std` use the [`pyo3_asyncio::async_std::test`](https://docs.rs/pyo3-asyncio/latest/pyo3_asyncio/async_std/attr.test.html) attribute:
//! ```rust
//! # #[cfg(all(feature = "async-std-runtime", feature = "attributes"))]
//! mod tests {
//!     use std::{time::Duration, thread};
//!
//!     use pyo3::prelude::*;
//!
//!     // tests can be async
//!     #[pyo3_asyncio::async_std::test]
//!     async fn test_async_sleep() -> PyResult<()> {
//!         async_std::task::sleep(Duration::from_secs(1)).await;
//!         Ok(())
//!     }
//!
//!     // they can also be synchronous
//!     #[pyo3_asyncio::async_std::test]
//!     fn test_blocking_sleep() -> PyResult<()> {
//!         thread::sleep(Duration::from_secs(1));
//!         Ok(())
//!     }
//! }
//!
//! # #[cfg(all(feature = "async-std-runtime", feature = "attributes"))]
//! #[pyo3_asyncio::async_std::main]
//! async fn main() -> pyo3::PyResult<()> {
//!     pyo3_asyncio::testing::main().await
//! }
//! # #[cfg(not(all(feature = "async-std-runtime", feature = "attributes")))]
//! # fn main() {}
//! ```
//!
//! For `tokio` use the [`pyo3_asyncio::tokio::test`](https://docs.rs/pyo3-asyncio/latest/pyo3_asyncio/tokio/attr.test.html) attribute:
//! ```rust
//! # #[cfg(all(feature = "tokio-runtime", feature = "attributes"))]
//! mod tests {
//!     use std::{time::Duration, thread};
//!
//!     use pyo3::prelude::*;
//!
//!     // tests can be async
//!     #[pyo3_asyncio::tokio::test]
//!     async fn test_async_sleep() -> PyResult<()> {
//!         tokio::time::sleep(Duration::from_secs(1)).await;
//!         Ok(())
//!     }
//!
//!     // they can also be synchronous
//!     #[pyo3_asyncio::tokio::test]
//!     fn test_blocking_sleep() -> PyResult<()> {
//!         thread::sleep(Duration::from_secs(1));
//!         Ok(())
//!     }
//! }
//!
//! # #[cfg(all(feature = "tokio-runtime", feature = "attributes"))]
//! #[pyo3_asyncio::tokio::main]
//! async fn main() -> pyo3::PyResult<()> {
//!     pyo3_asyncio::testing::main().await
//! }
//! # #[cfg(not(all(feature = "tokio-runtime", feature = "attributes")))]
//! # fn main() {}
//! ```
//!
//! ## Lib Tests
//!
//! Unfortunately, as we mentioned at the beginning, these utilities will only run in integration
//! tests and doc tests. Running lib tests are out of the question since we need control over the
//! main function. You can however perform compilation checks for lib tests. This is much more
//! useful in doc tests than it is for lib tests, but the option is there if you want it.
//!
//! `my-crate/src/lib.rs`
//! ```
//! # #[cfg(all(
//! #     any(feature = "async-std-runtime", feature = "tokio-runtime"),
//! #     feature = "attributes"
//! # ))]
//! mod tests {
//!     use pyo3::prelude::*;
//!
//! #   #[cfg(feature = "async-std-runtime")]
//!     #[pyo3_asyncio::async_std::test]
//!     async fn test_async_std_async_test_compiles() -> PyResult<()> {
//!         Ok(())
//!     }
//! #   #[cfg(feature = "async-std-runtime")]
//!     #[pyo3_asyncio::async_std::test]
//!     fn test_async_std_sync_test_compiles() -> PyResult<()> {
//!         Ok(())
//!     }
//!
//! #   #[cfg(feature = "tokio-runtime")]
//!     #[pyo3_asyncio::tokio::test]
//!     async fn test_tokio_async_test_compiles() -> PyResult<()> {
//!         Ok(())
//!     }
//! #   #[cfg(feature = "tokio-runtime")]
//!     #[pyo3_asyncio::tokio::test]
//!     fn test_tokio_sync_test_compiles() -> PyResult<()> {
//!         Ok(())
//!     }
//! }
//!
//! # fn main() {}
//! ```

use std::{future::Future, pin::Pin};

use clap::{Arg, Command};
use futures::stream::{self, StreamExt};
use pyo3::prelude::*;

/// Args that should be provided to the test program
///
/// These args are meant to mirror the default test harness's args.
/// > Currently only `--filter` is supported.
pub struct Args {
    filter: Option<String>,
}

impl Default for Args {
    fn default() -> Self {
        Self { filter: None }
    }
}

/// Parse the test args from the command line
///
/// This should be called at the start of your test harness to give the CLI some
/// control over how our tests are run.
///
/// Ideally, we should mirror the default test harness's arguments exactly, but
/// for the sake of simplicity, only filtering is supported for now. If you want
/// more features, feel free to request them
/// [here](https://github.com/awestlake87/pyo3-asyncio/issues).
///
/// # Examples
///
/// Running the following function:
/// ```
/// # use pyo3_asyncio::testing::parse_args;
/// let args = parse_args();
/// ```
///
/// Produces the following usage string:
///
/// ```bash
/// Pyo3 Asyncio Test Suite
/// USAGE:
/// test_example [TESTNAME]
///
/// FLAGS:
/// -h, --help       Prints help information
/// -V, --version    Prints version information
///
/// ARGS:
/// <TESTNAME>    If specified, only run tests containing this string in their names
/// ```
pub fn parse_args() -> Args {
    let matches = Command::new("PyO3 Asyncio Test Suite")
        .arg(
            Arg::new("TESTNAME")
                .help("If specified, only run tests containing this string in their names"),
        )
        .get_matches();

    Args {
        filter: matches.value_of("TESTNAME").map(|name| name.to_string()),
    }
}

type TestFn = dyn Fn() -> Pin<Box<dyn Future<Output = PyResult<()>> + Send>> + Send + Sync;

/// The structure used by the `#[test]` macros to provide a test to the `pyo3-asyncio` test harness.
#[derive(Clone)]
pub struct Test {
    /// The fully qualified name of the test
    pub name: &'static str,
    /// The function used to create the task that runs the test.
    pub test_fn: &'static TestFn,
}

impl Test {
    /// Create the task that runs the test
    pub fn task(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = pyo3::PyResult<()>> + Send>> {
        (self.test_fn)()
    }
}

inventory::collect!(Test);

/// Run a sequence of tests while applying any necessary filtering from the `Args`
pub async fn test_harness(tests: Vec<Test>, args: Args) -> PyResult<()> {
    stream::iter(tests)
        .for_each_concurrent(Some(4), |test| {
            let mut ignore = false;

            if let Some(filter) = args.filter.as_ref() {
                if !test.name.contains(filter) {
                    ignore = true;
                }
            }

            async move {
                if !ignore {
                    test.task().await.unwrap();

                    println!("test {} ... ok", test.name);
                }
            }
        })
        .await;

    Ok(())
}

/// Parses test arguments and passes the tests to the `pyo3-asyncio` test harness
///
/// This function collects the test structures from the `inventory` boilerplate and forwards them to
/// the test harness.
///
/// # Examples
///
/// ```
/// # #[cfg(all(feature = "async-std-runtime", feature = "attributes"))]
/// use pyo3::prelude::*;
///
/// # #[cfg(all(feature = "async-std-runtime", feature = "attributes"))]
/// #[pyo3_asyncio::async_std::main]
/// async fn main() -> PyResult<()> {
///     pyo3_asyncio::testing::main().await
/// }
/// # #[cfg(not(all(feature = "async-std-runtime", feature = "attributes")))]
/// # fn main() { }
/// ```
pub async fn main() -> PyResult<()> {
    let args = parse_args();

    test_harness(
        inventory::iter::<Test>().map(|test| test.clone()).collect(),
        args,
    )
    .await
}

#[cfg(test)]
#[cfg(all(
    feature = "testing",
    feature = "attributes",
    any(feature = "async-std-runtime", feature = "tokio-runtime")
))]
mod tests {
    use pyo3::prelude::*;

    use crate as pyo3_asyncio;

    #[cfg(feature = "async-std-runtime")]
    #[pyo3_asyncio::async_std::test]
    async fn test_async_std_async_test_compiles() -> PyResult<()> {
        Ok(())
    }
    #[cfg(feature = "async-std-runtime")]
    #[pyo3_asyncio::async_std::test]
    fn test_async_std_sync_test_compiles() -> PyResult<()> {
        Ok(())
    }

    #[cfg(feature = "tokio-runtime")]
    #[pyo3_asyncio::tokio::test]
    async fn test_tokio_async_test_compiles() -> PyResult<()> {
        Ok(())
    }
    #[cfg(feature = "tokio-runtime")]
    #[pyo3_asyncio::tokio::test]
    fn test_tokio_sync_test_compiles() -> PyResult<()> {
        Ok(())
    }
}

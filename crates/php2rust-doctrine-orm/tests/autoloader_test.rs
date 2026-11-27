//! Port of tests/Tests/Proxy/AutoloaderTest.php. `DIRECTORY_SEPARATOR`
//! is `std::path::MAIN_SEPARATOR`.
//!
//! Upstream: doctrine/orm tests/Tests/Proxy/AutoloaderTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::path::MAIN_SEPARATOR as DIRECTORY_SEPARATOR;

use php2rust_doctrine_orm::proxy::Autoloader;

fn data_resolve_file() -> Vec<(&'static str, &'static str, &'static str, String)> {
    vec![
        (
            "/tmp",
            "MyProxy",
            "MyProxy\\RealClass",
            format!("/tmp{DIRECTORY_SEPARATOR}RealClass.php"),
        ),
        (
            "/tmp",
            "MyProxy",
            "MyProxy\\__CG__\\RealClass",
            format!("/tmp{DIRECTORY_SEPARATOR}__CG__RealClass.php"),
        ),
        (
            "/tmp",
            "MyProxy\\Subdir",
            "MyProxy\\Subdir\\__CG__\\RealClass",
            format!("/tmp{DIRECTORY_SEPARATOR}__CG__RealClass.php"),
        ),
        (
            "/tmp",
            "MyProxy",
            "MyProxy\\__CG__\\Other\\RealClass",
            format!("/tmp{DIRECTORY_SEPARATOR}__CG__OtherRealClass.php"),
        ),
    ]
}

#[test]
fn testResolveFile() {
    for (proxy_dir, proxy_namespace, class_name, expected_proxy_file) in data_resolve_file() {
        let actual_proxy_file =
            Autoloader::resolve_file(proxy_dir, proxy_namespace, class_name).unwrap();
        assert_eq!(expected_proxy_file, actual_proxy_file);
    }
}

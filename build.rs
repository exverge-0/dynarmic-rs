use std::env::var;
use std::path::PathBuf;

#[cfg(all(
    not(target_env = "msvc"),
    not(target_env = ""),
    not(target_env = "gnu")
))]
compile_error!(
    "Unsupported compiler; dynarmic only supports MSVC and Itanium-ABI (GNU/Clang) compilers."
);

fn main() {
    println!("cargo:rerun-if-changed=src/wrapper.hpp");
    println!("cargo:rerun-if-changed=src/wrapper.cpp");

    // Apply patches
    std::process::Command::new("git")
        .current_dir("dynarmic")
        .args([
            "apply",
            "../patches/0001-Fix-eden-s-refactor-hell.patch",
            "../patches/0002-Don-t-generate-mig-in-source-dir.patch",
            "../patches/0003-Fix-cmake-install-issues.patch",
            "../patches/0004-interface-Boost-should-not-be-in-public-interface.patch"
        ])
        .status().unwrap();

    // Compile dynarmic
    let mut cmake = cmake::Config::new("dynarmic/CMakeLists.txt");
    cmake
        .always_configure(false)
        .define("MASTER_PROJECT", "OFF")
        .define("DYNARMIC_TESTS", "OFF")
        .define("DYNARMIC_USE_BUNDLED_EXTERNALS", "ON")
        .define("DYNARMIC_WARNINGS_AS_ERRORS", "OFF")
        .define(
            "Boost_INCLUDE_DIR",
            PathBuf::from(var("CARGO_MANIFEST_DIR").unwrap())
                .join("ext-boost")
                .to_str()
                .unwrap(),
        )
        .generator("Ninja");

    if var("DEBUG").unwrap_or("".into()).eq("true") {
        cmake.define("CMAKE_BUILD_TYPE", "RelWithDebInfo");
    } else {
        cmake.define("CMAKE_BUILD_TYPE", "Release");
    }

    let dst = cmake.build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=dynarmic");
    println!("cargo:rustc-link-lib=fmt");

    // Compile constants and FFI helper functions
    cc::Build::new()
        .cpp(true)
        .flag(if cfg!(target_env = "msvc") {
            ""
        } else {
            "-Wno-dynamic-class-memaccess"
        })
        .std("c++20")
        .file("src/wrapper.cpp")
        .include(format!("{}/include", dst.display()))
        .compile("wrapper");
}

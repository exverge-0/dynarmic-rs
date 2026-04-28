use std::env::var;
use std::path::PathBuf;

#[cfg(all(not(target_env = "msvc"), not(target_env = ""), not(target_env = "gnu")))]
compile_error!("Unsupported compiler; dynarmic only supports MSVC and Itanium-ABI (GNU/Clang) compilers.");

fn main() {
    println!("cargo:rerun-if-changed=src/wrapper.hpp");
    println!("cargo:rerun-if-changed=src/wrapper.cpp");

    // Compile dynarmic
    let mut cmake = cmake::Config::new("dynarmic/CMakeLists.txt");
    cmake
        .always_configure(false)
        .define("MASTER_PROJECT", "OFF")
        .define("DYNARMIC_USE_BUNDLED_EXTERNALS", "ON")
        .define("DYNARMIC_WARNINGS_AS_ERRORS", "OFF")
        .define(
            "Boost_INCLUDE_DIR",
            PathBuf::from(var("CARGO_MANIFEST_DIR").unwrap())
                .join("dynarmic")
                .join("externals")
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
    println!("cargo:rustc-link-lib=mcl");

    if var("CARGO_CFG_TARGET_ARCH").unwrap() == "x86_64" {
        println!("cargo:rustc-link-lib=Zydis");
    }

    if var("PROFILE").unwrap_or("".into()) == "test" {
        // todo:
        let bindgen = bindgen::Builder::default()
            .header("src/wrapper.hpp")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .allowlist_type("CompilerConstants")
            .clang_arg("-std=c++20")
            .clang_arg(format!("-I{}/include", dst.display()))
            .generate()
            .expect("Unable to generate bindings");

        bindgen
            .write_to_file(PathBuf::from(var("OUT_DIR").unwrap()).join("bindings.rs"))
            .expect("Couldn't write bindings");
    }

    // Compile constants and FFI helper functions
    cc::Build::new()
        .cpp(true)
        .flag(if cfg!(target_env = "msvc") { "" } else { "-Wno-dynamic-class-memaccess" })
        .std("c++20")
        .file("src/wrapper.cpp")
        .include(format!("{}/include", dst.display()))
        .compile("wrapper");
}

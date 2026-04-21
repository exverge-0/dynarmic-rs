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

    cmake.define("CMAKE_BUILD_TYPE", "RelWithDebInfo");

    if var("CARGO_FEATURE_DEVIRTUALIZE").is_ok() {
        cmake.define("DYNARMIC_RS_CUSTOM_EMIT_BEHAVIOR", "ON");
    }
    let dst = cmake.build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=dynarmic");
    println!("cargo:rustc-link-lib=fmt");
    println!("cargo:rustc-link-lib=mcl");

    if var("CARGO_CFG_TARGET_ARCH").unwrap() == "x86_64" {
        println!("cargo:rustc-link-lib=Zydis");
    }

    // todo: what do we really generate here? we can manually bind everything else and drop the libclang dep
    let bindgen = bindgen::Builder::default()
        .header("src/wrapper.hpp")
        .enable_cxx_namespaces()
        .vtable_generation(true)
        .opaque_type("std::_.*")
        .opaque_type("std::.*string.*")
        .opaque_type("std::unique_ptr.*")
        .blocklist_item("std::shared_.*")
        .blocklist_item("std::vector.*")
        .blocklist_item("std::optional.*")
        .blocklist_type("Dynarmic::.*Vector")
        .blocklist_type("Dynarmic::.*::UserConfig")
        .blocklist_type("Dynarmic::.*::UserCallbacks.*")
        .blocklist_type("Dynarmic::OptimizationFlag")
        .module_raw_line(
            "root::std",
            "pub use crate::CppVector as vector;",
        )
        .module_raw_line(
            "root::std",
            "pub use crate::CppOptional as optional;",
        )
        .module_raw_line(
            "root::std",
            "pub use crate::CppSharedPtr as shared_ptr;",
        )
        .module_raw_line("root::Dynarmic::A64", "pub type UserConfig = crate::internal::A64Config<u8>;")
        .module_raw_line("root::Dynarmic::A32", "pub type UserConfig = crate::internal::A32Config<u8>;")
        .module_raw_line("root::Dynarmic::A64", "pub type Vector = u128;")
        .module_raw_line("root::Dynarmic", "pub type Vector = u128;")
        .module_raw_line(
            "root::Dynarmic",
            "pub type OptimizationFlag = crate::OptimizationFlag;",
        )
        .rustified_enum("Dynarmic::HaltReason")
        .rustified_enum("Dynarmic::A32::CoprocReg")
        .rustified_enum("Dynarmic::.*::Exception")
        .rustified_enum("Dynarmic::.*::ArchVersion")
        .rustified_enum("Dynarmic::.*::DataCacheOperation")
        .rustified_enum("Dynarmic::.*::InstructionCacheOperation")
        .allowlist_type("Dynarmic::.*")
        .allowlist_function("Dynarmic::.*")
        .allowlist_var("Dynarmic::.*")
        .clang_arg("-std=c++20")
        .clang_arg(format!("-I{}/include", dst.display()))
        .generate()
        .expect("Unable to generate bindings");

    bindgen
        .write_to_file(PathBuf::from(var("OUT_DIR").unwrap()).join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Compile constants and FFI helper functions
    cc::Build::new()
        .cpp(true)
        .flag(if cfg!(target_env = "msvc") { "" } else { "-Wno-dynamic-class-memaccess" })
        .std("c++20")
        .file("src/wrapper.cpp")
        .include(format!("{}/include", dst.display()))
        .compile("wrapper");
}

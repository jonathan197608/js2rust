// build.rs
// Transpile JS to Zig during Cargo build, then compile Zig to static library.

fn main() {
    // Transpile JS source files in "js_src/" directory
    js2zig_build::transpile("js_src");

    // Run zig build to compile generated Zig code to static library
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let zig_project_dir = std::path::Path::new(&out_dir).join("js2zig").join("main");

    let status = std::process::Command::new("zig")
        .arg("build")
        .current_dir(&zig_project_dir)
        .status();

    match status {
        Ok(s) if s.success() => {
            // Tell cargo to link the generated static library
            println!("cargo:rustc-link-search=native={}", zig_project_dir.join("zig-out/lib").display());
            println!("cargo:rustc-link-lib=static=main");
        }
        Ok(s) => {
            eprintln!("zig build failed: {}", s);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("zig not found: {}", e);
            std::process::exit(1);
        }
    }
}


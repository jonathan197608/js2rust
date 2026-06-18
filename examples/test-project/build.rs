// build.rs
// Transpile JS to Zig during Cargo build, then compile Zig to static library.

fn main() {
    println!("cargo:warning=test-project/build.rs: starting");
    // Transpile JS source files in "js_src/" directory
    // Output will be written to $OUT_DIR/js2zig/
    js2zig_build::transpile("js_src");
    println!("cargo:warning=test-project/build.rs: transpile returned");
    
    // Run zig build to compile generated Zig code to static library
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let zig_project_dir = std::path::Path::new(&out_dir).join("js2zig").join("main");
    println!("cargo:warning=test-project/build.rs: running zig build in {}", zig_project_dir.display());
    
    let status = std::process::Command::new("zig")
        .arg("build")
        .current_dir(&zig_project_dir)
        .status();
    
    match status {
        Ok(s) if s.success() => {
            println!("cargo:warning=test-project/build.rs: zig build succeeded");
            // Tell cargo to link the generated static library
            // Zig project name is "main", so library name is "main"
            println!("cargo:rustc-link-search=native={}", zig_project_dir.join("zig-out/lib").display());
            println!("cargo:rustc-link-lib=static=main");
        }
        Ok(s) => {
            eprintln!("test-project/build.rs: zig build failed: {}", s);
        }
        Err(e) => {
            eprintln!("test-project/build.rs: zig not found: {}", e);
        }
    }
    
    println!("cargo:warning=test-project/build.rs: finished");
}

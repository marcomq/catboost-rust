extern crate bindgen;

use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const CATBOOST_VERSION: &str = "1.2.8"; // The single source of truth for the version

struct FileInfo {
    url: String,
    checksum: &'static str,
}

// Query all checksums for all platforms by running CATBOOST_UPDATE_CHECKSUMS=1 cargo build   

fn c_api_header() -> FileInfo {
    FileInfo {
        url: format!("https://raw.githubusercontent.com/catboost/catboost/v{CATBOOST_VERSION}/catboost/libs/model_interface/c_api.h"),
        checksum: "ecc734868dcd485e2fa7434287ad1fe418a7e3be606ff16ce46a403fb0a7912a", // Verified for v1.2.8
    }
}

fn lib_linux_x86_64() -> FileInfo {
    FileInfo {
        url: format!("https://github.com/catboost/catboost/releases/download/v{CATBOOST_VERSION}/libcatboostmodel-linux-x86_64-{CATBOOST_VERSION}.so"),
        checksum: "5a0de49d0bc81e460fd983da0f0f9d819e804b4ae19c6c9451b31e4f57d06545",
    }
}
fn lib_linux_aarch64() -> FileInfo {
    FileInfo {
        url: format!("https://github.com/catboost/catboost/releases/download/v{CATBOOST_VERSION}/libcatboostmodel-linux-aarch64-{CATBOOST_VERSION}.so"),
        checksum: "0f5f55286b805c30b719bd52fdf661dc94a694207ae1914facbd5018e09349f3",
    }
}
fn lib_darwin_universal() -> FileInfo {
    FileInfo {
        url: format!("https://github.com/catboost/catboost/releases/download/v{CATBOOST_VERSION}/libcatboostmodel-darwin-universal2-{CATBOOST_VERSION}.dylib"),
        checksum: "1b95b7a4523696f6dcf6fd4c009a86014a7cbb42e031912706be2020de111e67",
    }
}
fn lib_windows_dll() -> FileInfo {
    FileInfo {
        url: format!("https://github.com/catboost/catboost/releases/download/v{CATBOOST_VERSION}/catboostmodel-windows-x86_64-{CATBOOST_VERSION}.dll"),
        checksum: "835e1f8b885ca7f7dd1e9ef657b04d33a32ad54dd14d0d81d50e91b2c4d75bcc",
    }
}
fn lib_windows_lib() -> FileInfo {
    FileInfo {
        url: format!("https://github.com/catboost/catboost/releases/download/v{CATBOOST_VERSION}/catboostmodel-windows-x86_64-{CATBOOST_VERSION}.lib"),
        checksum: "d37e0c453980f572a7e05fabd4664bcc5f6cc575259f905bf3e4e1070c4edc7b",
    }
}

fn get_platform_info() -> (String, String) {
    let target = env::var("TARGET").unwrap();

    // Determine OS
    let os = if target.contains("apple-darwin") {
        "darwin"
    } else if target.contains("linux") {
        "linux"
    } else if target.contains("windows") {
        "windows"
    } else {
        panic!("Unsupported target: {}", target);
    };

    // Determine architecture
    let arch = if target.contains("x86_64") {
        "x86_64"
    } else if target.contains("aarch64") || target.contains("arm64") {
        "aarch64"
    } else {
        panic!("Unsupported architecture for target: {}", target);
    };

    (os.to_string(), arch.to_string())
}

fn verify_checksum(data: &[u8], expected_checksum: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    let hex_hash = format!("{:x}", hash);
    hex_hash == expected_checksum
}

fn get_cache_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let target_dir = out_dir
        .ancestors()
        .find(|p| p.ends_with("target"))
        .ok_or("Could not find target directory")?;
    let cache_dir = target_dir.join("catboost-cache");
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)?;
    }
    Ok(cache_dir)
}

fn download_and_verify(
    file_info: &FileInfo,
    destination_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "cargo:warning=Downloading {} to {}",
        &file_info.url,
        destination_path.display()
    );

    let response = ureq::get(&file_info.url).call()?;
    let status = response.status();
    if !(200..300).contains(&status) {
        return Err(format!("Download failed: HTTP {}", status).into());
    }
    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;

    if !verify_checksum(&bytes, file_info.checksum) {
        return Err(format!(
            "Checksum mismatch for {}. Expected: {}, Got: {}",
            file_info.url,
            file_info.checksum,
            format!("{:x}", Sha256::digest(&bytes))
        )
        .into());
    }

    fs::write(destination_path, &bytes)?;
    println!(
        "cargo:warning=Successfully downloaded and verified {}",
        destination_path.display()
    );

    Ok(())
}

fn fetch_file_to_cache(
    file_info: &FileInfo,
    cache_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let filename = Path::new(&file_info.url).file_name().ok_or("Could not get filename from URL")?;
    let cached_path = cache_dir.join(filename);

    if cached_path.exists() {
        let existing_bytes = fs::read(&cached_path)?;
        if verify_checksum(&existing_bytes, file_info.checksum) {
            println!("cargo:info=Using cached file: {}", cached_path.display());
            return Ok(cached_path);
        }
    }

    download_and_verify(file_info, &cached_path)?;
    Ok(cached_path)
}

fn download_compiled_library(out_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let (os, arch) = get_platform_info();
    let lib_dir = out_dir.join("libs");
    fs::create_dir_all(&lib_dir)?;

    let cache_dir = get_cache_dir()?;

    match (os.as_str(), arch.as_str()) {
        ("linux", "x86_64") => {
            let cached_path = fetch_file_to_cache(&lib_linux_x86_64(), &cache_dir)?;
            fs::copy(cached_path, lib_dir.join("libcatboostmodel.so"))?;
        }
        ("linux", "aarch64") => {
            let cached_path = fetch_file_to_cache(&lib_linux_aarch64(), &cache_dir)?;
            fs::copy(cached_path, lib_dir.join("libcatboostmodel.so"))?;
        }
        ("darwin", "x86_64") | ("darwin", "aarch64") => {
            let lib_path = lib_dir.join("libcatboostmodel.dylib");
            let cached_path = fetch_file_to_cache(&lib_darwin_universal(), &cache_dir)?;
            fs::copy(cached_path, &lib_path)?;

            // Modify the copy in OUT_DIR, not the cached original
            let status = std::process::Command::new("install_name_tool")
                .arg("-id")
                .arg("@loader_path/libcatboostmodel.dylib")
                .arg(&lib_path)
                .status()?;
            if !status.success() {
                return Err("install_name_tool failed".into());
            }
        }
        ("windows", "x86_64") => {
            let cached_dll = fetch_file_to_cache(&lib_windows_dll(), &cache_dir)?;
            fs::copy(cached_dll, lib_dir.join("catboostmodel.dll"))?;

            let cached_lib = fetch_file_to_cache(&lib_windows_lib(), &cache_dir)?;
            fs::copy(cached_lib, lib_dir.join("catboostmodel.lib"))?;
        }
        _ => return Err(format!("Unsupported platform: {}-{}", os, arch).into()),
    }
    Ok(())
}

fn run_checksum_updater() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:warning=-----------------------------------------------------------------");
    println!("cargo:warning=CATBOOST_UPDATE_CHECKSUMS mode enabled.");
    println!("cargo:warning=Downloading files for ALL platforms and printing checksums.");
    println!("cargo:warning=-----------------------------------------------------------------");

    let files_to_check = vec![
        c_api_header(),
        lib_linux_x86_64(),
        lib_linux_aarch64(),
        lib_darwin_universal(),
        lib_windows_dll(),
        lib_windows_lib(),
    ];

    for file_info in &files_to_check {
        println!("cargo:warning=Downloading {}...", &file_info.url);
        let response = ureq::get(&file_info.url).call()?;
        let mut bytes = Vec::new();
        response.into_reader().read_to_end(&mut bytes)?;

        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash = hasher.finalize();
        let hex_hash = format!("{:x}", hash);

        println!("cargo:warning=URL:      {}", file_info.url);
        println!("cargo:warning=Checksum: {}", hex_hash);
        println!("cargo:warning=-----------------------------------------------------------------");
    }

    Ok(())
}

fn main() {
    // Check if the user wants to update checksums instead of building.
    if env::var("CATBOOST_UPDATE_CHECKSUMS").is_ok() {
        if let Err(e) = run_checksum_updater() {
            panic!("Failed to run checksum updater: {}", e);
        }
        // We panic here to stop the build process cleanly after printing the checksums.
        // This is the intended behavior for this utility mode.
        panic!("Checksum update process finished. Please update the checksums in build.rs and re-run the build without CATBOOST_UPDATE_CHECKSUMS set.");
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cb_model_interface_root = out_dir.join("libs/model_interface");

    // Declare custom cfg flags for Cargo's check-cfg feature
    println!("cargo::rustc-check-cfg=cfg(catboost_embeddings)");
    println!("cargo::rustc-check-cfg=cfg(catboost_text_count)");
    println!("cargo::rustc-check-cfg=cfg(catboost_staged_prediction)");
    println!("cargo::rustc-check-cfg=cfg(catboost_feature_indices)");
    println!("cargo:rustc-cfg=catboost_embeddings");
    println!("cargo:rustc-cfg=catboost_text_count");
    println!("cargo:rustc-cfg=catboost_staged_prediction");
    println!("cargo:rustc-cfg=catboost_feature_indices");
    println!("cargo:rustc-cfg=catboost_embeddings");
    println!("cargo:rustc-cfg=catboost_text_count");

    // Download the model interface headers
    let cache_dir = get_cache_dir().expect("Failed to get cache directory");
    let cached_header_path: PathBuf =
        fetch_file_to_cache(&c_api_header(), &cache_dir).expect("Failed to fetch c_api.h");

    let model_interface_dir = out_dir.join("libs/model_interface");
    fs::create_dir_all(&model_interface_dir).expect("Failed to create model_interface dir");
    fs::copy(cached_header_path, model_interface_dir.join("c_api.h"))
        .expect("Failed to copy c_api.h");

    // Download the compiled library
    if let Err(e) = download_compiled_library(&out_dir) {
        eprintln!("Failed to download compiled library: {}", e);
        panic!("Cannot proceed without compiled library");
    }

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", cb_model_interface_root.display()))
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate bindings.");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings.");

    // 1. Get platform info using your existing function
    let (os, _arch) = get_platform_info();

    // 2. Determine the library filename based on the OS
    let lib_filename = match os.as_str() {
        "windows" => "catboostmodel.dll",
        "darwin" => "libcatboostmodel.dylib", // "darwin" comes from your function
        _ => "libcatboostmodel.so",           // Default to Linux/Unix
    };

    // 3. Copy the library from OUT_DIR/libs to the final target directory
    let lib_source_path = out_dir.join("libs").join(lib_filename);

    // Find the final output directory (e.g., target/release)
    let target_dir = out_dir
        .ancestors()
        .find(|p| p.ends_with("target"))
        .unwrap()
        .join(env::var("PROFILE").unwrap());

    let lib_dest_path = target_dir.join(lib_filename);
    fs::copy(&lib_source_path, &lib_dest_path).expect("Failed to copy library to target directory");

    // On macOS/Linux, change the install name/soname to use @loader_path/$ORIGIN
    // This needs to be done on the source library in OUT_DIR before linking
    if os == "linux" {
        use std::process::Command;
        // Use patchelf to set soname to just the library filename on Linux (if available)
        // This is optional - if patchelf is not installed, we just skip it
        let _ = Command::new("patchelf")
            .arg("--set-soname")
            .arg(lib_filename)
            .arg(&lib_source_path)
            .output(); // Use output() to silently ignore if patchelf doesn't exist
        let _ = Command::new("patchelf")
            .arg("--set-soname")
            .arg(lib_filename)
            .arg(&lib_dest_path)
            .output();
    }

    // 4. Set the library search path for the build-time linker
    let lib_search_path = out_dir.join("libs");
    println!(
        "cargo:rustc-link-search=native={}",
        lib_search_path.display()
    );

    // 5. Set the rpath for the run-time linker based on the OS
    match os.as_str() {
        "darwin" => {
            // For macOS, add multiple rpath entries for IDE compatibility
            println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
            println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../..");
            println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
            println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/../..");
            println!(
                "cargo:rustc-link-arg=-Wl,-rpath,{}",
                lib_search_path.display()
            );
            // Add the target directory to rpath as well
            if let Some(target_root) = out_dir.ancestors().find(|p| p.ends_with("target")) {
                println!(
                    "cargo:rustc-link-arg=-Wl,-rpath,{}/debug",
                    target_root.display()
                );
                println!(
                    "cargo:rustc-link-arg=-Wl,-rpath,{}/release",
                    target_root.display()
                );
            }
        }
        "linux" => {
            // For Linux, use $ORIGIN
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../..");
            println!(
                "cargo:rustc-link-arg=-Wl,-rpath,{}",
                lib_search_path.display()
            );
            // Add the target directory to rpath as well
            if let Some(target_root) = out_dir.ancestors().find(|p| p.ends_with("target")) {
                println!(
                    "cargo:rustc-link-arg=-Wl,-rpath,{}/debug",
                    target_root.display()
                );
                println!(
                    "cargo:rustc-link-arg=-Wl,-rpath,{}/release",
                    target_root.display()
                );
            }
        }
        _ => {} // No rpath needed for Windows
    }

    println!("cargo:rustc-link-lib=dylib=catboostmodel");
}

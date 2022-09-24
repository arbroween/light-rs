use std::{
    env::{self, current_dir},
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

fn get_output_path() -> PathBuf {
    let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_type = env::var("PROFILE").unwrap();
    let path = Path::new(&manifest_dir_string)
        .join("target")
        .join(build_type);
    path
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let type_ = entry.file_type()?;
        if type_.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn main() {
    let src_data = current_dir()
        .expect("Could not get current directory")
        .join("data")
        .canonicalize()
        .expect("Could not canonicalize path");

    let out_data = get_output_path().join("data");
    match fs::create_dir(&out_data) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
        Err(error) => panic!("Could not create ./target/<build_type>/data: {}", error),
    };

    eprintln!("src_data = {:?}", &src_data);
    copy_dir_all(src_data, out_data).expect("Could not copy files");

    println!("cargo:rerun-if-changed=build.rs");
}

use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::PathBuf,
};
use winresource::WindowsResource;

fn main() -> io::Result<()> {
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        WindowsResource::new()
            .set_icon(if cfg!(feature = "uninstaller") {
                "assets/uninstaller.ico"
            } else {
                "assets/icon.ico"
            })
            .compile()
            .unwrap();
    }
    let mut f = BufWriter::new(File::create("data.bin").unwrap());
    let mut d = HashMap::<String, Vec<u8>>::new();
    let mut w = vec![fs::read_dir("assets").unwrap()];
    while let Some(l) = w.pop() {
        for e in l.flatten() {
            if e.file_type().unwrap().is_dir() {
                w.push(fs::read_dir(e.path()).unwrap());
            } else {
                d.insert(
                    e.path()
                        .components()
                        .skip(1)
                        .collect::<PathBuf>()
                        .to_str()
                        .unwrap()
                        .to_string(),
                    fs::read(e.path()).unwrap(),
                );
            }
        }
    }
    f.write_all(&postcard::to_allocvec(&d).unwrap()).unwrap();
    f.flush().unwrap();
    Ok(())
}

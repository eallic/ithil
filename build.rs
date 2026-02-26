use fatfs::FileSystem;
use fatfs::FormatVolumeOptions;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let img_path = out_dir.join("ithil.img");

    let bootloader_path =
        PathBuf::from(env::var_os("CARGO_BIN_FILE_BOOTLOADER_bootloader").unwrap());
    let kernel_path = PathBuf::from(env::var_os("CARGO_BIN_FILE_KERNEL_kernel").unwrap());

    create_image(&img_path, &bootloader_path, &kernel_path);

    let code_path = Path::new("/usr/share/edk2-ovmf/x64/OVMF_CODE.4m.fd");
    let vars_path = Path::new("/usr/share/edk2-ovmf/x64/OVMF_VARS.4m.fd");

    let vars_path = copy_if_missing(vars_path, &out_dir);

    println!("cargo:rustc-env=IMG_PATH={}", img_path.display());
    println!("cargo:rustc-env=CODE_PATH={}", code_path.display());
    println!("cargo:rustc-env=VARS_PATH={}", vars_path.display());
}

fn copy_if_missing(src: &Path, dst: &Path) -> PathBuf {
    let dst = dst.join(src.file_name().unwrap());

    if !dst.exists() {
        fs::copy(src, &dst).unwrap();
    }

    dst
}

fn create_image(img_path: &Path, bootloader_path: &Path, kernel_path: &Path) {
    const MB: u64 = 1024 * 1024;

    let bootloader_size = fs::metadata(bootloader_path).unwrap().len();
    let kernel_size = fs::metadata(kernel_path).unwrap().len();

    let needed_bytes = bootloader_size + kernel_size;
    let needed_mb = (needed_bytes + MB - 1) / MB + 4;
    let needed_bytes = needed_mb * MB;

    let fat_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(img_path)
        .unwrap();

    fat_file.set_len(needed_bytes).unwrap();

    fatfs::format_volume(&fat_file, FormatVolumeOptions::new()).unwrap();

    let fs = FileSystem::new(&fat_file, fatfs::FsOptions::new()).unwrap();
    let root = fs.root_dir();

    root.create_dir("EFI").unwrap();
    root.create_dir("EFI/BOOT").unwrap();

    let bootloader_bytes = fs::read(bootloader_path).unwrap();
    let kernel_bytes = fs::read(kernel_path).unwrap();

    root.create_file("EFI/BOOT/BOOTX64.EFI")
        .unwrap()
        .write_all(&bootloader_bytes)
        .unwrap();

    root.create_file("KERNEL.ELF")
        .unwrap()
        .write_all(&kernel_bytes)
        .unwrap();
}

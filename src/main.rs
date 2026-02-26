use std::env;
use std::process::Command;

fn main() {
    let img_path = env!("IMG_PATH");
    let code_path = env!("CODE_PATH");
    let vars_path = env!("VARS_PATH");

    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.arg("-debugcon").arg("stdio");
    cmd.arg("-display").arg("none");
    cmd.arg("-enable-kvm");
    cmd.arg("-machine").arg("q35");
    cmd.arg("-drive").arg(format!("format=raw,file={img_path}"));

    cmd.arg("-drive").arg(format!(
        "if=pflash,format=raw,unit=0,file={},readonly=on",
        code_path
    ));

    cmd.arg("-drive").arg(format!(
        "if=pflash,format=raw,unit=1,file={},snapshot=on",
        vars_path
    ));

    let mut child = cmd.spawn().unwrap();
    child.wait().unwrap();
}

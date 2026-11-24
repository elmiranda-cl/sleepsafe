use std::process::Command;

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows") {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let out_file = format!("{}/sleepsafe_res.o", out_dir);

        let windres_candidates = ["x86_64-w64-mingw32-windres", "windres"];
        let mut compiled = false;

        for cmd in windres_candidates {
            if let Ok(status) = Command::new(cmd)
                .args(["sleepsafe.rc", "-O", "coff", "-o", &out_file])
                .status()
            {
                if status.success() {
                    compiled = true;
                    println!("cargo:rustc-link-arg={}", out_file);
                    break;
                }
            }
        }

        if !compiled {
            let res_file = format!("{}/sleepsafe.res", out_dir);
            if let Ok(status) = Command::new("rc.exe")
                .args(["/fo", &res_file, "sleepsafe.rc"])
                .status()
            {
                if status.success() {
                    compiled = true;
                    println!("cargo:rustc-link-arg={}", res_file);
                }
            }
        }

        if !compiled {
            println!("cargo:warning=Resource compiler (windres/rc.exe) not found or failed; Windows resources not embedded");
        }

        println!("cargo:rerun-if-changed=sleepsafe.rc");
        println!("cargo:rerun-if-changed=assets/sleepsafe.ico");
    }
}

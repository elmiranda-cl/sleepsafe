#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    use std::fs;
    use std::path::PathBuf;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ,
    };
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDYES, MB_ICONINFORMATION, MB_ICONQUESTION, MB_SETFOREGROUND, MB_TOPMOST,
        MB_YESNO, SW_SHOW,
    };

    let payload: &[u8] = include_bytes!("../../target/x86_64-pc-windows-gnu/release/sleepsafe.exe");

    let is_silent = std::env::args().any(|a| a == "--silent" || a == "/S" || a == "-s");

    if !is_silent {
        let prompt_text = to_wide(
            "=========================================\r\n\
             🛡️ Instalador Oficial de SleepSafe v0.1.0\r\n\
             =========================================\r\n\r\n\
             ¿Deseas instalar SleepSafe en tu computadora?\r\n\r\n\
             • Se instalará en tu carpeta de usuario (sin requerir permisos de administrador).\r\n\
             • Se configurará para iniciar automáticamente con Windows.\r\n\
             • Protegerá tu notebook contra sobrecalentamiento con tapa cerrada.\r\n\r\n\
             Haz clic en 'Sí' para instalar y activar la protección ahora.",
        );
        let prompt_title = to_wide("Instalar SleepSafe");

        let res = unsafe {
            MessageBoxW(
                HWND::default(),
                PCWSTR(prompt_text.as_ptr()),
                PCWSTR(prompt_title.as_ptr()),
                MB_YESNO | MB_ICONQUESTION | MB_SETFOREGROUND | MB_TOPMOST,
            )
        };

        if res != IDYES {
            return;
        }
    }

    // Determine target installation directory: %LOCALAPPDATA%\Programs\SleepSafe
    let target_dir = if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local_appdata).join("Programs").join("SleepSafe")
    } else if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("SleepSafe")
    } else {
        PathBuf::from("C:\\SleepSafe")
    };

    if let Err(e) = fs::create_dir_all(&target_dir) {
        if !is_silent {
            let err_text = to_wide(&format!("Error al crear la carpeta de instalación: {}", e));
            let err_title = to_wide("Error de Instalación");
            unsafe {
                let _ = MessageBoxW(
                    HWND::default(),
                    PCWSTR(err_text.as_ptr()),
                    PCWSTR(err_title.as_ptr()),
                    windows::Win32::UI::WindowsAndMessaging::MB_OK
                        | windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
                );
            }
        }
        return;
    }

    let target_exe = target_dir.join("sleepsafe.exe");

    // If an existing instance is running, try to kill or write directly
    if let Err(_e) = fs::write(&target_exe, payload) {
        // If file is locked, try terminating any running SleepSafe instance first
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "sleepsafe.exe"])
            .output();
        std::thread::sleep(std::time::Duration::from_millis(500));

        if let Err(e) = fs::write(&target_exe, payload) {
            if !is_silent {
                let err_text = to_wide(&format!(
                    "No se pudo copiar el archivo ejecutable. Asegúrate de que SleepSafe no esté bloqueado.\nDetalle: {}",
                    e
                ));
                let err_title = to_wide("Error al copiar archivos");
                unsafe {
                    let _ = MessageBoxW(
                        HWND::default(),
                        PCWSTR(err_text.as_ptr()),
                        PCWSTR(err_title.as_ptr()),
                        windows::Win32::UI::WindowsAndMessaging::MB_OK
                            | windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
                    );
                }
            }
            return;
        }
    }

    // Register Auto-Start in Windows Registry (HKCU\Software\Microsoft\Windows\CurrentVersion\Run)
    unsafe {
        let subkey = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_SET_VALUE, &mut hkey).is_ok() {
            let value_name = w!("SleepSafe");
            let formatted = format!("\"{}\"", target_exe.to_string_lossy());
            let wide_val: Vec<u16> = formatted
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let bytes = std::slice::from_raw_parts(
                wide_val.as_ptr() as *const u8,
                wide_val.len() * 2,
            );
            let _ = RegSetValueExW(hkey, value_name, 0, REG_SZ, Some(bytes));
            let _ = RegCloseKey(hkey);
        }
    }

    // Launch the newly installed SleepSafe
    let target_w = to_wide(&target_exe.to_string_lossy());
    unsafe {
        let _ = ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(target_w.as_ptr()),
            None,
            None,
            SW_SHOW,
        );
    }

    if !is_silent {
        let success_text = to_wide(
            "=========================================\r\n\
             🎉 ¡Instalación Completada con Éxito!\r\n\
             =========================================\r\n\r\n\
             SleepSafe ha sido instalado e iniciado correctamente.\r\n\r\n\
             • Ubicación: %LOCALAPPDATA%\\Programs\\SleepSafe\r\n\
             • Inicio automático: Activado con Windows.\r\n\
             • Estado: Activo y protegiendo en la barra de tareas.\r\n\r\n\
             Puedes gestionar tu configuración y ver lecturas haciendo clic en el icono junto al reloj.",
        );
        let success_title = to_wide("SleepSafe - Instalado");
        unsafe {
            let _ = MessageBoxW(
                HWND::default(),
                PCWSTR(success_text.as_ptr()),
                PCWSTR(success_title.as_ptr()),
                MB_ICONINFORMATION | MB_SETFOREGROUND | MB_TOPMOST,
            );
        }
    }
}

#[cfg(not(windows))]
fn main() {
    println!("SleepSafe installer is designed for Windows.");
}

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

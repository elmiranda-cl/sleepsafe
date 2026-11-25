<p align="center">
  <img src="assets/sleepsafe_256.png" width="160" alt="Logo de SleepSafe" />
</p>

# SleepSafe

> Protección térmica inteligente contra el drenaje de batería y sobrecalentamiento con la tapa cerrada en laptops con Windows.

[Read in English](README.md)

---

## ¿Qué es SleepSafe?

En las laptops con Windows, cuando configuras la opción **"Al cerrar la tapa: No hacer nada"** mientras estás conectado a la corriente (CA), el sistema operativo no vuelve a comprobar el estado de la tapa si el cargador se desconecta más tarde o si se corta la luz.

Si desconectas la laptop mientras la tapa ya está cerrada y la guardas en una mochila o bolso, el equipo permanece encendido, las salidas de aire quedan obstruidas y la temperatura aumenta rápidamente. Esto causa un rápido consumo de batería, ralentización térmica severa o daños permanentes en los componentes internos (problema conocido popularmente como el **"Síndrome del Horno en la Mochila"** o *Backpack Oven Syndrome*).

**SleepSafe** soluciona esto ejecutándose silenciosamente en segundo plano como un guardián ligero:

1. **Detección de desconexión:** Cuando el cargador se desconecta y la tapa ya está cerrada, SleepSafe inicia un monitoreo térmico activo.
2. **Diferenciación entre Mochila y Escritorio:**
   - **En un escritorio durante un corte de luz:** La ventilación ambiental mantiene el equipo fresco y estable. SleepSafe otorga una ventana de cortesía de 10 minutos por si vuelve la electricidad.
   - **Dentro de una mochila:** La tela bloquea el flujo de aire y la temperatura sube de forma continua. SleepSafe detecta esta pendiente térmica ascendente (o un límite crítico de 58°C) y **suspende la máquina de inmediato** para proteger tu hardware.
3. **Reconocimiento de monitores externos:** Si hay un monitor secundario conectado (HDMI/DisplayPort), SleepSafe reconoce que estás trabajando en modo *docking* en un escritorio y desactiva las alarmas de suspensión.

---

## Características

- **Bandeja del sistema (System Tray) nativa:** Se integra junto al reloj con icono de escudo, diagnósticos en tiempo real y menú contextual (ver logs en vivo, abrir carpeta de configuración, pausar/reanudar, activar inicio automático y salir).
- **Inicio automático con Windows:** Configurable con un solo clic desde el menú del icono en la bandeja o en el archivo de configuración, sincronizándose automáticamente con el registro de Windows (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`).
- **Ultraligero:** Desarrollado 100% en Rust con APIs nativas de Win32 (sin runtimes pesados ni Node/Electron, menos de 8 MB de memoria RAM y 0% de uso de CPU en reposo).
- **Parámetros configurables:** Archivo de configuración en formato JSON (`%APPDATA%\SleepSafe\config.json`) para ajustar límites térmicos, tiempos de gracia e intervalos de sondeo.
- **Metadatos e iconos oficiales de Windows:** Incluye recursos de icono multirresolución (16x16 hasta 256x256) y propiedades VersionInfo integradas en el binario.

---

## Instalación

### Opción 1: Instalador estándar (Recomendado)
Descarga `sleepsafe-setup.exe` desde la sección de [Releases de GitHub](https://github.com/elmiranda-cl/sleepsafe/releases/latest). Ejecútalo para instalar SleepSafe en tu carpeta de usuario y habilitar el inicio automático.

### Opción 2: Ejecutable portable
Descarga `sleepsafe.exe` directamente, guárdalo donde prefieras y ejecútalo. Puedes activar la opción "Iniciar automáticamente con Windows" directamente desde el menú del icono en la barra de tareas.

### Opción 3: Winget (Administrador de paquetes de Windows)
```cmd
winget install elmiranda-cl.SleepSafe
```

---

## Desarrollo y Compilación

### Requisitos previos
- [Rust](https://www.rust-lang.org/) (cadena de herramientas estable)
- Windows 10 u 11

### Ejecutar pruebas
```bash
cargo test
```

### Compilar binario de distribución (Release)
```bash
cargo build --release
```

El ejecutable compilado se encontrará en `target/release/sleepsafe.exe`.

---

## Licencia
Licencia MIT - Desarrollado por elmiranda-cl.

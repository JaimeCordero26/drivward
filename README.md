# Drivward

Tus drivers, en orden. Desde el formateo hasta el día a día.

Drivward es un gestor de drivers para Windows 10 y 11 x64, gratis y de código abierto (GPL-3.0). Tiene dos versiones sobre un mismo núcleo en Rust:

- **Portable**: corre desde una USB, sin instalación ni internet. Instala el driver de red para recuperar la conexión de una compu recién formateada y después revisa qué falta.
- **Escritorio**: un servicio de Windows con una app en la bandeja del sistema. Registra los cambios de drivers, vigila la estabilidad y permite volver a la versión anterior.

Sitio: <https://drivward.sacortech.xyz>

> **Estado:** en desarrollo. No hay versión estable todavía. Prueba primero en máquinas virtuales o equipos propios.

## Principios

- Nada se instala sin tu aprobación.
- Antes de cambiar algo se respaldan los drivers actuales y se intenta crear un punto de restauración.
- Solo drivers firmados y de fuentes oficiales o aprobadas. Los archivos de la USB se verifican con SHA-256 antes de instalar.
- Se elige el driver más estable, no solo el más nuevo: coincidencia exacta de Hardware ID, estabilidad, fabricante del equipo, firma, origen, versión y fecha, en ese orden. Cada recomendación explica por qué.
- La app no afirma que un driver causó un fallo por una coincidencia de tiempo: muestra la evidencia y el nivel de confianza.
- No envía datos del equipo. Ver [PRIVACY.md](PRIVACY.md).

## Estructura

| Ruta | Contenido |
| --- | --- |
| `crates/drivward-core` | Núcleo sin dependencias del sistema: analizador INF, índice con SHA-256, selección de drivers, clasificación, catálogo firmado (Ed25519), registro de acciones y reportes HTML/JSON |
| `crates/drivward-platform` | Acceso a Windows: inventario (CIM/WMI), `pnputil`, Windows Update Agent, registro de eventos, puntos de restauración, prueba de conectividad |
| `crates/drivward-portable` | Versión portable: ventana nativa Win32 (sin WebView2 ni .NET) y modo `--consola` |
| `crates/drivward-service` | Servicio de escritorio: SQLite, detección de cambios, línea base de estabilidad, políticas de actualización, rollback y tuberías con nombre protegidas |
| `crates/drivward-ipc` | Protocolo entre el servicio y la app de bandeja |
| `crates/drivward-index` | Herramienta para técnicos y mantenedores: genera y verifica `index.json`, prepara la USB y firma catálogos |
| `apps/desktop` | App de bandeja con Tauri, React y TypeScript |
| `web` | Sitio del proyecto (React + Vite, publicado en Cloudflare) |
| `docs` | [Requerimientos](docs/requerimientos.md), [arquitectura](docs/arquitectura.md) y [trazabilidad](docs/trazabilidad.md) |

## Compilar

Requiere Rust estable. En Windows además Visual Studio Build Tools; para la app de escritorio, Node.js 20 o superior.

```powershell
cargo test --workspace
cargo build --release
```

Resultado en `target\release`:

- `drivward-portable.exe`: la portable. Pide permisos de administrador al abrirse.
- `drivward-service.exe`: el servicio (`install`, `uninstall`, `run`, `console`).
- `drivward-index.exe`: herramienta de la librería.

App de escritorio e instalador:

```powershell
cargo build --release -p drivward-service
cd apps\desktop
npm ci
npm run tauri build
```

El instalador NSIS registra el servicio con inicio automático y abre la bandeja al iniciar sesión.

## Armar la USB

Los drivers no forman parte del repositorio. El preparador los descarga de la fuente oficial en tu compu:

1. Copia `library\fuentes.ejemplo.json` a `library\fuentes.json` y completa cada paquete con su URL HTTPS oficial, SHA-256 y licencia.
2. Ejecuta:

   ```powershell
   .\scripts\preparar-usb.ps1 -Usb E: -Manifiesto .\library\fuentes.json
   ```

La USB queda así:

```
E:\Drivward\
  drivward-portable.exe
  drivers\            paquetes extraídos e index.json
  fuentes.json        origen, URL, hash y licencia de cada paquete
  logs\               registro de acciones (se crea al usar)
  respaldos\          drivers exportados antes de cada cambio
  reportes\           reportes HTML y JSON para el cliente
```

`drivward-index build <carpeta>` y `drivward-index verify <carpeta>` sirven para librerías armadas a mano.

## Uso de la portable

1. Abre `drivward-portable.exe` en la compu formateada.
2. Si no hay internet, Drivward busca el adaptador de red sin driver y propone el mejor de la librería. Al aprobarlo, lo instala y prueba la conexión real.
3. Si no hay driver compatible, el botón **Internet desde el celular** explica cómo compartir la conexión por USB.
4. Con internet, **Escanear todo** revisa el resto de dispositivos con la librería y Windows Update. Selecciona los que apruebas y pulsa **Instalar seleccionados**.
5. **Exportar reporte** guarda el resultado para el cliente.

Si un driver pide reiniciar, Drivward guarda lo que falta y se abre solo al volver.

## Uso de la versión de escritorio

- **Observar** (predeterminado): solo registra cambios y fallos.
- **Recomendar**: propone actualizaciones de Windows Update después de 7 días de espera y pide aprobación.
- **Automático** (experimental): instala solo drivers de bajo riesgo. Gráfica, chipset, almacenamiento y red siempre piden aprobación.

La app de bandeja puede consultar el estado como usuario normal. Para instalar, volver atrás o cambiar ajustes hay que abrirla como administrador.

## Licencia

GPL-3.0-only. Los drivers no forman parte de este repositorio y conservan la licencia de cada fabricante. Windows es una marca de Microsoft; Drivward no está afiliado a Microsoft ni a fabricantes de hardware.

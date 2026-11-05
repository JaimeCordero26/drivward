# Arquitectura

Las dos versiones comparten un núcleo en Rust; solo cambian la forma de ejecutarse y la interfaz.

```
                 ┌──────────────────────┐
                 │   drivward-core      │  INF, índice, selección, catálogo,
                 │   (sin sistema)      │  registro, reportes
                 └─────────┬────────────┘
                           │
                 ┌─────────▼────────────┐
                 │  drivward-platform   │  Windows: CIM/WMI, pnputil,
                 │  (trait Platform)    │  Windows Update, eventos
                 └───┬─────────────┬────┘
                     │             │
        ┌────────────▼───┐   ┌─────▼──────────────┐       ┌────────────────────┐
        │ drivward-      │   │ drivward-service   │◄─────►│ apps/desktop       │
        │ portable       │   │ servicio Windows   │ pipes │ bandeja Tauri      │
        │ ventana Win32  │   │ SQLite             │ (ipc) │ React + TypeScript │
        └────────────────┘   └────────────────────┘       └────────────────────┘
```

La portable y el servicio llaman al mismo núcleo; la app de bandeja solo habla con el servicio.

## Decisiones

**Núcleo sin sistema operativo (RNF-10).** `drivward-core` no usa APIs de Windows. Todo lo que toca el sistema pasa por el trait `Platform`, que tiene una implementación para Windows y otra que responde "no soportado". Así el núcleo y los flujos se prueban en cualquier sistema con plataformas falsas, y Linux puede sumarse después con su propia implementación (RNF-11).

**Inventario con CIM/WMI desde PowerShell.** `Win32_PnPEntity` y `Win32_PnPSignedDriver` dan Hardware IDs, Compatible IDs, código de problema, proveedor, versión, fecha, INF y firma. PowerShell 5.1 viene con Windows 10 y 11, así que la portable no requiere instalar nada (RC-1). Los scripts se envían codificados con `-EncodedCommand` y devuelven JSON que se valida con serde. Si en pruebas reales resulta lento, la misma interfaz puede implementarse con SetupAPI/CfgMgr32 sin tocar el resto.

**Instalación con `pnputil`.** `/add-driver <inf> /install` agrega el paquete al Driver Store e instala en los dispositivos que coincidan. El código 3010 indica reinicio y 259 que el driver no aplica. Antes de instalar se exportan los drivers de terceros con `/export-driver *` y se intenta crear un punto de restauración.

**Selección de drivers.** Orden de preferencia: coincidencia de Hardware ID sobre Compatible ID (y la más específica primero), estabilidad según el catálogo, driver del fabricante del equipo, firma comprobada, origen, versión y fecha. Se descartan los candidatos de otra arquitectura, con build mínima mayor, sin firma válida, de origen no aprobado, bloqueados o que serían un downgrade no autorizado. Cada recomendación guarda sus razones.

**Librería verificable.** `index.json` registra el SHA-256 de cada archivo. La portable compara la librería completa al abrir y vuelve a comprobar la carpeta del paquete justo antes de instalarlo. Los paquetes alterados no se ofrecen.

**Catálogo firmado.** El catálogo remoto es JSON firmado con Ed25519. El cliente verifica la firma antes de leer el contenido, exige HTTPS en cada URL y elige localmente, sin enviar su inventario (API-05). La clave privada vive fuera de los recolectores (API-07); `drivward-index sign` firma el archivo.

**Servicio y estabilidad.** Cada 15 minutos el servicio compara el inventario con el anterior, registra los cambios y lee del registro de eventos los reinicios inesperados (41, 6008), pantallazos azules (1001), fallos de video (4101) y drivers que no cargan (219). Para cada cambio compara la frecuencia de fallos en los 30 días previos con la posterior. La confianza sube si hay varios fallos, si la frecuencia se multiplica y si el tipo de fallo encaja con el dispositivo (por ejemplo, 4101 con la gráfica). Un cambio sin fallos durante el periodo de observación se marca como validado y su driver se respalda.

**Rollback.** Windows suele conservar la versión anterior en el Driver Store. Para volver atrás, el servicio reinstala el respaldo si existe, quita el paquete nuevo con `pnputil /delete-driver /uninstall` y comprueba que el dispositivo quedó con la versión esperada. Para que Windows Update no lo reinstale, la versión puede ocultarse (DESK-23) desde la app, con permiso del usuario.

**Tuberías protegidas (SEG-07).** `\\.\pipe\drivward-estado` acepta a usuarios de la sesión, pero solo responde consultas. `\\.\pipe\drivward-control` solo acepta a SYSTEM y administradores con token elevado. Ambas se crean con `FILE_FLAG_FIRST_PIPE_INSTANCE` y rechazan clientes remotos.

**Ventana nativa de la portable.** Usa controles Win32 (native-windows-gui), que funcionan con el adaptador de video básico de una compu recién formateada y no dependen de WebView2 ni .NET.

## Datos

| Versión | Ubicación | Contenido |
| --- | --- | --- |
| Portable | `<USB>\Drivward\logs\acciones.jsonl` | Registro de acciones (rotación a 5 MB, 5 archivos) |
| Portable | `<USB>\Drivward\respaldos\` | Drivers exportados antes de cada sesión de cambios |
| Portable | `<USB>\Drivward\reportes\` | Reportes HTML y JSON |
| Escritorio | `%ProgramData%\Drivward\drivward.db` | Inventario, historial, eventos, versiones buenas, bloqueos y ajustes |
| Escritorio | `%ProgramData%\Drivward\respaldos\` | Copia de las versiones que funcionaron |
| Escritorio | `%ProgramData%\Drivward\logs\` | Registro de acciones |

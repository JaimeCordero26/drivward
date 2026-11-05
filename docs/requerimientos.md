# Drivward: requerimientos

Nov 4, 2026 · @Sacor Tech

## Visión y alcance

El producto deja una compu con internet después de formatear y luego mantiene sus drivers estables. Tiene dos versiones: una **portable en USB** y una **de escritorio** que funciona como cuidador.

**Objetivos**

- **OBJ-1:** devolver la conexión a una compu recién formateada sin depender de internet.
- **OBJ-2:** detectar e instalar los drivers que faltan sin ir driver por driver.
- **OBJ-3:** elegir el driver más estable para cada equipo, no solo el más nuevo.
- **OBJ-4:** vigilar la estabilidad y revertir un driver cuando cause conflictos.
- **OBJ-5:** dar un mantenimiento transparente, con historial y control del usuario.

**Dentro del alcance**

- Windows 10 y Windows 11 x64.
- Wi-Fi, Ethernet, almacenamiento, gráfica, chipset, audio y otros dispositivos PnP.
- Escaneo, instalación, respaldo, rollback, monitoreo y reportes.

**Fuera del alcance en la primera versión**

- Linux, que entra en una fase posterior con alcance propio.
- Windows ARM64.
- Actualización de BIOS o firmware.
- Limpieza y optimización general del sistema.

## Usuarios, entorno y restricciones

El usuario principal es un técnico que formatea y mantiene compus de clientes. La versión de escritorio también debe ser segura para una persona sin conocimientos técnicos.

| Usuario | Necesidad | Versión |
| --- | --- | --- |
| Técnico | Dejar equipos listos rápido, usar kits por modelo y entregar reportes | Portable y escritorio |
| Dueño del equipo | Mantener la compu estable sin decidir detalles técnicos | Escritorio |
| Mantenedor del producto | Actualizar la librería y publicar catálogos seguros | Herramientas internas |

**Supuestos**

- El programa corre con permisos de administrador.
- La USB tiene un sistema de archivos legible por Windows y espacio para la librería.
- La compu puede iniciar Windows, salvo en el flujo de rescate.

**Restricciones**

- **RC-1:** la portable no puede requerir internet, instalación previa, .NET ni WebView2 para su flujo inicial.
- **RC-2:** la primera versión solo soporta x64.
- **RC-3:** todo driver debe estar firmado y ser aceptado por las políticas de Windows.
- **RC-4:** los drivers solo pueden provenir de fuentes oficiales o aprobadas.
- **RC-5:** antes de publicar una USB ya armada se debe revisar la licencia de cada paquete incluido.

## Núcleo compartido

El núcleo contiene la lógica común de detección, selección, instalación y registro. La prioridad indica si la función es indispensable para su fase.

| ID | Requerimiento | Prioridad | Fase |
| --- | --- | --- | --- |
| CORE-01 | Identificar versión, build, edición y arquitectura de Windows | Alta | 1 |
| CORE-02 | Identificar fabricante, modelo, placa, BIOS, CPU, GPU y número de producto | Alta | 1 |
| CORE-03 | Listar dispositivos PnP con Hardware IDs, Compatible IDs, clase, estado y código de error | Alta | 1 |
| CORE-04 | Leer driver actual: proveedor, versión, fecha, INF y firma | Alta | 1 |
| CORE-05 | Clasificar cada dispositivo como falta driver, error, actualizable, correcto o desconocido | Alta | 1 |
| CORE-06 | Analizar carpetas de drivers y extraer de cada INF sus IDs, versión, fecha, proveedor, arquitectura y sistemas compatibles | Alta | 1 |
| CORE-07 | Generar `index.json` con esquema versionado, ruta relativa y hash SHA-256 | Alta | 1 |
| CORE-08 | Validar que el índice coincide con los archivos antes de usarlo | Alta | 1 |
| CORE-09 | Comparar candidatos por coincidencia, compatibilidad, firma, origen, versión y fecha | Alta | 1 |
| CORE-10 | Priorizar coincidencias exactas de Hardware ID sobre Compatible IDs | Alta | 1 |
| CORE-11 | Preferir el driver del fabricante del equipo cuando sea compatible | Alta | 2 |
| CORE-12 | Explicar por qué se recomienda cada driver | Media | 1 |
| CORE-13 | Instalar drivers INF con `pnputil` y registrar su salida | Alta | 1 |
| CORE-14 | Detectar si la instalación requiere reinicio y permitir reanudar después | Alta | 1 |
| CORE-15 | Respaldar los drivers instalados antes de hacer cambios | Alta | 1 |
| CORE-16 | Crear un punto de restauración cuando el sistema lo permita | Alta | 1 |
| CORE-17 | Registrar cada acción con fecha, dispositivo, versión anterior, versión nueva y resultado | Alta | 1 |
| CORE-18 | Exportar el análisis y los resultados en HTML y JSON | Media | 1 |
| CORE-19 | Consultar drivers aplicables mediante Windows Update Agent | Alta | 1 |
| CORE-20 | Descargar e instalar los drivers de Windows Update aprobados | Alta | 1 |
| CORE-21 | Integrar catálogos de Dell, Lenovo y HP | Media | 2 |
| CORE-22 | Integrar fuentes oficiales de NVIDIA, AMD e Intel | Media | 2 |
| CORE-23 | Excluir de la selección las versiones marcadas como problemáticas | Alta | 2 |
| CORE-24 | Soportar instaladores oficiales que incluyan software adicional | Media | 2 |
| CORE-25 | Evitar instalar una versión menor salvo que el flujo sea un downgrade autorizado | Alta | 1 |
| CORE-26 | Mantener una lista de dispositivos ignorados por el usuario | Media | 2 |
| CORE-27 | Mostrar la interfaz y los reportes en español, con estructura lista para inglés | Media | 1 |

## Versión portable

La portable debe dejar el equipo con internet sin nada instalado y después completar el escaneo. Su flujo es: detectar red, instalar el driver, confirmar conexión y escanear el resto.

| ID | Requerimiento | Prioridad | Fase |
| --- | --- | --- | --- |
| PORT-01 | Ejecutarse desde la USB sin instalación | Alta | 1 |
| PORT-02 | Solicitar permisos de administrador al abrirse | Alta | 1 |
| PORT-03 | Funcionar sin internet hasta instalar la red | Alta | 1 |
| PORT-04 | Incluir drivers Wi-Fi de Intel, Realtek, MediaTek, Qualcomm Atheros y Broadcom | Alta | 1 |
| PORT-05 | Incluir drivers Ethernet de Realtek, Intel, Killer y adaptadores USB comunes | Alta | 1 |
| PORT-06 | Incluir drivers de almacenamiento Intel RST/VMD y AMD RAID | Media | 2 |
| PORT-07 | Detectar adaptadores de red sin driver o con error | Alta | 1 |
| PORT-08 | Seleccionar e instalar el mejor driver de red desde la librería | Alta | 1 |
| PORT-09 | Probar conectividad real, no solo que el adaptador aparezca | Alta | 1 |
| PORT-10 | Abrir la configuración de Wi-Fi cuando haga falta conectarse a una red | Alta | 1 |
| PORT-11 | Ofrecer el modo de compartir internet desde el celular si no hay driver compatible | Media | 1 |
| PORT-12 | Lanzar el escaneo completo al recuperar la conexión | Alta | 1 |
| PORT-13 | Permitir aprobar o rechazar cada driver antes de instalarlo | Alta | 1 |
| PORT-14 | Guardar y reutilizar kits de drivers por modelo | Media | 2 |
| PORT-15 | Generar un reporte de servicio para el cliente | Media | 2 |
| PORT-16 | Actualizar la librería y regenerar el índice desde una compu con internet | Media | 2 |
| PORT-17 | Verificar los hashes antes de instalar desde la USB | Alta | 1 |
| PORT-18 | Mantener los registros en la USB sin dejar datos personales innecesarios | Media | 1 |
| PORT-19 | Quitar un driver problemático de un Windows que no inicia, desde WinRE | Baja | 5 |

PORT-19 es el flujo de rescate. Debe hacer una copia de seguridad y mostrar una confirmación fuerte antes de modificar el sistema apagado.

## Versión de escritorio

La versión de escritorio instala cambios solo con evidencia suficiente y puede revertirlos. Trabaja como un servicio de Windows con una app en la bandeja del sistema.

**Monitoreo y aplicación**

| ID | Requerimiento | Prioridad | Fase |
| --- | --- | --- | --- |
| DESK-01 | Instalarse como servicio de Windows con inicio automático | Alta | 2 |
| DESK-02 | Incluir una app de bandeja para estado, alertas y aprobaciones | Alta | 2 |
| DESK-03 | Mostrar un panel con estado general, drivers, historial y recomendaciones | Alta | 2 |
| DESK-04 | Ofrecer los modos Observar, Recomendar y Automático | Alta | 2 |
| DESK-05 | Usar Recomendar como modo predeterminado cuando se habilite la instalación | Alta | 4 |
| DESK-06 | Registrar cambios de drivers hechos por Windows Update o por el usuario | Alta | 2 |
| DESK-07 | Revisar pantallazos azules, reinicios inesperados, fallos de video y errores de dispositivos | Alta | 2 |
| DESK-08 | Mantener una línea base de estabilidad de cada equipo | Alta | 2 |
| DESK-09 | Mostrar relaciones entre cambios y fallos, con su nivel de confianza | Alta | 2 |
| DESK-10 | Permitir pausar el monitoreo y las actualizaciones | Media | 2 |

**Actualizaciones**

| ID | Requerimiento | Prioridad | Fase |
| --- | --- | --- | --- |
| DESK-11 | Buscar actualizaciones en un horario configurable | Alta | 4 |
| DESK-12 | Esperar 7 días configurables antes de recomendar una versión nueva | Alta | 4 |
| DESK-13 | Consultar una lista de versiones conocidas como problemáticas | Alta | 4 |
| DESK-14 | Comprobar antes de actualizar que no haya tareas críticas, batería baja o reinicio pendiente | Alta | 4 |
| DESK-15 | Aplicar un solo cambio importante a la vez | Alta | 4 |
| DESK-16 | Dar por validada una actualización tras un periodo de observación configurable | Alta | 4 |
| DESK-17 | En modo Automático, instalar solo drivers de bajo riesgo | Alta | 5 |
| DESK-18 | Pedir siempre aprobación para gráfica, chipset, almacenamiento y red | Alta | 5 |
| DESK-19 | Notificar con claridad qué cambió y cómo revertirlo | Alta | 4 |

**Rollback y conflictos**

| ID | Requerimiento | Prioridad | Fase |
| --- | --- | --- | --- |
| DESK-20 | Conservar un historial de versiones funcionales por dispositivo | Alta | 3 |
| DESK-21 | Hacer rollback con un clic y comprobar el resultado | Alta | 3 |
| DESK-22 | Hacer downgrade quitando el driver nuevo del Driver Store e instalando el anterior | Alta | 3 |
| DESK-23 | Evitar que Windows Update reinstale una versión bloqueada, con permiso del usuario | Alta | 3 |
| DESK-24 | Permitir ver y quitar bloqueos desde la app | Alta | 3 |
| DESK-25 | Sugerir rollback automáticamente cuando la estabilidad empeore | Alta | 3 |
| DESK-26 | Definir combinaciones conocidas de chipset y gráfica que chocan | Media | 4 |
| DESK-27 | Marcar como buena una combinación que ya demostró estabilidad en ese equipo | Media | 4 |
| DESK-28 | Hacer rollback automático solo si el usuario lo habilita y la evidencia es alta | Baja | 5 |
| DESK-29 | Alertar si un driver impide usar internet o el video, con pasos de recuperación | Alta | 3 |

La app no debe afirmar que un driver causó un fallo con solo una coincidencia de tiempo. Debe mostrar la evidencia y el nivel de confianza.

## Seguridad y calidad

La prioridad del producto es no dejar un equipo peor de como lo encontró. Los valores numéricos son metas propuestas y deben ajustarse después de medir en equipos reales.

**Seguridad**

| ID | Requerimiento | Prioridad | Fase |
| --- | --- | --- | --- |
| SEG-01 | Firmar con certificado de código los ejecutables distribuidos | Alta | 2 |
| SEG-02 | Firmar los catálogos y las actualizaciones del programa | Alta | 2 |
| SEG-03 | Verificar firma y hash antes de instalar un driver o una actualización | Alta | 1 |
| SEG-04 | Rechazar drivers sin firma válida o de una fuente no aprobada | Alta | 1 |
| SEG-05 | Usar HTTPS para todas las descargas | Alta | 1 |
| SEG-06 | Dar al servicio solo los permisos necesarios | Alta | 2 |
| SEG-07 | Proteger la comunicación entre la app y el servicio contra usuarios no administradores | Alta | 2 |
| SEG-08 | No enviar datos del equipo sin consentimiento explícito | Alta | 2 |
| SEG-09 | No guardar datos personales en reportes o registros salvo que sean necesarios | Alta | 1 |
| SEG-10 | Pedir confirmación antes de bloquear actualizaciones o cambiar políticas del sistema | Alta | 3 |

**Calidad**

| ID | Requerimiento | Meta propuesta | Fase |
| --- | --- | --- | --- |
| RNF-01 | Tiempo del análisis local | Menos de 30 segundos | 1 |
| RNF-02 | Tiempo de abrir la portable desde USB 3.0 | Menos de 10 segundos | 1 |
| RNF-03 | Tamaño de la librería inicial | Menos de 8 GB | 1 |
| RNF-04 | Consumo del servicio en reposo | Menos de 100 MB de RAM y CPU cercana a 0 % | 2 |
| RNF-05 | Recuperación ante un fallo | Una instalación interrumpida no deja el sistema en estado desconocido | 1 |
| RNF-06 | Reversibilidad | Todo cambio de driver mantiene una ruta de vuelta documentada | 3 |
| RNF-07 | Claridad | Cada alerta dice qué pasó, qué riesgo tiene y qué puede hacer el usuario | 2 |
| RNF-08 | Accesibilidad | Uso completo con teclado y escalado de pantalla | 2 |
| RNF-09 | Registros | Rotación automática y exportación para soporte | 2 |
| RNF-10 | Mantenibilidad | El núcleo no depende de la interfaz | 1 |
| RNF-11 | Compatibilidad futura | La detección y la instalación se separan por sistema operativo para añadir Linux | 2 |

## Requerimientos técnicos

Las dos versiones comparten un núcleo en Rust; solo cambian la forma de ejecutarse y la interfaz.

Ver [arquitectura.md](arquitectura.md).

La portable y el servicio llaman al mismo núcleo; la app de bandeja solo habla con el servicio.

| Área | Requerimiento |
| --- | --- |
| Lenguaje | Rust estable para núcleo, portable y servicio |
| Portable | Un `.exe` x64 autónomo, con ventana nativa y sin WebView2 |
| Escritorio | Servicio de Windows en Rust y app de bandeja con Tauri, React y TypeScript |
| Windows | APIs de SetupAPI o CfgMgr32, WMI, Windows Update Agent y Registro de eventos |
| Instalación | `pnputil`, DISM y Restauración del sistema |
| Datos locales | SQLite para historial, estado y configuración |
| Librería | Carpeta `drivers/`, `index.json`, hashes y `kits/` por modelo |
| Catálogo remoto | JSON firmado con fuentes, versiones recomendadas y versiones bloqueadas |
| Empaquetado | Instalador firmado para escritorio y script para preparar la USB |
| Integración continua | Compilación en Windows, pruebas automáticas y publicación de versiones |

**Herramientas de desarrollo**

- Windows 11 x64 con Rust, Visual Studio Build Tools y Windows SDK.
- Git y GitHub.
- Node.js para la interfaz de Tauri.
- Máquinas virtuales con snapshots para Windows 10 y 11.
- Certificado de firma de código antes de distribuir la versión de escritorio.

**Pruebas**

| ID | Requerimiento | Fase |
| --- | --- | --- |
| TEST-01 | Pruebas unitarias para el analizador INF, el índice y la selección de drivers | 1 |
| TEST-02 | Conjunto de archivos INF reales y dañados para pruebas | 1 |
| TEST-03 | Pruebas en máquinas virtuales de Windows 10 y 11 limpias | 1 |
| TEST-04 | Pruebas en al menos 10 modelos físicos, incluidos Dell, HP, Lenovo y ASUS | 1 |
| TEST-05 | Pruebas de interrupción: apagón, USB retirada y reinicio durante la instalación | 2 |
| TEST-06 | Pruebas de rollback y bloqueo de Windows Update | 3 |
| TEST-07 | Pruebas de actualización y alteración de catálogos firmados | 4 |
| TEST-08 | Periodo beta con equipos propios antes de habilitar el modo Automático | 5 |

## Catálogo, API y distribución

La API publica datos de drivers, no archivos de drivers. Cada entrada apunta a la descarga oficial y trae el hash que permite verificarla.

| ID | Requerimiento | Prioridad | Fase |
| --- | --- | --- | --- |
| API-01 | Recolectar por tareas programadas versiones y drivers nuevos desde fuentes permitidas | Alta | 2 |
| API-02 | Guardar proveedor, versión, fecha, Hardware IDs, modelos, URL oficial, hash y licencia | Alta | 2 |
| API-03 | Marcar cada versión como nueva, en observación, recomendada o bloqueada | Alta | 4 |
| API-04 | Publicar catálogos JSON firmados y fragmentados por proveedor | Alta | 2 |
| API-05 | Permitir que el cliente elija localmente, sin enviar el inventario de su equipo | Alta | 2 |
| API-06 | Conservar versiones anteriores y metadatos aunque una fuente deje de ofrecer la descarga | Alta | 3 |
| API-07 | Mantener la clave de firma fuera de los recolectores y de la API | Alta | 2 |
| API-08 | Alertar cuando un recolector falle o una fuente cambie de formato | Media | 2 |
| API-09 | Respetar los términos, límites y reglas de acceso de cada fuente | Alta | 2 |
| API-10 | Permitir reportes opcionales y anónimos de estabilidad por versión | Baja | 5 |
| DIST-01 | Publicar el código como proyecto gratuito de código abierto, con licencia GPL-3.0 | Alta | 1 |
| DIST-02 | No incluir drivers propietarios en el repositorio | Alta | 1 |
| DIST-03 | Incluir un preparador que descargue los paquetes oficiales y arme la USB en la compu del usuario | Alta | 1 |
| DIST-04 | Registrar licencia, origen, URL y hash de cada paquete | Alta | 1 |
| DIST-05 | Publicar una USB ya armada solo con paquetes cuya licencia permita redistribuirlos | Media | 5 |

## Criterios de aceptación

Cada fase avanza solo cuando cumple sus criterios en equipos reales.

| Fase | Criterio de salida |
| --- | --- |
| 1. Portable | En 9 de 10 modelos de prueba recién formateados se recupera internet sin intervención técnica extra; el escaneo no instala nada sin aprobación |
| 2. Observar | La app detecta los cambios de drivers y los eventos de falla en pruebas controladas, sin modificar el sistema |
| 3. Rollback | Revierte 20 de 20 cambios de prueba y Windows Update no reinstala la versión bloqueada |
| 4. Recomendar | Durante un mes de uso en equipos propios no causa fallos que queden sin ruta de recuperación |
| 5. Automático | Solo instala drivers de bajo riesgo y las pruebas beta no registran equipos inutilizables |

## Preguntas abiertas

**Decisiones tomadas**

- La portable tendrá una ventana nativa.
- El producto se llamará Drivward y será gratis y de código abierto.
- El código usará la licencia GPL-3.0.
- Una API mantendrá actualizado el catálogo de drivers y versiones.

**Por confirmar**

- drivward.com ya está registrado: ¿qué dominio usará el proyecto y quién hará la revisión formal de marca?
- ¿Se aprueba la lista de 10 equipos de prueba: Dell, HP, Lenovo, ASUS, Acer, Intel, AMD, un equipo de 2017 o anterior, uno reciente y una desktop con Ethernet?
- ¿Quién revisará las versiones antes de marcarlas como recomendadas?
- ¿Qué licencia tiene cada paquete de la librería inicial?

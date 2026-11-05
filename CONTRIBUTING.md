# Contribuir

Gracias por ayudar. Drivward toca drivers de equipos reales, así que la prioridad es no dejar una compu peor de como estaba.

1. Abre un issue antes de cambios grandes.
2. Ejecuta antes de enviar un pull request:

   ```sh
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

3. Toda lógica nueva va en `drivward-core` o detrás del trait `Platform`, con pruebas que usen plataformas falsas.
4. Nunca agregues archivos de drivers al repositorio. Los paquetes se describen en el manifiesto con su URL oficial, SHA-256 y licencia.
5. Los textos de la interfaz se escriben en español claro, sin alarmas ni exageraciones. Cada alerta dice qué pasó, qué riesgo tiene y qué puede hacer la persona.

Para probar instalaciones reales usa máquinas virtuales con snapshots de Windows 10 y 11.

# Guía para agentes

Proyecto: **Nookly**, una barra de tareas en Rust + Slint para Windows (`src/` y `ui/`).

- `src/main.rs` tiene el estado (`Estado`, `Ctx`) y arranca todo; lo que hace la barra va por
  partes en `src/barra/` (bóveda, buscador, configuración, personaje, reproductor, tira…), cada
  una con sus métodos de `Ctx` y su `conectar`, que engancha sus eventos de la interfaz.
- `ui/barra.slint` es la ventana de la barra (la tira y el personaje) y junta los paneles, cada
  uno en su archivo de `ui/` (`configuracion.slint`, `boveda.slint`, `buscador.slint`…); los
  colores y datos compartidos están en `ui/tema.slint`, y los iconos y controles en
  `ui/iconos.slint` y `ui/controles.slint`.
- Los **addons** (`src/addons/`, y su parte de la barra en `src/barra/addons.rs`) se cargan en
  tiempo de ejecución desde `%APPDATA%\nook\addons\<id>\`: su `panel.slint` se compila con
  `slint-interpreter` y se mete en el `ComponentContainer` de `ui/barra.slint` (es una función
  experimental de Slint, activada en `build.rs`), y su `logica.rhai` corre en Rhai (`guion.rs`).
  Se instalan desde el catálogo del repositorio `fate-9x/nook-addons` (`catalogo.rs`), donde
  están los addons y la guía para hacerlos. `nook.exe --mcp` (`mcp.rs`) es el servidor MCP para
  la IA del usuario, y habla con la barra por una tubería con nombre (`tuberia.rs`).
- El **personaje** (`src/personaje/`) se dibuja por software con `zeno`. Su figura sale de una
  carpeta con `personaje.json` y un `personaje.svg` por capas o unos PNG (`figura.rs`, que lee el
  SVG con `svg.rs`): Nook y el Limón están en `personajes/` y el usuario hace los suyos en
  `%APPDATA%\nook\personajes\<carpeta>\` (`coleccion.rs`, que los vuelve a leer al cambiar). La
  guía para hacerlos es `docs/personajes.md`.

## Nookly y Nookly Pro

Hay dos versiones, cada una en su repositorio, que comparten casi todo el código:

| Versión | Repositorio | Rama local | Carpeta |
|---|---|---|---|
| **Nookly** (gratuita) | `fate-9x/nook` (público), remoto `origin` | `main` | `Desktop/nook` |
| **Nookly Pro** (de pago, Microsoft Store) | `fate-9x/nook-pro` (privado), remoto `pro` | `pro` | `Desktop/nook-pro` |

Las dos carpetas son el mismo repositorio de git (`git worktree`): cada una tiene su rama, y los
cambios pasan de una a otra sin subir ni bajar nada.

- **Los cambios solo van de la gratuita a la Pro.** Lo que es común (un arreglo, la bóveda, el
  personaje…) se hace en `Desktop/nook` y se pasa a la Pro con `git merge main` desde
  `Desktop/nook-pro`. Lo que es solo de Pro se hace en `Desktop/nook-pro`. Nunca se hace merge de
  `pro` en `main`.
- **El código de las funciones Pro** está en `src/pro/`, que solo existe en la rama `pro`. Con esa
  carpeta, `build.rs` compila con `cfg(pro)` y `src/main.rs` toma los módulos de ahí; sin ella,
  los de `src/gratis/`, que tienen lo mismo pero no hacen nada. En el código, `PRO` dice si es
  Nookly Pro.
- **Una función Pro nueva:** su parte que habla con Windows va en un módulo de `src/pro/` (rama
  `pro`), con su versión vacía en `src/gratis/` (rama `main`, y de ahí a `pro` con el merge) y las
  dos líneas `#[cfg_attr(…, path = …)]` en `src/main.rs`. Sus ajustes se apagan en la gratuita en
  `Ajustes::quitar_funciones_pro` (`src/config.rs`) y sus opciones de la configuración llevan
  `solo-pro: true` (`ui/configuracion.slint`), que en la gratuita las muestra con la marca "PRO".
- **El historial anterior a la separación** (hasta la v0.2.0, con sus tags y releases) solo está
  en `nook-pro`: `main` empieza en un commit nuevo, y `pro` está enlazada a él con un merge.
- **Protección:** los hooks de `.githooks/` no dejan hacer un commit de `src/pro/` en `main` ni
  subir al repositorio público nada que lo lleve, ni el historial anterior a la separación. Se activan una vez por copia del repositorio con
  `git config core.hooksPath .githooks`.
- **Subir:** `main` con `git push origin main`; `pro` con `git push pro pro:main`.

## Commits

- Haz un commit cada vez que termines un cambio sistemático completo y funcional (una
  funcionalidad, una corrección o un grupo de cambios relacionados). No acumules muchos cambios
  sin commit ni mezcles asuntos distintos en uno solo.
- Antes de cada commit comprueba que compila (`cargo build`) y que pasan los tests (`cargo test`).
- El mensaje debe seguir la convención de commits: `tipo: mensaje` en minúsculas y en español,
  por ejemplo `feat: añadir arrastre de la barra`. Tipos: `feat`, `fix`, `perf`, `refactor`,
  `style`, `docs`, `test`, `chore`.
- **No añadas coautores** a los commits: nada de líneas `Co-Authored-By` ni atribuciones a
  Claude u otras herramientas, ni en commits ni en descripciones de pull requests.

## Notas de las releases

Las notas de cada release (en GitHub Releases) se escriben solas a partir de los mensajes de los
commits desde el tag anterior (ver `.github/workflows/release.yml`). Por eso **todo cambio que
note el usuario tiene que quedar en un commit con el tipo correcto**, para que siempre aparezca en
las notas:

| Tipo | Sección de las notas |
|---|---|
| `feat` | **Novedades**: funciones nuevas o cambios en lo que se ve o se puede hacer |
| `fix` | **Correcciones**: errores arreglados |
| `perf`, `refactor`, `style` | **Otros cambios** |
| `docs`, `test`, `chore` | No aparecen |

- Escribe el mensaje pensando en quien lee las notas, no en el código: qué puede hacer ahora o qué
  ya no falla. Cada mensaje aparece tal cual como una línea de la lista, sin el `tipo:`.
  - Bien: `feat: cambiar de sitio las aplicaciones de la barra arrastrándolas`,
    `fix: mantener la barra centrada al cambiar su tamaño`.
  - Mal: `fix: arreglar bug`, `feat: cambios en main.rs`, `chore: mejoras de la barra` (un cambio
    visible no puede ir como `chore`, porque no saldría en las notas).
- Una novedad o corrección por commit, para que cada una tenga su propia línea en las notas.
- Un cambio que rompe algo de antes (por ejemplo, la configuración guardada deja de valer) se marca
  con `!`: `feat!: …`.

### Formato de las notas

Las notas tienen siempre este formato: una sección por tipo, en este orden (Novedades,
Correcciones, Otros cambios; las vacías no salen), y en cada una una línea por commit, de la más
reciente a la más antigua. Ejemplo de una release real:

```markdown
## Novedades

- el personaje rebota al hacer clic en él
- la barra reaparece cuando termina de deslizarse a su sitio
- al arrastrar la barra solo se ve el personaje
- efecto lupa en los iconos de la barra al acercar el cursor

## Correcciones

- el efecto lupa agranda solo el icono que tiene el cursor encima
- mantener la barra centrada al cambiar su tamaño
```

Para que todas las líneas sigan ese estilo, el mensaje del commit (lo que va tras `tipo: `):

- Empieza en minúscula y no lleva punto final.
- Es una sola línea corta en español, que se entiende sin contexto, sin nombres de archivos,
  funciones ni detalles internos. Si hace falta explicar más, va en el cuerpo del commit, que no sale
  en las notas.
- Describe el resultado para el usuario, con un verbo en infinitivo (`quitar un atajo de teclado y
  dejarlo sin definir`) o una frase sobre lo que ahora pasa (`el personaje rebota al hacer clic en
  él`).

Las notas no se escriben ni se corrigen a mano en GitHub: si una línea queda mal, el arreglo es
escribir bien los commits siguientes, y el formato lo mantiene el workflow
(`.github/workflows/release.yml`), que no hay que cambiar salvo que se pida.

## Publicar una versión

Esto es para la versión gratuita (`main`, repositorio público). Cómo se publica Nookly Pro en la
Microsoft Store está por decidir: los tags de las dos versiones comparten el repositorio local,
así que los de Pro no pueden llamarse también `vX.Y.Z`.

- La versión sale del tag: se publica subiendo un tag `vX.Y.Z` (`git tag v0.2.0` y
  `git push origin v0.2.0`). El workflow compila `nook.exe` con esa versión, que es la que se ve
  en la configuración y la que usa la app para buscar actualizaciones. No hace falta cambiar la
  versión del `Cargo.toml`.
- Con él compila también el instalador, `nook-setup.exe` (Inno Setup, `instalador/nook.iss`), y
  sube los dos a la release. La app se actualiza sola descargando el `nook.exe`, así que ese
  nombre no puede cambiar. Para probar el instalador en local hace falta Inno Setup 6:
  `cargo build --release` y luego `ISCC /DVersion=0.0.0 instalador\nook.iss` (lo deja en
  `target\instalador`).
- El workflow no pasa los tests (tardan demasiado en GitHub Actions): pásalos en local con
  `cargo test` antes de crear el tag.
- Elige el número según los commits desde el tag anterior: con algún `feat` sube el segundo número
  (0.2.0 → 0.3.0); solo con `fix`, el tercero (0.2.0 → 0.2.1); con un cambio marcado con `!`, el
  primero.

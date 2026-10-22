# Personajes de Nookly: lo esencial

Un personaje es una carpeta con `personaje.svg` (el dibujo, de frente y quieto) y `personaje.json`
(qué es cada parte). Nookly lo anima solo: mira al cursor girando la cara, parpadea, salta con los
avisos, se alegra al hacerle clic y baila con la música. Tú no animas nada.

Carpeta de personajes del usuario: `{carpeta}` (cada personaje, una subcarpeta).

## Cómo trabajar

1. Pide el ejemplo más parecido con `personaje__guia` y `tipo`. Cópialo y cambia formas, colores y
   partes; no partas de cero ni cambies la estructura sin motivo.
2. Guárdalo con `personaje__guardar` (una carpeta nueva por personaje): te devuelve la comprobación
   y una imagen con 8 vistas (frente, derecha, izquierda, arriba, abajo, aviso, clic y música).
3. Si hay errores o avisos, o algo se ve mal en alguna vista, corrígelo y vuelve a guardar.
4. Cuando quede bien, `personaje__poner` lo pone en la barra del usuario.

## Tipos de ejemplo

| `tipo` | Para |
|---|---|
| `simple` | Una mascota redonda sin nada que sobresalga (mochi, slime, gota, bola). La base de todo. |
| `animal` | Animales con orejas o cuernos a los lados (gato, zorro, perro, oso, conejo, dragón). |
| `pelo` | Personas con pelo: flequillo, pelo alrededor, nuca y moño o coleta. |
| `completo` | Lo más complejo: orejas + pelo en punta + flequillo + cejas + un ojo de cada color. |
| `antena` | Algo encima de la cabeza que se balancea (robot, insecto, alien) y cuerpos no redondos. |
| `fruta` | Comida, plantas y objetos: silueta con forma, hojas encima y algo que asoma al girar. |
| `imagenes` | Pixel art o dibujos en PNG con una animación por momento (no mira al cursor). |

## Reglas que siempre valen (dibujo SVG)

- **Coordenadas:** el centro del cuerpo en (0, 0) y el cuerpo de unos 90-100 de ancho; la y crece
  hacia abajo. El `viewBox` da igual.
- **Cada parte es un elemento o un grupo `<g>` con su `id`.** Nombres fijos: `cuerpo` (la silueta,
  obligatoria), `ojo` (**solo el ojo derecho**, el de x positiva: el izquierdo se hace como su
  reflejo), `ojo-izquierdo` (si los ojos son distintos), `ojo-alegre` (el ojo derecho contento) y
  `boca`. El resto de nombres los eliges tú.
- **En `capas` del json van todas las partes que se dibujan, en orden de pintado** (la primera queda
  debajo). Lo que no está en `capas` no se dibuja. Cada `id` una sola vez y existente en el SVG.
- **Tres tipos de capa para lo que no es cuerpo, ojo ni boca:**
  - `pintado` (el de por defecto): va pintado en la cara (mejillas, manchas, nariz, cejas, pantalla).
    Gira con la cara y **se recorta por la silueta**: nunca sobresale.
  - `franja`: una zona de color que se rellena **desde una línea** (su borde, un trazado abierto)
    hasta fuera del cuerpo, hacia `hacia`. El `fill` del trazado es el color de la zona. Para
    flequillos, barrigas, hocicos y el pelo de la nuca.
  - `cabeza`: **todo lo que sobresale del cuerpo** (orejas, pelo alrededor, cuernos, antenas, hojas).
    Se mueve con la cabeza según su ancla: un circulito sin relleno con id `ancla-<id de la capa>` en
    el punto donde se pega a la cabeza. Las anclas no van en `capas`.
- **Lo que está a los dos lados** (orejas, cuernos) va en **dos capas**, una por lado, cada una con
  su ancla, `"detras": true, "delante": "girar"` y `"profundidad"` de `0` a `-0.2`.
- **Espacio:** caben 1,55 radios del cuerpo hacia arriba, 1,05 hacia abajo y 1,35 a los lados; si
  algo sale más, todo el personaje se encoge.
- **Solo formas con relleno y borde** (`path`, `circle`, `ellipse`, `rect`, `polygon`, `g` con
  `transform`, opacidades). No se dibujan textos ni imágenes, no se aplican máscaras ni recortes, y
  los degradados se pintan con su primer color.
- **Colores del usuario (opcional):** `"colores": { "cuerpo": "#…", "rasgos": "#…" }` cambia cada
  trazado de exactamente ese color por el color de acento del usuario (cuerpo) y por el que se lee
  sobre él (rasgos: ojos y boca). Sin ellos, el personaje queda con los colores del dibujo.
  `"aviso"` es el color del circulito de las notificaciones.
- **Nombre:** `"nombre": { "es": "…", "en": "…" }` es el que sale en la lista.
- **La cara, sin pegarla a los bordes:** al girar, lo que está cerca del borde se aplasta. Ojos a
  unos ±15 del centro y la boca entre 5 y 20 por debajo.

## Si la comprobación avisa

- «no hay nada llamado «x» en el dibujo»: un `id` de `capas` no está en el SVG (o está mal escrito).
- «falta la capa «cuerpo»»: la silueta tiene que tener `id="cuerpo"`.
- «tiene cosas a los dos lados de la cabeza»: separa la capa en una por lado, o, si es pelo que rodea
  la cabeza, ponle `"detras": true, "delante": "nunca"` y una profundidad pequeña.
- «engancha a un lado de la cabeza y no tiene profundidad»: ponle `"profundidad"` de `0` a `-0.2`.
- Algo se despega de la cabeza al mirar a un lado: cambia su `profundidad` de 0,1 en 0,1.

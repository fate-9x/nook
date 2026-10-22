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
| `caricatura` | Personas o personajes de series con contorno: cabeza con su forma, orejas, párpados, nariz y bigote. |
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

## Trucos

- **Copiar un personaje: mide antes de dibujar.** En la imagen más de frente, con el ancho de la
  cabeza como 1, mide su alto y dónde es más ancha, el ancho y el alto de cada ojo, la separación
  entre ellos y su altura, y la altura de la nariz, la boca y las orejas; luego pásalo a
  coordenadas. Una cabeza redonda o unos ojos de otro tamaño lo convierten en otro personaje.
- **Pelo corto (hasta las orejas):** sin franjas de nuca. Basta con el nacimiento del pelo como
  franja hacia arriba, con sus extremos a los lados a la altura a la que acaba el pelo: al girar, el
  motor lo alarga en horizontal y cubre la parte de atrás justo hasta ahí. Las franjas de nuca se
  alargan en vertical y, con el pelo corto, al girar bajan hasta la barbilla.
- **Fijo en el cuerpo:** las formas que van dentro del grupo `cuerpo`, después de la primera, se
  pintan sobre él pero **no giran con la cara** (un cuello de camisa, un caparazón, una barriga
  quieta). Lo pintado, en cambio, gira con la cara.
- **Contorno:** un `stroke` en el `cuerpo` se dibuja fijo sobre la silueta; en una franja, a lo
  largo de su borde.
- **Lo pintado se desvanece por elementos**, según dónde quede el centro de cada uno: dos mejillas o
  dos cejas sueltas desaparecen cada una al pasar al otro lado; algo de un solo elemento centrado
  (una uniceja, una nariz, un bigote) se ve siempre.
- **`"muelle": true` sirve en cualquier capa:** en el `cuerpo`, la parte de arriba se bambolea como
  gelatina (slimes, flanes); en el pelo o una antena, se queda atrás al moverse y luego se asienta.
- **Ojos con párpado, pestañas o varias piezas:** ponles siempre `ojo-alegre`; sin él, el motor
  recorta el ojo en arco al estar contento y solo queda bien con ojos rellenos de una pieza.
- **Semitransparente:** `fill-opacity` funciona (brillos, sombras); y `fill-rule="evenodd"` hace
  agujeros (unas gafas dibujadas, un dónut).
- **Los extremos de una franja, dentro de la cara:** el motor descarta los puntos del borde que
  caen fuera de la cara o en su mismo borde y, desde el primero y el último que quedan, alarga en
  recto hasta fuera del cuerpo (en horizontal las de `arriba`/`abajo`; en vertical las de los
  lados). Si un extremo está pegado al borde de la cabeza, la franja empieza donde no esperas.
- **Profundidad en el pelo:** pon dos franjas iguales, una debajo de un tono más oscuro con las
  puntas algo más abajo (asoman entre los mechones claros) y, si quieres, una tercera del color de
  la piel en sombra, más abajo aún y sin `encima`. Que las tres empiecen y acaben en el mismo punto.
- **Sin líneas sueltas en el pelo:** a las franjas de los lados no les pongas `stroke`; el cambio de
  color ya separa el pelo de la cara.
- **Nada pintado pegado al borde de arriba:** al mirar arriba se aplasta contra él y se ve como una
  mancha oscura.

## Si la comprobación avisa

- «no hay nada llamado «x» en el dibujo»: un `id` de `capas` no está en el SVG (o está mal escrito).
- «falta la capa «cuerpo»»: la silueta tiene que tener `id="cuerpo"`.
- «tiene cosas a los dos lados de la cabeza»: separa la capa en una por lado, o, si es pelo que rodea
  la cabeza, ponle `"detras": true, "delante": "nunca"` y una profundidad pequeña.
- «engancha a un lado de la cabeza y no tiene profundidad»: ponle `"profundidad"` de `0` a `-0.2`.
- Algo se despega de la cabeza al mirar a un lado: cambia su `profundidad` de 0,1 en 0,1.

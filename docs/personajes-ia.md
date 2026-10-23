# Personajes de Nookly: lo esencial

Un personaje es una carpeta con `personaje.svg` (el dibujo, de frente y quieto) y `personaje.json`
(qué es cada parte). Nookly lo anima solo: mira al cursor girando la cara, parpadea, salta con los
avisos, se alegra al hacerle clic y baila con la música. Tú no animas nada.

Carpeta de personajes del usuario: `{carpeta}` (cada personaje, una subcarpeta).

## Cómo trabajar

0. Antes de nada, haz al usuario las preguntas de «Antes de dibujar», más abajo.
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
| `caricatura` | Personas o personajes de series con contorno: cabeza con su forma, orejas, párpados, nariz, bigote y expresiones. |
| `antena` | Algo encima de la cabeza que se balancea (robot, insecto, alien) y cuerpos no redondos. |
| `fruta` | Comida, plantas y objetos: silueta con forma, hojas encima y algo que asoma al girar. |
| `imagenes` | Pixel art o dibujos en PNG con una animación por momento (no mira al cursor). |

## Reglas que siempre valen (dibujo SVG)

- **Coordenadas:** el centro del cuerpo en (0, 0) y el cuerpo de unos 90-100 de ancho; la y crece
  hacia abajo. El `viewBox` da igual.
- **Cada parte es un elemento o un grupo `<g>` con su `id`.** Nombres fijos: `cuerpo` (la silueta,
  obligatoria), `ojo` (**solo el ojo derecho**, el de x positiva: el izquierdo se hace como su
  reflejo), `ojo-izquierdo` (si los ojos son distintos), `ojo-alegre` (el ojo derecho contento),
  `boca` y `oreja` (**solo la derecha**: la izquierda es su reflejo, salvo que dibujes
  `oreja-izquierda`). El resto de nombres los eliges tú.
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
- **Orejas: la capa `oreja`.** Dibuja la derecha pegada al lado de la cabeza, con el relleno
  entrando un poco en ella y la línea de contorno solo por fuera (no por la base): el motor la funde
  con la cabeza (le tapa el contorno), la engancha por su lado de dentro, la refleja para el otro
  lado y la esconde detrás al girar. No necesita opciones en el json. En `capas`, ponla después
  del cuerpo y de las franjas de pelo de los lados: como pasa a ser parte de la silueta, lo que se
  pinte después hasta el borde de la cabeza la tapa.
- **Lo demás que está a los dos lados** (cuernos, alas) va en **dos capas** de tipo `cabeza`, una
  por lado, cada una con su ancla, `"detras": true, "delante": "girar"` y `"profundidad"` de `0` a
  `-0.2`.
- **Espacio:** caben 1,55 radios del cuerpo hacia arriba, 1,05 hacia abajo y 1,35 a los lados; si
  algo sale más, todo el personaje se encoge.
- **Solo formas con relleno y borde** (`path`, `circle`, `ellipse`, `rect`, `polygon`, `g` con
  `transform`, opacidades). No se dibujan textos ni imágenes, no se aplican máscaras ni recortes, y
  los degradados se pintan con su primer color.
- **Colores del usuario (opcional):** `"colores": { "cuerpo": "#…", "rasgos": "#…" }` cambia cada
  trazado de exactamente ese color por el color de acento del usuario (cuerpo) y por el que se lee
  sobre él (rasgos: ojos y boca). Sin ellos, el personaje queda con los colores del dibujo.
  `"aviso"` es el color del circulito de las notificaciones.
- **Nombre:** `"nombre": "…"`, el que sale en la lista (un solo texto vale para todos los idiomas).
- **La cara, sin pegarla a los bordes:** al girar, lo que está cerca del borde se aplasta. Ojos a
  unos ±15 del centro y la boca entre 5 y 20 por debajo.

## Párpados y expresiones

- **Párpados:** `"parpado": 0.4` en el json y el motor los dibuja: tapa ese tanto de cada ojo por
  arriba con el color del cuerpo, recortado a la forma del ojo, con una línea del color y el grosor
  del contorno del ojo (si lo tiene), y parpadea bajándolos. No los dibujes tú. Con
  `"parpado": { "cierre": 0.4, "estilo": "cortado" }` el contorno del ojo solo se ve por debajo
  de la línea del párpado (el ojo queda cortado por arriba); con `"abierto"`, el contorno sube por
  los lados por encima del párpado y queda abierto solo en lo alto; por defecto, `"cerrado"`, se ve entero.
- **Piezas que miran hacia un lado** (una nariz de perfil, un mechón, una sonrisa torcida): en su
  capa, `"voltear": "derecha"` (o `"izquierda"`), el lado hacia el que mira tal como la dibujas.
  De frente y mirando hacia ese lado se ve tal cual; en cuanto la cabeza mira un poco hacia el
  otro, el motor la refleja sobre su centro. Solo en capas pintadas en la cara.
- **Cejas:** dibuja solo la derecha con `id="ceja"` (y `ceja-izquierda` si son distintas): el motor
  la mueve con el ojo, así que al mirar arriba o abajo no se separa de él, y sube un poco al
  sorprenderse. Unas cejas pintadas en la cara, en cambio, se quedan atrás al mirar abajo.
- **Expresiones:** el usuario elige una en la configuración. Cada una cambia unas capas por otras
  piezas del dibujo y, si quiere, los párpados; lo demás sigue igual:

  ```json
  "expresiones": {
    "Enojado": { "cejas": "cejas-enojado", "boca": "boca-enojado", "parpado": 0.5 },
    "Triste": { "cejas": "cejas-triste", "boca": "boca-triste", "parpado": 0.45 },
    "Sorprendido": { "boca": "boca-sorprendido", "parpado": 0.1 }
  }
  ```

  - El nombre es el que ve el usuario; salen en este orden, después de «Normal».
  - Cada pieza (`cejas-enojado`…) es un elemento o grupo del SVG **fuera de las capas** (no va en
    `capas`), dibujado en su sitio; se dibuja como la capa a la que sustituye.
  - Una capa puede estar vacía en la cara normal si alguna expresión le da piezas: pon
    `{"id": "cejas"}` en `capas` sin nada llamado `cejas` en el dibujo, y solo tendrá cejas al
    enfadarse o entristecerse.
  - Compruébalas con `personaje__ver` y su `expresion`.

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
- **Pupilas que siguen al cursor:** dentro del grupo del ojo, dale a la pupila (o a un grupo con
  la pupila y su brillo) un `id` que empiece por `pupila` (`pupila`, `pupila-izquierda`…). Además
  de girar la cabeza, se mueve dentro del ojo hacia donde mira, por el sitio que le deja el resto
  del ojo. Va en su orden dentro del grupo: lo que esté después (un párpado) la tapa. Para limitar
  por dónde se mueve (que no se meta bajo el párpado), dibuja un rectángulo sin relleno con id
  `ancla-pupila` (y `ancla-pupila-izquierda` si hay `ojo-izquierdo`): es la zona por la que puede
  ir el centro de la pupila, y puede ser asimétrica (por ejemplo, empezar justo bajo el párpado).
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
- **Límite de giro:** si al girar mucho algo se ve raro (el pelo se despega, un ojo se aplasta
  contra el borde), pon en el json `"giro": 22` (grados a cada lado; sin él, unos 35) y, si hace
  falta, `"cabeceo": 18` (arriba y abajo). Mira con `personaje__ver` y `guinada` a partir de qué
  ángulo falla. La cabeza se para ahí; las pupilas siguen hasta el borde del ojo.
- **Pelo que gira con la cabeza:** a un pelo de tipo `cabeza` que la rodea y va delante (en punta,
  alborotado), ponle `"orientacion": "envolver"`, `"detras": true` y una `"profundidad"` como la
  de la cara (unos `0.6`): al girar, el lado hacia el que mira se estrecha y pasa por detrás de la
  cabeza, y el otro se ensancha, en vez de moverse todo el pelo de una pieza.
- **Barbilla que gira (caras de persona):** pon `"menton": 0.5` en el json (de 0 a 1, cuánto sale
  la barbilla hacia delante). Al girar la cabeza, la barbilla y la mandíbula se van hacia donde mira
  con la boca; sin él, el contorno se queda quieto y la boca se aparta de la barbilla. Para
  mascotas redondas, no lo pongas.
- **Nada pintado pegado al borde de arriba:** al mirar arriba se aplasta contra él y se ve como una
  mancha oscura.

## Si la comprobación avisa

- «no hay nada llamado «x» en el dibujo»: un `id` de `capas` no está en el SVG (o está mal escrito).
- «falta la capa «cuerpo»»: la silueta tiene que tener `id="cuerpo"`.
- «tiene cosas a los dos lados de la cabeza»: separa la capa en una por lado, o, si es pelo que rodea
  la cabeza, ponle `"detras": true, "delante": "nunca"` y una profundidad pequeña.
- «engancha a un lado de la cabeza y no tiene profundidad»: ponle `"profundidad"` de `0` a `-0.2`.
- Algo se despega de la cabeza al mirar a un lado: cambia su `profundidad` de 0,1 en 0,1.

# Guía: crea tu propio personaje para Nookly

Esta guía te lleva paso a paso, desde una carpeta vacía hasta tener tu personaje en la barra. No
hace falta programar: un personaje es un dibujo y un archivo de texto corto. Cuando quieras ver
todas las opciones, están en la [referencia](personajes.md).

Hay dos maneras de hacerlo:

| | Dibujo vectorial (SVG) | Imágenes (PNG) |
|---|---|---|
| Con qué se hace | Inkscape, Figma, Illustrator… | Cualquier programa de dibujo o de pixel art |
| Mira al cursor y parpadea | Sí | No (mira siempre de frente) |
| Cada momento puede ser un dibujo distinto | No (Nookly lo anima) | Sí (reposo, aviso, música, clic) |
| Empieza por | [El tutorial del gato](#tutorial-1-un-gato-vectorial) | [El tutorial del fantasma](#tutorial-2-un-fantasma-de-pixel-art) |

Los dos tutoriales tienen el resultado listo en [`ejemplos/`](ejemplos): si prefieres, copia esa
carpeta y ve cambiándola.

---

## Antes de empezar: dónde viven los personajes

Cada personaje es **una carpeta** dentro de la carpeta de personajes de Nookly:

```
%APPDATA%\nook\personajes\
├── gato\
│   ├── personaje.json
│   └── personaje.svg
└── fantasma\
    ├── personaje.json
    ├── reposo-1.png
    └── …
```

Para llegar a ella: abre la **configuración** de Nookly → sección **Personaje** → **Abrir carpeta**
(en el apartado «Tus personajes»). Si no existe todavía, se crea.

Nookly vigila esa carpeta: **en cuanto guardas un archivo, el cambio se ve en la barra** (en un
segundo más o menos), sin reiniciar nada. Deja la configuración abierta en la sección Personaje
mientras trabajas: la vista previa grande se mueve igual que el personaje de la barra.

### El camino más rápido

Si solo quieres un Nook con otros colores o retocado, pulsa **Nuevo personaje**: se crea una copia
de Nook en una carpeta nueva, se elige y se abre su carpeta. Abre su `personaje.svg` con Inkscape,
cambia lo que quieras, guarda… y ya está.

---

## Tutorial 1: un gato vectorial

Vas a hacer este gato: un cuerpo naranja, dos orejas que giran con la cabeza, unas rayas, los ojos
y una boca. Tardarás unos 15 minutos.

### Paso 1. Crea la carpeta

En la carpeta de personajes, crea una carpeta llamada `gato`. El nombre de la carpeta es solo para
ti; el que sale en Nookly lo pondrás después.

### Paso 2. Dibuja el cuerpo

Abre Inkscape y crea un documento nuevo (el tamaño da igual).

1. Dibuja una elipse grande y rellénala de naranja. Ese es el **cuerpo**: la silueta del personaje.
   Puede tener la forma que quieras.
2. Ponle nombre: con la elipse seleccionada, abre **Objeto → Propiedades del objeto**
   (`Ctrl+Mayús+O`), escribe `cuerpo` en **ID** y pulsa **Establecer**.

> **Así se nombra cada parte.** Nookly reconoce las partes del dibujo por su **ID**. También vale
> el nombre de una **capa** de Inkscape (panel *Capas y objetos*, `Ctrl+Mayús+L`) o el de un
> **grupo**: todo lo que haya dentro cuenta como esa parte. Usa nombres sin espacios ni tildes.

Dibuja **de frente y quieto**, con la cara mirándote. Nookly se encarga de moverlo.

### Paso 3. Dibuja los ojos y la boca

1. Dibuja **solo el ojo derecho** (el que queda a la derecha según lo miras), en su sitio, y ponle
   el ID `ojo`. Nookly hace el izquierdo como su reflejo.
2. Dibuja la boca (por ejemplo, una línea curva con forma de «w») y ponle el ID `boca`. Si tiene
   varias piezas (una lengua, un diente), agrúpalas (`Ctrl+G`) y ponle el ID al grupo.

Los ojos y la boca se pintan sobre la cara como sobre una pelota: cuando el gato mire a un lado, se
irán hacia ese lado y se estrecharán. Además, los ojos parpadean, se abren de sorpresa con los avisos
y, si son rellenos, se vuelven arcos «∩» cuando está contento.

### Paso 4. Guarda y mira cómo queda

Guarda el dibujo en la carpeta `gato` con el nombre **`personaje.svg`** (vale el formato «SVG de
Inkscape» o «SVG simple»).

Con solo eso ya funciona: abre la configuración → **Personaje**, y en la lista de **Figura**
aparecerá `gato`. Elígelo. Si mueves el cursor, te seguirá con la mirada.

### Paso 5. Añade las orejas

Las orejas salen de la cabeza, así que no van en el cuerpo: van en su propia capa, que se moverá con
la cabeza.

1. Dibuja la oreja izquierda (un triángulo) y ponle el ID `oreja-izquierda`. Haz lo mismo con la
   derecha: `oreja-derecha`. Cada oreja va **en su propia capa**: así Nookly puede esconder detrás
   de la cabeza la que queda al otro lado cuando el gato gira.
2. Dibuja un circulito pequeño en la base de cada oreja, donde se «engancha» a la cabeza, y ponles
   los ID `ancla-oreja-izquierda` y `ancla-oreja-derecha`. Quítales el relleno: las anclas no se
   dibujan, solo marcan un sitio.

Si guardas ahora, las orejas **no aparecen**: falta decirle a Nookly qué son. Eso va en el paso 7.
Para entender bien cómo se mueve lo que sale de la cabeza (y no llevarte sorpresas con el pelo o las
orejas de tus personajes), mira [Que gire bien](#que-gire-bien-lo-que-sale-de-la-cabeza).

### Paso 6. Añade las rayas

Dibuja unas rayas en la frente, agrúpalas y ponle al grupo el ID `rayas`. Como van pintadas en la
cara, girarán con ella y nunca se saldrán del cuerpo.

### Paso 7. Escribe el personaje.json

Crea en la carpeta `gato` un archivo de texto llamado **`personaje.json`** (con el Bloc de notas o
cualquier editor) con esto:

```json
{
  "nombre": { "es": "Gato", "en": "Cat" },
  "colores": { "aviso": "#e0455a" },
  "capas": [
    { "id": "oreja-izquierda", "tipo": "cabeza", "detras": true, "delante": "girar", "profundidad": -0.1 },
    { "id": "oreja-derecha", "tipo": "cabeza", "detras": true, "delante": "girar", "profundidad": -0.1 },
    { "id": "cuerpo" },
    { "id": "rayas" },
    { "id": "ojo" },
    { "id": "boca" }
  ]
}
```

Qué dice:

- **`nombre`**: cómo sale en la lista, en cada idioma.
- **`colores.aviso`**: el color del circulito que sale cuando llega una notificación.
- **`capas`**: las partes del dibujo **en el orden en que se pintan** (las primeras quedan debajo).
  - Las orejas son de tipo **`cabeza`**: se mueven con la cabeza. Con `"detras": true` se ven
    también por detrás del cuerpo, y con `"delante": "girar"` cada una pasa por delante cuando el
    gato gira la cabeza hacia su lado y se esconde detrás cuando la gira hacia el otro. Con
    `"profundidad": -0.1` están un poco hacia atrás, en los lados de la cabeza (ver [Que gire
    bien](#que-gire-bien-lo-que-sale-de-la-cabeza)). Por eso van las primeras: debajo de todo.
  - `cuerpo`, `ojo` y `boca` no necesitan tipo: lo saben por su nombre.
  - `rayas` no dice tipo, así que es **pintado** (pintado en la cara).

Guarda. Las orejas aparecen, y el nombre en la lista pasa a ser «Gato».

> Cuando hay un `personaje.json` con `capas`, **solo se dibuja lo que está en la lista**. Si añades
> algo al dibujo y no sale, revisa que esté aquí.

### Paso 8. Pruébalo

- Mueve el cursor de un lado a otro del gato: gira la cabeza, y la oreja del lado contrario se
  esconde detrás.
- En la configuración, **Probar aviso**: salta y le sale el circulito rojo.
- Pon música: baila.
- Ponle accesorios (gafas, gorro…): se colocan sobre su cara y su cabeza.
  ¿Quieres hacer tú los accesorios? Mira [Accesorios propios](personajes.md#accesorios-propios).

¡Ya tienes tu gato! El resultado completo está en [`ejemplos/gato`](ejemplos/gato).

### Si usas Figma

Figma también vale. El ID de cada parte es **el nombre de su capa**. Al exportar el marco como SVG,
abre las opciones de exportación y marca **«Include "id" attribute»**; sin eso, los nombres no
llegan al archivo. Las anclas pueden ser círculos sin relleno, como en Inkscape.

---

## Tutorial 2: un fantasma de pixel art

Ahora un personaje hecho de imágenes: un fantasma de 24 × 24 píxeles que parpadea y se pone naranja
cuando llega un aviso.

### Paso 1. Dibuja las imágenes

Con tu programa de dibujo o de pixel art favorito (Aseprite, Piskel, Krita, Photoshop…), dibuja:

- `reposo-1.png`: el fantasma con los ojos abiertos.
- `reposo-2.png`: el mismo, con los ojos cerrados.
- `aviso-1.png`: el fantasma de otro color, para cuando llega una notificación.

Reglas:

- **Fondo transparente.** Lo que no es transparente en la primera imagen del reposo es el cuerpo, y
  su tamaño es el que manda.
- **Todas las imágenes del mismo tamaño**, con el personaje en el mismo sitio en todas (si no, daría
  saltos al cambiar de imagen).
- PNG, de hasta 1024 × 1024 píxeles y 64 imágenes en total.

Guárdalas en una carpeta nueva, `fantasma`, dentro de la de personajes.

### Paso 2. Escribe el personaje.json

```json
{
  "nombre": { "es": "Fantasma pixel", "en": "Pixel ghost" },
  "imagenes": {
    "reposo": ["reposo-1.png", "reposo-1.png", "reposo-1.png", "reposo-2.png"],
    "aviso": ["aviso-1.png"],
    "fps": 4,
    "pixelado": true
  }
}
```

- Cada animación es una lista de imágenes que se repite a **`fps`** imágenes por segundo. Repetir
  `reposo-1.png` tres veces y luego `reposo-2.png` hace que **parpadee** de vez en cuando.
- **`aviso`** se ve cuando llega una notificación. También puedes poner **`musica`** (cuando suena
  música) y **`clic`** (al hacer clic en él). Las que no pongas usan el reposo.
- **`"pixelado": true`** amplía las imágenes sin suavizar, para que los píxeles se vean nítidos. Para
  dibujos normales, quítalo.

Guarda y elígelo en la configuración. Salta, se estira y baila como los demás, pero **mira siempre de
frente**: las imágenes no pueden girar la cara.

El resultado completo está en [`ejemplos/fantasma`](ejemplos/fantasma).

---

## Darle más vida (dibujo vectorial)

Algunas ideas, todas con capas del `personaje.json` (los detalles están en la
[referencia](personajes.md)):

| Quiero… | Cómo |
|---|---|
| Que tome el color de acento que elige el usuario | Pinta el cuerpo de un color y pon ese color en `"colores": { "cuerpo": "#…" }`. Haz lo mismo con `"rasgos"` para los ojos y la boca. |
| Unos ojos distintos cuando está contento | Dibuja el ojo derecho contento con el ID `ojo-alegre`. |
| Ojos diferentes (uno guiñado, un parche…) | Dibuja el izquierdo con el ID `ojo-izquierdo`. |
| Un flequillo o una barriga de otro color | Una capa de tipo `franja`: dibuja **solo el borde**, como una línea de izquierda a derecha, y di hacia dónde se rellena (`"hacia": "arriba"` o `"abajo"`). |
| Pelo que tape los ojos y se mueva | Una `franja` con `"encima": true` y `"muelle": true`. |
| Una antena que se balancee | Una capa de tipo `cabeza` con `"muelle": true`. |
| Algo que se vea solo al mirarlo (como el arándano del Limón) | Una capa de tipo `cabeza` con `"detras": true` y `"delante": "revelar"`. |
| Que un gorro no esconda las orejas | En su capa, `"tapable": false`. |

Mira también cómo están hechos los dos personajes de Nookly: [Nook](../personajes/nook) (un rulo
que se funde con el cuerpo) y [el Limón](../personajes/limon) (franjas, pelo con muelle y un
arándano que asoma).

---

## Que gire bien: lo que sale de la cabeza

Cuando el personaje mira a un lado, su silueta no cambia: lo que gira es lo que va **sobre** la
cabeza. Los ojos, la boca y lo pintado se deslizan por la cara como sobre una pelota, y lo que sale
de la cabeza (capas de tipo `cabeza`: pelo, orejas, cuernos, antenas…) se mueve según **dónde está
enganchado**. Si eso no está bien puesto, al girar la cabeza el pelo o las orejas se despegan de
ella, aunque de frente se vea perfecto.

### Dónde se engancha: el ancla y su profundidad

Cada capa de tipo `cabeza` se engancha a la cabeza en un punto, su **ancla**:

- **Dónde está en el dibujo** lo dice el circulito `ancla-<capa>` (sin él, abajo y en medio de lo
  que dibuja la capa).
- **Cuánto hacia delante o hacia atrás** está lo dice `"profundidad"`. Sin ella, Nookly pone el
  ancla **en la superficie de la cara** que hay delante de ese punto: como si estuviera pegada a la
  piel, por delante.

Eso es lo que quieres para algo pegado por delante (un rulo en la frente, una antena en lo alto),
pero **no** para lo que está detrás de la cabeza o a sus lados. Al girar, lo de delante se va hacia
donde mira el personaje y lo de detrás hacia el otro lado, como en una cabeza de verdad.

`"profundidad"` se mide en radios del cuerpo: `0` es el plano del centro de la cabeza, los positivos
van hacia delante y los negativos hacia atrás.

| Qué es | Cómo ponerlo |
|---|---|
| **Pelo que rodea la cabeza** (pinchos alrededor, una melena que asoma por los lados) | `"detras": true`, `"delante": "nunca"` y una profundidad **pequeña**, de `0` a `-0.15`. Sin profundidad, al girar se va hacia donde mira, como si estuviera pegado a la frente; con mucha, se va entero hacia el otro lado y parece que está en la nuca. |
| **La parte de atrás de la cabeza cubierta de pelo** (la que se ve al girar) | Dos franjas a los lados, ver [abajo](#la-parte-de-atrás-de-la-cabeza). |
| **Algo en la nuca** (una coleta, un moño, una capucha) | `"detras": true`, `"delante": "nunca"` y una profundidad **negativa**, de `-0.3` a `-0.5`: al girar se va hacia el otro lado, como la parte de atrás de una cabeza. |
| **Orejas, cuernos o alas a los lados** | **Una capa por lado**, cada una con su ancla en su base (donde se pega a la cabeza), `"detras": true`, `"delante": "girar"` y una profundidad de `0` a `-0.2`. Si las dos van en una capa con un ancla en medio, se mueven juntas con la cara y no se esconde la del otro lado. |
| **Algo encima de la cabeza** (una antena, un rulo, un sombrerito) | El ancla en su base, sobre la cabeza. No hace falta profundidad. |
| **Algo delante de la cara** (una nariz grande, un bigote, un pico) | Mejor de tipo `pintado` (o `boca`, si va con ella): así se desliza por la cara con los ojos. |

Por ejemplo, el pelo de un personaje con pinchos alrededor de la cabeza:

```json
{ "id": "pelo", "tipo": "cabeza", "detras": true, "delante": "nunca", "profundidad": -0.15 }
```

Lo que sale de la cabeza es una pieza plana y rígida: al girar se desplaza entera, sin cambiar de
forma. Por eso el pelo que rodea la cabeza casi no tiene que moverse (como la silueta de la cabeza,
que tampoco se mueve), y lo que se ve girar es la cara.

### La parte de atrás de la cabeza

Al mirar a un lado, la cara se desliza hacia ese lado y, por el otro, queda a la vista lo que sería
la parte de atrás de la cabeza. Si tu personaje tiene pelo ahí (Rick, por ejemplo), dibújalo con
**dos franjas, una a cada lado**:

1. Dibuja una línea pegada al borde izquierdo de la cabeza, por dentro, desde arriba hasta donde
   acaba el pelo (a la altura de la oreja, por ejemplo), y al final llévala hacia fuera de la cabeza:
   así la franja no sigue hacia abajo. Ponle el ID `nuca-izquierda` y el color del pelo. Haz lo
   mismo en el lado derecho: `nuca-derecha`.
2. En `capas`, antes de los ojos:
   ```json
   { "id": "nuca-izquierda", "tipo": "franja", "hacia": "izquierda" },
   { "id": "nuca-derecha", "tipo": "franja", "hacia": "derecha" },
   ```

De frente, como el borde está pegado al de la cabeza, casi no se ven. Al mirar a la derecha, la
línea izquierda se va con la cara y la franja cubre de pelo lo que queda detrás; al mirar a la
izquierda, al revés.

Por ejemplo, para una cabeza de 64 de ancho (de -32 a 32) con el centro en (0, 0):

```svg
<path id="nuca-izquierda" d="M -12 -44 C -24 -42 -31 -28 -30 -8 L -30 6 L -36 14" fill="none" stroke="#a6dcf0"/>
<path id="nuca-derecha" d="M 12 -44 C 24 -42 31 -28 30 -8 L 30 6 L 36 14" fill="none" stroke="#a6dcf0"/>
```

### La cara, sin pegarla a los bordes

Al girar, lo que está cerca del borde de la cabeza se aplasta contra él. Deja algo de sitio entre
los ojos (o la uniceja, las mejillas…) y el borde: si los ojos ocupan todo el ancho de la cabeza,
de lado se verán apretados.

### Cómo comprobarlo

Con la configuración abierta en **Personaje**, mueve el cursor muy a la izquierda, muy a la
derecha, arriba y abajo, y mira la vista previa: el personaje gira hasta unos 35° a los lados.
Si algo se despega, cambia su profundidad de 0,1 en 0,1 y guarda: el cambio se ve al momento.

---

## Compartir tu personaje

Comprime la carpeta del personaje en un `.zip` y compártela. Quien la reciba solo tiene que
descomprimirla dentro de su carpeta de personajes (configuración → Personaje → Abrir carpeta) y
elegirlo en la lista.

---

## Si algo no sale bien

Cuando un personaje tiene un error, la configuración lo dice en el apartado **«Personajes que no se
pueden leer»**. Mientras lo arreglas, se queda la última versión que funcionaba.

| Lo que pasa o lo que dice | Qué hacer |
|---|---|
| «falta la capa «cuerpo»» | Ponle el ID `cuerpo` a la silueta (paso 2). Revisa que esté escrito igual, en minúsculas. |
| «no hay nada llamado «x» en el dibujo» | En `capas` hay un `id` que no existe en el SVG. Revisa el nombre en el dibujo y en el JSON. |
| «personaje.json: unknown field …» | Hay una opción mal escrita (por ejemplo, `encma` en vez de `encima`). |
| «personaje.json: expected …» o «EOF» | Al JSON le falta o le sobra algo: una coma, una comilla, una llave. Pégalo en un validador de JSON. |
| Algo del dibujo no aparece | Si hay `capas` en el JSON, solo se dibuja lo que está en la lista. Y lo que no tiene ID (ni está en un grupo con ID) no se dibuja. |
| Un texto del dibujo no aparece | Los textos no se dibujan: en Inkscape, **Trayecto → Objeto a trayecto**. |
| Un degradado sale de un solo color | Los degradados se pintan con su primer color. |
| De frente se ve bien, pero al mirar a un lado el pelo o las orejas se despegan de la cabeza | Les falta la profundidad: mira [Que gire bien](#que-gire-bien-lo-que-sale-de-la-cabeza). |
| Al mirar a un lado, la parte de atrás de la cabeza se ve sin pelo | Añade dos franjas a los lados: mira [La parte de atrás de la cabeza](#la-parte-de-atrás-de-la-cabeza). |
| El personaje se ve pequeño | Si algo sale mucho del cuerpo (una antena muy alta), todo se encoge para que quepa en la barra al saltar. |
| Los ojos no siguen al cursor | Los personajes de imágenes miran siempre de frente. Para que mire, hazlo en SVG. |
| «todas las imágenes tienen que medir lo mismo» | Exporta todas las imágenes con el mismo tamaño de lienzo. |
| Las imágenes se ven borrosas | Si es pixel art, añade `"pixelado": true`. |
| No aparece en la lista y no hay error | La carpeta tiene que estar dentro de la de personajes (no más adentro) y tener un `personaje.svg` o un `personaje.json`. Las carpetas que empiezan por un punto se ignoran. |

# Guía: crea tu propio personaje para Nookly

Esta guía te lleva paso a paso, desde una carpeta vacía hasta tener tu personaje en la barra. No
hace falta programar: un personaje es un dibujo y un archivo de texto corto. Al final está la
[referencia](#referencia-todas-las-opciones) con todas las opciones del motor, una por una: cada
nombre de parte, cada campo del json, cada valor que puede tener y lo que hace Nookly solo.

Hay dos maneras de hacerlo:

| | Dibujo vectorial (SVG) | Imágenes (PNG) |
|---|---|---|
| Con qué se hace | Inkscape, Figma, Illustrator… | Cualquier programa de dibujo o de pixel art |
| Mira al cursor y parpadea | Sí | No (mira siempre de frente) |
| Cada momento puede ser un dibujo distinto | No (Nookly lo anima) | Sí (reposo, aviso, música, clic) |
| Empieza por | [El tutorial del gato](#tutorial-1-un-gato-vectorial) | [El tutorial del fantasma](#tutorial-2-un-fantasma-de-pixel-art) |

Los dos tutoriales tienen el resultado listo en [`ejemplos/`](ejemplos): si prefieres, copia esa
carpeta y ve cambiándola. En la misma carpeta están también [`minimo`](ejemplos/minimo) (lo menos
que puede ser un personaje: solo cuerpo, ojo y boca, sin `personaje.json`) y
[`trucos`](ejemplos/trucos), que usa todas las opciones del motor.

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

La carpeta de cada personaje va directamente dentro de la de personajes (no una dentro de otra).
Las carpetas cuyo nombre empieza por un punto (`.`) se ignoran. El nombre de la carpeta es solo
para ti y para el archivo: el que sale en Nookly lo pondrás en el `personaje.json`.

Nookly vigila esa carpeta: **en cuanto guardas un archivo, el cambio se ve en la barra** (en un
segundo más o menos), sin reiniciar nada. Deja la configuración abierta en la sección Personaje
mientras trabajas: la vista previa grande se mueve igual que el personaje de la barra. Si algo está
mal, la configuración lo dice en el apartado **«Personajes que no se pueden leer»**, y mientras lo
arreglas se queda la última versión que funcionaba.

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
  ¿Quieres hacer tú los accesorios? Mira [Accesorios propios](#accesorios-propios).

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
[referencia](#referencia-todas-las-opciones)):

| Quiero… | Cómo |
|---|---|
| Que tome el color de acento que elige el usuario | Pinta el cuerpo de un color y pon ese color en `"colores": { "cuerpo": "#…" }`. Haz lo mismo con `"rasgos"` para los ojos y la boca. |
| Unos ojos distintos cuando está contento | Dibuja el ojo derecho contento con el ID `ojo-alegre`. |
| Ojos diferentes (uno guiñado, un parche…) | Dibuja el izquierdo con el ID `ojo-izquierdo`. |
| Un flequillo o una barriga de otro color | Una capa de tipo `franja`: dibuja **solo el borde**, como una línea de izquierda a derecha, con el **relleno** del color de la franja, y di hacia dónde se rellena (`"hacia": "arriba"` o `"abajo"`). Si le pones también **línea**, se dibuja a lo largo del borde. |
| Pelo que tape los ojos y se mueva | Una `franja` con `"encima": true` y `"muelle": true`. |
| Una antena que se balancee | Una capa de tipo `cabeza` con `"muelle": true`. |
| Algo que se vea solo al mirarlo (como el arándano del Limón) | Una capa de tipo `cabeza` con `"detras": true` y `"delante": "revelar"`. |
| Que un gorro no esconda las orejas | En su capa, `"tapable": false`. |

Mira también cómo están hechos los dos personajes de Nookly: [Nook](../personajes/nook) (un rulo
que se funde con el cuerpo) y [el Limón](../personajes/limon) (franjas, pelo con muelle y un
arándano que asoma). Y [`trucos`](ejemplos/trucos), un personaje que junta todas las opciones de la
referencia, para copiar y desarmar.

---

## Hacerlo con una IA

Nookly trae un servidor MCP para la IA del usuario (Claude Desktop, Claude Code…), el mismo que
usan sus addons. Con las herramientas `personaje__` la IA puede hacer personajes enteros, y
corregirse sola:

1. **`personaje__guia`** le da esta guía entera, sin que se la copies.
2. Escribe la carpeta del personaje (un `personaje.svg`, o los PNG y su `personaje.json`) donde
   quiera.
3. **`personaje__comprobar`** le dice si se puede leer, qué avisos tiene y cómo lo entiende Nookly
   (capas, ojos, boca, medidas). Itera hasta que no queden errores ni avisos.
4. **`personaje__ver`** la dibuja y le enseña la imagen: por defecto, un mosaico con el personaje
   de frente, mirando a los dos lados y hacia arriba y abajo, con una notificación, al hacerle clic
   y con música; también una vista sola, con los ángulos que le pidas.
5. Cuando ya quede bien, **`personaje__poner`** lo elige en tu barra y lo ves enseguida.

Para que la IA vea las herramientas, tiene que tener a Nookly entre sus servidores MCP: en la
configuración, sección **Addons**, está la conexión con Claude Desktop y el comando para Claude
Code (la misma que la de los addons). Comprobar y ver personajes no hace falta que la barra esté
abierta; ponerlos sí la despierta.

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
   así la franja no sigue hacia abajo. Ponle el ID `nuca-izquierda`, de **relleno** el color del
   pelo y, si quieres que se vea dónde empieza el pelo, una **línea** del color de los contornos.
   Haz lo mismo en el lado derecho: `nuca-derecha`.
2. En `capas`, antes de los ojos:
   ```json
   { "id": "nuca-izquierda", "tipo": "franja", "hacia": "izquierda" },
   { "id": "nuca-derecha", "tipo": "franja", "hacia": "derecha" },
   ```

De frente, como el borde está pegado al de la cabeza, casi no se ven. Al mirar a la derecha, la
línea izquierda se va con la cara y la franja cubre de pelo lo que queda detrás, con su línea
marcando dónde empieza; al mirar a la izquierda, al revés.

Por ejemplo, para una cabeza de 64 de ancho (de -32 a 32) con el centro en (0, 0):

```svg
<path id="nuca-izquierda" d="M -12 -44 C -24 -42 -31 -28 -30 -8 L -30 6 L -36 14" fill="#a6dcf0" stroke="#22201e" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
<path id="nuca-derecha" d="M 12 -44 C 24 -42 31 -28 30 -8 L 30 6 L 36 14" fill="#a6dcf0" stroke="#22201e" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
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
| «la capa «x» está dos veces» | En `capas` hay un `id` repetido: cada capa va una sola vez. |
| «la capa «x» no puede llamarse ancla-…» | Las capas no pueden empezar por `ancla-`: eso reserva el nombre para las anclas, que no van en la lista `capas`. |
| «solo puede haber una capa de tipo «cuerpo»» | Hay dos capas de tipo `cuerpo` (o dos partes llamadas `cuerpo` en el dibujo). Deja una. |
| «el cuerpo está vacío» o «el cuerpo no tiene superficie» | Lo que llamas `cuerpo` no tiene trazados o no ocupa ni de ancho ni de alto: revísalo en el dibujo. |
| «el dibujo es demasiado grande» o «demasiados puntos: simplifícalo» | El SVG pasa de 2 MB o de 30 000 puntos: simplifícalo o aplana los trazados. |
| «el dibujo no se puede leer» | El archivo no es un SVG válido. Ábrelo con Inkscape y guárdalo de nuevo. |
| «falta personaje.svg (o «imagenes» en personaje.json)» | El json no tiene `capas` ni `imagenes`, y no hay SVG. |
| «personaje.json: lleva «capas» e «imagenes»: es uno u otro» | Un personaje es de un solo tipo: quita uno de los dos. |
| «personaje.json: unknown field …» | Hay una opción mal escrita (por ejemplo, `encma` en vez de `encima`). |
| «personaje.json: expected …» o «EOF» | Al JSON le falta o le sobra algo: una coma, una comilla, una llave. Pégalo en un validador de JSON. |
| «personaje.json: «imagenes» necesita al menos una imagen en «reposo»» | La lista `reposo` está vacía o falta. |
| «demasiadas imágenes (n; como mucho 64)» | Entre todas las animaciones, hay más de 64 PNG. |
| «es demasiado grande» (un PNG) | Ese PNG pasa de 8 MB. |
| «mide ancho×alto (como mucho 1024×1024)» | Un PNG es más grande de 1024 × 1024 píxeles. |
| «tipo de color no admitido» | El PNG no es a color con transparencia (RGBA) ni en escala de grises con transparencia: expórtalo de nuevo. |
| «nombre de archivo no válido» | En `imagenes` hay un nombre con carpetas o caracteres raros. Usa letras, números, puntos, guiones, guiones bajos o espacios. |
| «todas las imágenes tienen que medir lo mismo» | Exporta todas las imágenes con el mismo tamaño de lienzo. |
| «la primera imagen del reposo es transparente» | Todo el primer PNG del reposo es transparente: dibuja el cuerpo. |
| Algo del dibujo no aparece | Si hay `capas` en el JSON, solo se dibuja lo que está en la lista. Y lo que no tiene ID (ni está en un grupo con ID) no se dibuja. |
| Un texto del dibujo no aparece | Los textos no se dibujan: en Inkscape, **Trayecto → Objeto a trayecto**. |
| Un degradado sale de un solo color | Los degradados se pintan con su primer color. |
| De frente se ve bien, pero al mirar a un lado el pelo o las orejas se despegan de la cabeza | Les falta la profundidad: mira [Que gire bien](#que-gire-bien-lo-que-sale-de-la-cabeza). |
| Al mirar a un lado, la parte de atrás de la cabeza se ve sin pelo | Añade dos franjas a los lados: mira [La parte de atrás de la cabeza](#la-parte-de-atrás-de-la-cabeza). |
| El personaje se ve pequeño | Si algo sale mucho del cuerpo (una antena muy alta), todo se encoge para que quepa en la barra al saltar. |
| Los ojos no siguen al cursor | Los personajes de imágenes miran siempre de frente. Para que mire, hazlo en SVG. |
| Las imágenes se ven borrosas | Si es pixel art, añade `"pixelado": true`. |
| No aparece en la lista y no hay error | La carpeta tiene que estar dentro de la de personajes (no más adentro) y tener un `personaje.svg` o un `personaje.json`. Las carpetas que empiezan por un punto se ignoran. |
| Un accesorio no sale en la lista | Mira el apartado **«Accesorios que no se pueden leer»** de la configuración: allí dice qué le pasa ([Accesorios propios](#accesorios-propios)). |

---

# Referencia: todas las opciones

Aquí está todo lo que el motor sabe hacer, opción por opción. Si solo quieres hacer tu personaje,
con los tutoriales de arriba te basta; esto es para afinar o para consultar mientras dibujas.

## Los archivos y las carpetas

Un personaje de dibujo vectorial es una carpeta con:

- **`personaje.svg`**: el dibujo, con cada parte con su nombre.
- **`personaje.json`**: qué es cada parte. Es opcional si el dibujo solo tiene `cuerpo`, `ojo` y
  `boca`.

Un personaje de imágenes es una carpeta con los PNG y un `personaje.json` que los describe (no lleva
SVG). Un personaje no puede ser de los dos: si el json lleva `capas` e `imagenes`, da un error.

Los accesorios propios van en su propia carpeta, dentro de la de accesorios o de la de un personaje
(ver [Accesorios propios](#accesorios-propios)).

## Cómo se lee el dibujo (SVG)

- **Cómo se nombra cada parte.** Vale el atributo `id` de un trazado, el de un grupo, o la
  **etiqueta de capa de Inkscape** (lo que se ve en el panel *Capas y objetos*). Cuando un trazado
  está dentro de grupos con nombre, cuenta primero su propio `id`, después su etiqueta y después
  los de los grupos que lo contienen, del más cercano al más lejano. Un elemento pertenece a la
  primera capa (o ancla) que coincida con alguno de esos nombres.
- **Un grupo con nombre reúne a todo lo que hay dentro.** Si agrupas varias piezas de la boca y
  llamas al grupo `boca`, todo eso es la boca.
- **Relleno y borde van por separado.** Un trazado con relleno y con línea se pinta dos veces:
  primero el relleno y encima la línea. Esto importa sobre todo en las [franjas](#franja).
- **Lo que sí se respeta:** curvas de Bézier, grupos y sus transformaciones, la opacidad de cada
  elemento (multiplicada por la de sus grupos) y la regla de relleno par-impar (`fill-rule:
  evenodd`, para formas con agujeros).
- **Lo que no:** los textos no se dibujan (conviértelos en trazados), de los degradados se usa su
  primer color y los patrones no se pintan.
- **Lo que no se ve también cuenta.** Un trazado oculto (`display: none`) o sin relleno ni borde no
  se pinta, pero sí marca su sitio: así se hacen las anclas.
- **El orden de pintado** es el del documento: lo de abajo del todo del archivo queda debajo. Cuando
  el json tiene `capas`, lo que manda es ese orden; dentro de una capa, el del documento.
- **El tamaño del lienzo da igual.** Todas las medidas se pasan a radios del cuerpo: da igual a qué
  escala dibujes o dónde pongas el origen del `viewBox`.
- **Límites:** como mucho 2 MB de archivo y 30 000 puntos en total (los puntos son los trazados ya
  convertidos en tramos rectos; cuantas más curvas y detalle, más puntos).

## Las partes del dibujo por su nombre

Estos nombres se reconocen solos, sin necesidad de `personaje.json`:

| Nombre | Qué es |
|---|---|
| `cuerpo` | La silueta. Obligatorio. Si hay varios elementos con ese nombre (o dentro de un grupo así llamado), su unión es la silueta. El color del primero de ellos es el color de base del cuerpo; los demás se pintan encima. |
| `ojo` | El ojo **derecho** (el de la derecha según lo miras), en su sitio. El izquierdo es su reflejo, salvo que haya `ojo-izquierdo`. |
| `ojo-izquierdo` | El ojo izquierdo, en su sitio, para cuando los ojos no son iguales. |
| `ojo-alegre` | El ojo derecho **cuando está contento** (al hacer clic o con música); el izquierdo sale como el reflejo del alegre. Si no lo hay, un ojo relleno se recorta solo por abajo y queda un arco «∩». |
| `boca` | La boca, en su sitio. |
| `pintado` | Todo lo que va pintado en la cara (manchas, coloretes, pecas), si quieres que sea una sola capa. |

Cualquier otro nombre es libre: lo que quieras (orejas, pelo, manchas…). Sin `personaje.json`, esas
partes libres no se dibujan; con él, se dice qué es cada una en `capas`.

## Las anclas

Las anclas marcan sitios y **no se dibujan**: hazlas como un circulito pequeño sin relleno (o un
trazado oculto) en el lugar que marcan. Su posición es el centro de lo que ocupa su forma. No se
ponen en la lista `capas`: con que existan en el dibujo vale.

| Nombre | Qué marca |
|---|---|
| `ancla-cuerpo` | El centro del cuerpo, si no quieres que sea el centro de su caja. Desde ahí se miden todas las distancias. |
| `ancla-ojo` | El centro de los ojos (por defecto, el centro del dibujo del ojo derecho). Es donde se colocan las gafas y demás accesorios de los ojos. |
| `ancla-boca` | El centro de la boca (por defecto, el centro del dibujo de la boca). |
| `ancla-muelle` | La altura desde la que el pelo con `muelle` se mueve (por defecto, un poco por debajo del borde superior de la cabeza). |
| `ancla-<capa>` | Dónde se pega a la cabeza una capa de tipo `cabeza` (por ejemplo, `ancla-oreja-izquierda` para la capa `oreja-izquierda`). Por defecto, abajo y en medio de lo que dibuja esa capa. |

Si no hay capa de ojos ni boca (por ejemplo, un personaje sin boca), las anclas `ancla-ojo` y
`ancla-boca` siguen sirviendo para colocar los accesorios. Sin nada, se usan las proporciones de
Nook: los ojos a un 27 % de la mitad del ancho del centro y la boca a un 38 % de la mitad de abajo.

## El personaje.json: nombre y colores

```json
{
  "nombre": { "es": "Gato", "en": "Cat" },
  "colores": { "cuerpo": "#3cb371", "rasgos": "#0b1230", "aviso": "#e0455a" }
}
```

- **`nombre`**: cómo sale en la lista de personajes, por idioma. Los idiomas de la interfaz son
  `es`, `en`, `ja` y `ko`; si falta el del usuario, se usa el español y, si no hay, el primero que
  haya. Sin `nombre`, sale el nombre de la carpeta.
- **`colores`** (los tres opcionales): colores del dibujo que Nookly cambia por colores que elige el
  usuario. La sustitución va por color exacto: **cada trazado del dibujo que sea exactamente de ese
  color** (en el formato `#rrggbb` o `#rgb`) pasa a llevar el color del usuario, sea del cuerpo, de
  una mancha, de una oreja o de lo que sea.
  - `cuerpo`: se cambia por el **color de acento** que elige el usuario en la configuración.
  - `rasgos`: se cambia por el color que se lee bien sobre el acento (blanco o azul oscuro). Es para
    los ojos, la boca y los contornos.
  - `aviso`: el color del circulito de las notificaciones, para cuando el usuario no elige uno él
    mismo. Si falta, se usa el color de los rasgos.

## Las capas

`capas` es la lista de las partes del dibujo **en el orden en que se pintan**: las primeras quedan
debajo. Cuando hay `capas`, **solo se dibuja lo que está en la lista** (más las anclas, que no se
dibujan). Sin `capas`, Nookly busca en el dibujo, por este orden: `cuerpo`, `pintado`, `ojo`,
`ojo-izquierdo`, `ojo-alegre` y `boca`, y usa los que haya.

Reglas de la lista:

- Cada capa es `{"id": "…"}` más sus opciones. El `id` es el nombre de la parte en el dibujo.
- No puede haber dos capas con el mismo `id`.
- Ningún `id` puede empezar por `ancla-` (eso es de las anclas, que no van en la lista).
- Tiene que haber exactamente una capa de tipo `cuerpo`.
- Cada `id` tiene que existir en el dibujo: si no, error.

El `tipo` de cada capa, si no lo dices, sale de su nombre (`cuerpo`, `ojo`, `ojo-izquierdo`,
`ojo-alegre` o `boca`) y cualquier otro nombre es de tipo `pintado`.

## Los tipos de capa

| Tipo | Qué hace |
|---|---|
| `cuerpo` | La silueta. Solo puede haber una. Todo lo que va pintado en la cara se recorta por ella, y ella manda el tamaño de todo el personaje. |
| `pintado` | Pintado en la cara (manchas, coloretes, pecas): gira con ella como si estuviera sobre una pelota y nunca se sale del cuerpo. Lo que queda al otro lado de la cabeza se desvanece poco a poco en vez de desaparecer de golpe. |
| `franja` | Una franja de color (un flequillo, una barriga, el pelo de los lados). Ver [Franja](#franja). |
| `ojo`, `ojo-izquierdo`, `ojo-alegre`, `boca` | Los rasgos: siguen al cursor, parpadean, se sorprenden y se vuelven arcos de alegría. |
| `cabeza` | Algo que sale de la cabeza (orejas, cuernos, antenas, un rulo): se mueve con ella. Ver [Que gire bien](#que-gire-bien-lo-que-sale-de-la-cabeza). |

### Franja

Una franja es una zona de color que se extiende desde una línea (su **borde**) hasta fuera del
cuerpo. Se dibuja distinto de lo que parece:

- **El borde** es el trazado de la parte, dibujado **solo con su línea** (`stroke`), como una línea
  normal, de un extremo al otro. De frente casi no se ve si está pegada al borde de la cabeza.
- **El relleno** (`fill`) es el **color de la franja**: no se dibuja la forma, se rellena desde el
  borde hasta fuera del cuerpo, hacia donde diga `hacia`, y luego se recorta por la silueta.
  Un trazado con relleno y línea hace las dos cosas: la franja y su borde marcado.
- El borde va pintado en la cara, así que se mueve con ella: al girar la cabeza, el borde se va
  hacia donde mira y la franja cubre lo que queda detrás (por eso sirven para el pelo de la nuca).
- Con `hacia` (`"arriba"` por defecto, `"abajo"`, `"izquierda"` o `"derecha"`) dices hacia dónde se
  rellena. Un borde de lado a lado se rellena hacia arriba o hacia abajo; un borde de arriba abajo
  (como el de una nuca), hacia un lado.
- Con `"encima": true`, la franja va **sobre los ojos y la boca** (un flequillo que los tapa). Por
  defecto se recorta igualmente por el cuerpo; con `"recorte"` (un número de `1` a `1.5`) puede
  salirse hasta esas veces su tamaño, agrandada desde el centro del cuerpo (el pelo del Limón va con
  `1.06` para que asome un poco por encima de la cabeza).
- Con `"muelle": true`, se mueve como el pelo (ver la opción `muelle` más abajo).

## Las opciones de cada capa

Además del `id` y el `tipo`, cada capa puede llevar estas opciones (las que no corresponden a su
tipo no hacen nada):

| Opción | Tipo | Para | Qué hace |
|---|---|---|---|
| `"muelle"` | `true`/`false` | cualquier capa | Se mueve como el pelo: se queda atrás cuando la cabeza se mueve y luego se asienta, como un mechón que se dobla. El movimiento nulo empieza en la altura de `ancla-muelle` (o un poco por debajo del borde superior de la cabeza) y crece hacia arriba. |
| `"hacia"` | `"arriba"` (por defecto), `"abajo"`, `"izquierda"` o `"derecha"` | `franja` | Hacia dónde se rellena la franja desde su borde. |
| `"encima"` | `true`/`false` | `franja` | Va por encima de los ojos y la boca (pelo que los tapa). |
| `"recorte"` | número, de `1` a `1.5` | `franja` con `encima` | Cuánto puede salirse del cuerpo: la silueta se agranda esas veces desde su centro y la franja se recorta por ella. Sin `recorte`, una franja `encima` se recorta por el cuerpo como las demás. |
| `"orientacion"` | `"cabeza"` (por defecto) o `"cuerpo"` | `cabeza` | Si se inclina con la cabeza (como un cartel pegado a ella) o solo se gira y se estira con el cuerpo (el rulo de Nook va con `"cuerpo"`). |
| `"detras"` | `true`/`false` | `cabeza` | También se dibuja detrás del cuerpo (las orejas de un lado se ven por detrás al girar). |
| `"delante"` | `"siempre"` (por defecto), `"nunca"`, `"girar"` o `"revelar"` | `cabeza` | Cuándo se ve delante del cuerpo: `"siempre"`, nunca (solo detrás), `"girar"` (pasa delante al girar la cabeza hacia su lado, como una oreja, y se esconde al girar hacia el otro) o `"revelar"` (de frente solo se ve su mitad de arriba; al mirar el personaje hacia él, asoma poco a poco hasta verse entero, como el arándano del Limón). |
| `"fundir"` | `true`/`false` | `cabeza` | Se suma a la silueta del cuerpo, como si fuera parte de él (el rulo de Nook es del mismo color y se funde con el cuerpo). |
| `"tapable"` | `true` (por defecto)/`false` | `cabeza` | Si vale `false`, los accesorios de la cabeza no lo esconden (un gorro tapa el pelo, pero no las orejas de un gato con `"tapable": false`). |
| `"profundidad"` | número | `cabeza` | Cuánto hacia delante (positivo) o hacia atrás (negativo) está su ancla, en radios del cuerpo: `0` es el plano del centro de la cabeza. Sin ella, el ancla queda en la superficie de la cara que hay delante de su punto, como pegada a la piel. |

Ejemplo con todo:

```json
{ "id": "pelo", "tipo": "cabeza", "detras": true, "delante": "nunca", "profundidad": -0.15, "muelle": true }
```

## Lo que Nookly mueve solo

No hay que animar nada: el motor mueve el personaje según lo que pasa.

- **Mira al cursor.** La cara gira hasta unos 35° a cada lado y hacia arriba y abajo. Los ojos y la
  boca se deslizan por la cara como sobre una pelota; lo de tipo `cabeza`, según su ancla y su
  profundidad.
- **Parpadea y respira.** Los ojos se cierran de vez en cuando (se estiran desde su centro) y el
  cuerpo respira muy de vez en cuando.
- **Aviso (notificación).** Se agacha, salta, cae aplastándose y rebota; abre mucho los ojos; le
  sale un circulito (la pastilla) al lado y lo mira un momento. El color de la pastilla es el que
  elija el usuario, o el `colores.aviso` del personaje, o el de los rasgos.
- **Clic.** Al hacer clic en él, da un salto más grande con rebotes y ojos de alegría: si hay
  `ojo-alegre`, se ve; si no, los ojos rellenos se recortan en arco «∩».
- **Música.** Se balancea al ritmo, inclina la cabeza, entorna los ojos con arcos de alegría y salta
  con cada golpe fuerte.
- **Accesorios.** Los accesorios se colocan solos sobre el personaje (ver
  [Accesorios propios](#accesorios-propios)); los que tapan la cabeza esconden las capas de tipo
  `cabeza` que sean `tapable`, y los que tapan ojos o boca no las dibujan.
- **El tamaño se adapta.** El radio del cuerpo es la mitad de su ancho o de su alto (lo que sea
  mayor), y el dibujo entero tiene que caber hacia arriba (1,55 radios), hacia abajo (1,05) y a los
  lados (1,35) para que quepa en la barra cuando salta: si algo sale más, todo se encoge un poco.

## El modo de imágenes: todas las opciones

```json
{
  "nombre": { "es": "Fantasma pixel", "en": "Pixel ghost" },
  "colores": { "aviso": "#e0455a" },
  "imagenes": {
    "reposo": ["reposo-1.png", "reposo-2.png"],
    "aviso": ["aviso-1.png"],
    "musica": ["baile-1.png", "baile-2.png"],
    "clic": ["salto.png"],
    "fps": 8,
    "pixelado": false,
    "ojo": [14, 10],
    "boca": [12, 16]
  }
}
```

- **`reposo`** (obligatorio, al menos una imagen), **`aviso`** (cuando llega una notificación),
  **`musica`** (cuando suena música) y **`clic`** (al hacer clic en él): cada una es la lista de
  archivos PNG de esa animación, en orden, que se repite sin parar. Si falta una animación (o su
  lista está vacía), se usan las del reposo.
- **`fps`**: a cuántas imágenes por segundo se repite cada animación. Por defecto, 8; se admite de
  0,5 a 60.
- **`pixelado`**: con `true`, las imágenes se amplían sin suavizar (pixel art nítido). Para dibujos
  normales, quítalo o déjalo en `false`.
- **`ojo`** y **`boca`**: dónde están el ojo derecho y la boca en las imágenes, **en píxeles** de las
  imágenes (contando desde la esquina de arriba a la izquierda). Sirven para colocar los accesorios:
  con `"ancla": "ojos"` o `"boca"`, el accesorio se lleva a esos puntos. Si faltan, se usan las
  proporciones de Nook (ojos a un 27 % del ancho a cada lado del centro, a la altura del centro; boca
  a un 38 % de la mitad de abajo del cuerpo).
- **El cuerpo** es lo que no es transparente en la **primera imagen del reposo**: su centro es el
  centro del personaje y su tamaño manda en todos los movimientos (saltos, estirones…).
- **Reglas de los archivos:**
  - Todos los PNG del personaje miden lo mismo, con el personaje en el mismo sitio en todos.
  - Como mucho 64 imágenes en total, de hasta 1024 × 1024 píxeles y 8 MB cada una.
  - Guardados como PNG a color con transparencia (RGBA) o en escala de grises con transparencia.
  - Nombres de archivo solo con letras, números, puntos, guiones, guiones bajos o espacios, sin
    carpetas (todo va junto en la carpeta del personaje) y sin empezar por punto.
- El personaje de imágenes **no mira al cursor ni parpadea** (no tiene cara que mover), pero salta,
  se estira, rebota al clic, baila con la música, lleva accesorios y tiene su pastilla de aviso como
  cualquier otro.

## Accesorios propios

Los accesorios (gorros, gafas, bigotes, capas…) también pueden ser tuyos: cada uno es una carpeta
con un dibujo y una descripción, y sirve para todos los personajes. Hay tres ejemplos listos para
copiar y cambiar en [`ejemplos-accesorios/`](ejemplos-accesorios): un sombrero, una aureola y un
bigote.

### Dónde van

- **Para todos los personajes:** `%APPDATA%\nook\accesorios\<carpeta>\`. La abre la configuración →
  Personaje → **Abrir carpeta de accesorios**.
- **Para un solo personaje:** dentro de su carpeta, en
  `personajes\<personaje>\accesorios\<carpeta>\`. Solo sale en la lista cuando ese personaje está
  elegido.

Como con los personajes, al guardar sus archivos se ve el cambio al momento, y si algo está mal, la
configuración lo dice en el apartado **«Accesorios que no se pueden leer»**.

### El dibujo: accesorio.svg

Hay una [plantilla con Nook](ejemplos-accesorios/plantilla-nook.svg) lista para abrir y dibujar
encima: la silueta gris y las guías se borran al terminar.

Se dibuja sobre **Nook**, que hace de plantilla: el (0, 0) del dibujo es el centro de su cuerpo, que
mide 98 de ancho (de -49 a 49) y llega a -39 por arriba y a 37,5 por abajo; los ojos están en
(±13, 0) y la boca en (0, 15). Da igual dónde pongas el `viewBox` o a qué tamaño dibujes: valen las
coordenadas del dibujo. Con otro personaje, el accesorio se coloca solo, moviéndose como diga su
`ancla`.

Vale lo mismo que con los personajes: curvas, grupos, transformaciones, opacidades y regla
par-impar; los textos hay que convertirlos en trazados; de un degradado se usa su primer color; y
como mucho 30 000 puntos. Los bordes (`stroke`) se dibujan siempre con los extremos y las uniones
redondeados, sea cual sea su `stroke-linecap` o `stroke-linejoin`.

### Los colores

**Cada color distinto del dibujo es un color que el usuario puede cambiar**, con su propia fila de
colores en la configuración. Empieza siendo el del dibujo, y siempre puede volver a él (es la
primera opción de la fila). Como mucho, **8 colores distintos**: si necesitas más, o hay uno que no
debe cambiar (el blanco de un ojo, por ejemplo), márcalo como fijo en el json.

### El accesorio.json

```json
{
  "nombre": { "es": "Sombrero", "en": "Hat" },
  "modo": "cartel",
  "ancla": "coronilla",
  "hueco": "cabeza",
  "orden": 6,
  "tapa": ["cabeza"],
  "colores": {
    "fijos": ["#ffffff"],
    "nombres": { "#e0455a": { "es": "Cinta", "en": "Band" } }
  }
}
```

| Campo | Qué dice |
|---|---|
| `nombre` | Cómo sale en la lista, por idioma (`es`, `en`, `ja`, `ko`). Sin él, el de la carpeta. |
| `modo` | **Obligatorio.** Cómo va puesto: `cara`, `cartel` o `alrededor` (ver [los tres modos](#los-tres-modos)). |
| `ancla` | A qué sitio de la plantilla se pega y con qué se mueve en otro personaje: `coronilla`, `ojos`, `boca`, `cuello`, `espalda` o `cuerpo` (ver [Las anclas de un accesorio](#las-anclas-de-un-accesorio)). Sin él, `cuerpo`. |
| `hueco` | El sitio que ocupa: `cabeza`, `ojos`, `boca`, `cuello` o `espalda`. En cada hueco solo cabe un accesorio: al poner uno, se quita el que hubiera (los que trae el programa también los ocupan: la corona, el gorro y los cuernos son de la cabeza). Sin él, no estorba a ninguno. |
| `orden` | En qué orden se dibuja, de abajo arriba. Los que trae el programa van del 0 (la capa) al 9 (los audífonos); sin él, el tuyo va el 10, encima de todos. |
| `tapa` | Lista de lo que no se dibuja mientras lo llevas puesto: `"cabeza"` (lo que sale de lo alto de la cabeza y sea `tapable`: el rulo de Nook, el arándano del Limón, lo que haya puesto el autor…), `"ojos"` o `"boca"`. |
| `colores` | `fijos`: colores del dibujo (`#rrggbb` o `#rgb`) que el usuario no puede cambiar. `nombres`: cómo se llama cada color que sí (por su color en el dibujo y por idioma); sin nombre, el color sale igual en la configuración, pero sin etiqueta. Un nombre que hable de un color que no está en el dibujo (o que sea fijo) da un error. |

### Los tres modos

- **`cara`**: pintado en la cara, como los ojos. Gira sobre ella y se desvanece al pasar al otro
  lado de la cabeza, y no se sale del cuerpo. Para gafas, antifaces, bigotes, pecas, mascarillas…
- **`cartel`**: se pega a la cabeza y siempre mira de frente: se inclina, se estira y salta con el
  cuerpo, pero no se estrecha al girar la cabeza. Para gorros, lazos, corbatines… Con el ancla
  `espalda` va detrás del cuerpo (capas, mochilas, alas).
- **`alrededor`**: rodea la cabeza, como una diadema o una aureola: lo que queda del lado que no se
  ve pasa por detrás del cuerpo. Cada forma es del lado en el que está su centro (a la izquierda o a
  la derecha de x = 0); lo que está en medio se ve siempre delante.

### Las anclas de un accesorio

El `ancla` decide a qué punto de la plantilla de Nook se pega el dibujo y cómo se mueve cuando el
personaje mide distinto:

| Ancla | Dónde se pega | Cómo se mueve en otro personaje |
|---|---|---|
| `coronilla` | Lo alto de la cabeza | Sube (o baja) lo que ese personaje mida de más (o de menos) que Nook hacia arriba. |
| `ojos` | A la altura de los ojos | Baja o sube hasta los ojos de ese personaje. |
| `boca` | A la altura de la boca | Baja o sube (y se corre a los lados) hasta la boca de ese personaje. |
| `cuello` | A la altura de la base del cuerpo | Baja o sube con la parte de abajo del cuerpo (para corbatines, bufandas). |
| `espalda` | Detrás del cuerpo | Igual que `cuello`, pero el accesorio se dibuja detrás del cuerpo (capas, mochilas, alas). |
| `cuerpo` | El centro del cuerpo | No se mueve: vale para dibujos que rodean todo el cuerpo. |

Con `modo: "cara"`, el accesorio pegado a los ojos o a la boca se desvanece al pasar al otro lado de
la cabeza, como lo pintado en ella.

### Consejos

- Empieza copiando uno de los ejemplos y cambiando lo que quieras.
- Para ver cómo queda en otros personajes, elige el Limón, uno de imágenes… y mira que sube, baja
  y se coloca bien. Si no, prueba con otra `ancla`.

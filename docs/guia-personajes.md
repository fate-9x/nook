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

1. Dibuja las dos orejas (dos triángulos), agrúpalas y ponle al grupo el ID `orejas`.
2. Dibuja un circulito pequeño donde las orejas se «enganchan» a la cabeza (arriba, en medio) y
   ponle el ID `ancla-orejas`. Quítale el relleno: las anclas no se dibujan, solo marcan un sitio.

Si guardas ahora, las orejas **no aparecen**: falta decirle a Nookly qué son. Eso va en el paso 7.

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
    { "id": "orejas", "tipo": "cabeza", "detras": true, "delante": "girar" },
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
  - `orejas` es de tipo **`cabeza`**: se mueve con la cabeza. Con `"detras": true` se ve también
    por detrás del cuerpo, y con `"delante": "girar"` pasa por delante cuando el gato gira la cabeza
    hacia su lado. Por eso va la primera: debajo de todo.
  - `cuerpo`, `ojo` y `boca` no necesitan tipo: lo saben por su nombre.
  - `rayas` no dice tipo, así que es **pintado** (pintado en la cara).

Guarda. Las orejas aparecen, y el nombre en la lista pasa a ser «Gato».

> Cuando hay un `personaje.json` con `capas`, **solo se dibuja lo que está en la lista**. Si añades
> algo al dibujo y no sale, revisa que esté aquí.

### Paso 8. Pruébalo

- Mueve el cursor alrededor del gato: gira la cabeza, y las orejas pasan de detrás a delante.
- En la configuración, **Probar aviso**: salta y le sale el circulito rojo.
- Pon música: baila.
- Ponle accesorios (gafas, gorro…): se colocan sobre su cara y su cabeza.

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
| El personaje se ve pequeño | Si algo sale mucho del cuerpo (una antena muy alta), todo se encoge para que quepa en la barra al saltar. |
| Los ojos no siguen al cursor | Los personajes de imágenes miran siempre de frente. Para que mire, hazlo en SVG. |
| «todas las imágenes tienen que medir lo mismo» | Exporta todas las imágenes con el mismo tamaño de lienzo. |
| Las imágenes se ven borrosas | Si es pixel art, añade `"pixelado": true`. |
| No aparece en la lista y no hay error | La carpeta tiene que estar dentro de la de personajes (no más adentro) y tener un `personaje.svg` o un `personaje.json`. Las carpetas que empiezan por un punto se ignoran. |

# Hacer tu propio personaje

El personaje de la barra puede ser cualquiera: lo dibujas tú y Nookly lo anima (mira al cursor,
parpadea, salta con los avisos, baila con la música y lleva los accesorios). Nook y el Limón están
hechos así: sus archivos, en [`personajes/`](../personajes), sirven de ejemplo.

## Empezar

En la configuración, sección **Personaje**, pulsa **Nuevo personaje**: se crea una copia de Nook en
una carpeta nueva, se elige y se abre la carpeta. Cambia sus archivos con cualquier editor (el
dibujo, con Inkscape, Figma, Illustrator…) y al guardar se ve el cambio en la barra al momento.

Cada personaje es una carpeta dentro de `%APPDATA%\nook\personajes\` (**Abrir carpeta** la abre).
Para compartirlo, comparte su carpeta: quien la copie en la suya lo tendrá en la lista.

Si algo está mal, la configuración dice por qué (encima de los accesorios). Mientras lo arreglas,
se queda la última versión que estaba bien; si nunca lo estuvo, no sale en la lista.

Hay dos maneras de hacer un personaje:

- **Dibujo vectorial** (`personaje.svg`): el más vivo. Sus ojos miran al cursor y parpadean, la cara
  gira sobre el cuerpo y lo que sale de la cabeza se mueve con ella.
- **Imágenes** (unos PNG): cualquier dibujo, pixel art o lo que quieras, con una animación para cada
  momento. Salta, se estira y se mueve como los demás, pero mira siempre de frente.

## Dibujo vectorial

La carpeta tiene:

- `personaje.svg`: el dibujo, con cada parte con su nombre.
- `personaje.json`: qué es cada parte (opcional si solo tiene cuerpo, ojo y boca).

### Las partes del dibujo

Cada parte se reconoce por su `id` o por el nombre de su capa en Inkscape. Puede ser un trazado o
un grupo con varios dentro. Lo que no tenga uno de los nombres de las capas no se dibuja.

| Nombre | Qué es |
|---|---|
| `cuerpo` | La silueta. Obligatorio. Su tamaño es el que manda: da igual a qué escala dibujes. |
| `ojo` | El ojo **derecho** (el de la derecha según lo miras), en su sitio. El izquierdo es su reflejo. |
| `ojo-izquierdo` | Si los ojos no son iguales: el izquierdo, en su sitio. |
| `ojo-alegre` | El ojo derecho cuando está contento. Si no hay, un ojo relleno se recorta en arco «∩». |
| `boca` | La boca, en su sitio. |
| Cualquier otro | Lo que digas en `personaje.json` (ver abajo). |

Además, las **anclas** marcan sitios y no se dibujan (pueden ser un círculo pequeño sin relleno):

| Nombre | Qué marca |
|---|---|
| `ancla-cuerpo` | El centro del cuerpo, si no es el centro de su silueta. |
| `ancla-ojo`, `ancla-boca` | El centro del ojo y de la boca, si no es el de su dibujo. |
| `ancla-<capa>` | Dónde se pega a la cabeza una capa de tipo `cabeza` (por defecto, abajo y en medio de lo que dibuja). |
| `ancla-muelle` | La altura desde la que el pelo se mueve con muelle (ver `muelle`). |

Los colores son los del dibujo. Las curvas, grupos, transformaciones y opacidades funcionan; los
textos no (conviértelos en trazados) y de los degradados se usa su primer color.

### personaje.json

```json
{
  "nombre": { "es": "Gato", "en": "Cat" },
  "colores": { "cuerpo": "#3cb371", "rasgos": "#0b1230", "aviso": "#e0455a" },
  "capas": [
    { "id": "cuerpo" },
    { "id": "manchas" },
    { "id": "orejas", "tipo": "cabeza", "detras": true, "delante": "girar" },
    { "id": "ojo" },
    { "id": "boca" }
  ]
}
```

- `nombre`: cómo sale en la lista, por idioma (`es`, `en`, `ja`, `ko`). Sin él, el de la carpeta.
- `colores` (todo opcional):
  - `cuerpo`: el color del dibujo que se cambia por el **color de acento** del usuario.
  - `rasgos`: el que se cambia por el color que se lee sobre el acento (blanco o azul oscuro).
  - `aviso`: el del circulito de las notificaciones, si el usuario no elige otro.
- `capas`: las partes del dibujo, **en el orden en que se pintan**. Sin `capas`, se usan `cuerpo`,
  `pintado`, `ojo`, `ojo-izquierdo`, `ojo-alegre` y `boca`, las que haya.

Cada capa tiene su `id` (el nombre de la parte en el dibujo) y su `tipo`. El tipo, si no se dice,
sale del nombre (`cuerpo`, `ojo`, `ojo-izquierdo`, `ojo-alegre`, `boca`) o es `pintado`:

| Tipo | Qué hace |
|---|---|
| `cuerpo` | La silueta. Solo puede haber una. |
| `pintado` | Pintado en la cara (manchas, coloretes, pecas): gira con ella y no se sale del cuerpo. |
| `franja` | Una franja de color de lado a lado (un flequillo, una barriga). Se dibuja **solo su borde**, como una línea de izquierda a derecha, y se rellena hasta fuera del cuerpo hacia `hacia`. |
| `ojo`, `ojo-izquierdo`, `ojo-alegre`, `boca` | Los rasgos: miran al cursor, se sorprenden, parpadean. |
| `cabeza` | Algo que sale de la cabeza (orejas, cuernos, antenas, un rulo): se mueve con ella. |

Opciones de las capas:

| Opción | Para | Qué hace |
|---|---|---|
| `"hacia": "arriba"` o `"abajo"` | `franja` | Hacia dónde se rellena desde su borde (por defecto, arriba). |
| `"encima": true` | `franja` | Va por encima de los ojos y la boca (pelo que los tapa). |
| `"recorte": 1.06` | `franja` con `encima` | Puede salirse del cuerpo hasta esas veces su tamaño (de 1 a 1,5). |
| `"muelle": true` | cualquiera | Se mueve como el pelo: se queda atrás cuando la cabeza se mueve y luego se asienta. |
| `"orientacion": "cabeza"` o `"cuerpo"` | `cabeza` | Si se inclina con la cabeza (por defecto) o solo se gira y se estira con el cuerpo. |
| `"detras": true` | `cabeza` | También se dibuja detrás del cuerpo. |
| `"delante"` | `cabeza` | Cuándo se ve delante: `"siempre"` (por defecto), `"nunca"`, `"girar"` (al girar la cabeza hacia su lado, como las orejas) o `"revelar"` (de frente se ve su mitad de arriba; mirándolo, entero). |
| `"fundir": true` | `cabeza` | Se suma al cuerpo, como si fuera parte de él (el rulo de Nook). |
| `"tapable": false` | `cabeza` | Que un gorro no lo esconda (por defecto, los accesorios de la cabeza lo tapan). |
| `"profundidad": 0.2` | `cabeza` | Cuánto sale hacia delante su ancla (en radios del cuerpo), si no es la del punto de la cabeza que tiene detrás. |

### Consejos

- Dibuja de frente y en reposo: Nookly se encarga de moverlo.
- El cuerpo puede tener cualquier forma. Lo que sale de él (orejas, antenas) mejor como capas de tipo
  `cabeza`, que se mueven con la cabeza.
- Si el personaje es muy alto o tiene algo que sale mucho, se hace más pequeño para que quepa en la
  barra al saltar.
- Mira cómo están hechos [Nook](../personajes/nook) (un rulo que se funde con el cuerpo) y
  [el Limón](../personajes/limon) (franjas, pelo con muelle y un arándano que asoma).

## Imágenes

La carpeta tiene los PNG y un `personaje.json` que dice cuáles son:

```json
{
  "nombre": { "es": "Pixel" },
  "imagenes": {
    "reposo": ["reposo-1.png", "reposo-2.png"],
    "aviso": ["aviso-1.png", "aviso-2.png", "aviso-3.png"],
    "musica": ["baile-1.png", "baile-2.png"],
    "clic": ["salto.png"],
    "fps": 8
  }
}
```

- `reposo` es obligatorio; `aviso` (cuando llega una notificación), `musica` (cuando suena música) y
  `clic` (al hacer clic en él) son opcionales: si faltan, se usa el reposo.
- Cada animación se repite a `fps` imágenes por segundo (por defecto, 8).
- Con `"pixelado": true` (para pixel art), las imágenes se amplían sin suavizar: los píxeles se ven
  nítidos.
- Todas las imágenes miden lo mismo y el personaje está en el mismo sitio en todas. Lo que no es
  transparente en la primera del reposo es su cuerpo: su tamaño es el que manda.
- Para los accesorios, `"ojo": [x, y]` y `"boca": [x, y]` (en píxeles de las imágenes) dicen dónde
  están el ojo derecho y la boca.
- Como mucho, 64 imágenes de hasta 1024 × 1024.

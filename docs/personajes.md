# Hacer tu propio personaje

El personaje de la barra puede ser cualquiera: lo dibujas tú y Nookly lo anima (mira al cursor,
parpadea, salta con los avisos, baila con la música y lleva los accesorios). Nook y el Limón están
hechos así: sus archivos, en [`personajes/`](../personajes), sirven de ejemplo. Los accesorios
también pueden ser tuyos: ver [Accesorios propios](#accesorios-propios).

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
| `franja` | Una franja de color de lado a lado (un flequillo, una barriga) o de arriba abajo (el pelo de los lados de la cabeza). Se dibuja **solo su borde**, como una línea, y se rellena hasta fuera del cuerpo hacia `hacia`. |
| `ojo`, `ojo-izquierdo`, `ojo-alegre`, `boca` | Los rasgos: miran al cursor, se sorprenden, parpadean. |
| `cabeza` | Algo que sale de la cabeza (orejas, cuernos, antenas, un rulo): se mueve con ella. |

Opciones de las capas:

| Opción | Para | Qué hace |
|---|---|---|
| `"hacia"` | `franja` | Hacia dónde se rellena desde su borde: `"arriba"` (por defecto) o `"abajo"`, con el borde de lado a lado, o `"izquierda"` o `"derecha"`, con el borde de arriba abajo. Una franja de un lado con el borde pegado al de la cabeza casi no se ve de frente, y al girar la cabeza cubre su parte de atrás (ver [la guía](guia-personajes.md#la-parte-de-atrás-de-la-cabeza)). |
| `"encima": true` | `franja` | Va por encima de los ojos y la boca (pelo que los tapa). |
| `"recorte": 1.06` | `franja` con `encima` | Puede salirse del cuerpo hasta esas veces su tamaño (de 1 a 1,5). |
| `"muelle": true` | cualquiera | Se mueve como el pelo: se queda atrás cuando la cabeza se mueve y luego se asienta. |
| `"orientacion": "cabeza"` o `"cuerpo"` | `cabeza` | Si se inclina con la cabeza (por defecto) o solo se gira y se estira con el cuerpo. |
| `"detras": true` | `cabeza` | También se dibuja detrás del cuerpo. |
| `"delante"` | `cabeza` | Cuándo se ve delante: `"siempre"` (por defecto), `"nunca"`, `"girar"` (al girar la cabeza hacia su lado, como las orejas) o `"revelar"` (de frente se ve su mitad de arriba; mirándolo, entero). |
| `"fundir": true` | `cabeza` | Se suma al cuerpo, como si fuera parte de él (el rulo de Nook). |
| `"tapable": false` | `cabeza` | Que un gorro no lo esconda (por defecto, los accesorios de la cabeza lo tapan). |
| `"profundidad": -0.4` | `cabeza` | Cuánto hacia delante (positivo) o hacia atrás (negativo) está su ancla, en radios del cuerpo: `0` es el plano del centro de la cabeza. Sin ella, el ancla está en la superficie de la cara, por delante. El pelo que rodea la cabeza va con una pequeña (de `0` a `-0.15`), lo que está en la nuca (una coleta, un moño) con una negativa (de `-0.3` a `-0.5`), y las orejas, una por lado, de `0` a `-0.2` (ver [la guía](guia-personajes.md#que-gire-bien-lo-que-sale-de-la-cabeza)). |

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

## Accesorios propios

Los accesorios (gorros, gafas, bigotes, capas…) también pueden ser tuyos: cada uno es una carpeta
con un dibujo y una descripción, que sirve para cualquier personaje. En la configuración, sección
**Personaje**, **Abrir carpeta de accesorios** abre `%APPDATA%\nook\accesorios\`: copia ahí la
carpeta de uno y sale en la lista de accesorios, con todos los que trae el programa. Como con los
personajes, al guardar sus archivos se ve el cambio al momento, y si algo está mal la
configuración dice qué.

Tres ejemplos listos, para copiar y cambiar, en [`ejemplos-accesorios/`](ejemplos-accesorios): un
sombrero, una aureola y un bigote.

Uno que es solo de un personaje va en la carpeta de este, en `personajes\<carpeta>\accesorios\<carpeta>\`,
y solo sale en la lista cuando ese personaje está elegido.

### El dibujo: accesorio.svg

Se dibuja sobre **Nook**: el (0, 0) del dibujo (da igual dónde pongas el viewBox) es el centro de
su cuerpo, que mide 98 de ancho (de -49 a 49) y llega a -39 por arriba y a 37,5 por abajo; la boca
está en (0, 15) y los ojos en (±13, 0). Con otro personaje, el accesorio se coloca solo: se mueve
con él como diga su `ancla` (ver abajo).

Lo que se pinta son los trazados con relleno o con borde (con sus curvas, grupos, transformaciones
y opacidades; los textos hay que convertirlos en trazados, y de un degradado se usa su primer
color). Como mucho, 30 000 puntos.

### Los colores

**Cada color distinto del dibujo es un color que el usuario puede cambiar**, con su propia fila de
colores en la configuración. Empieza siendo el del dibujo, y siempre puede volver a él (es la primera
opción de la fila). Como mucho, 8 colores distintos: si necesitas más, o hay uno que no debe cambiar
(el blanco de un ojo, por ejemplo), márcalo como fijo en el json.

### accesorio.json

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
| `modo` | **Obligatorio.** Cómo va puesto: `cara`, `cartel` o `alrededor` (ver abajo). |
| `ancla` | A qué sitio de Nook se pega y con qué se mueve en otro personaje: `coronilla` (sube lo que sea más alto que Nook), `ojos` y `boca` (bajan o suben con los de ese personaje), `cuello` y `espalda` (con la parte de abajo del cuerpo) o `cuerpo` (no se mueve; es lo que hay si no dices otra). |
| `hueco` | El sitio que ocupa: `cabeza`, `ojos`, `boca`, `cuello` o `espalda`. En cada hueco solo cabe un accesorio: al poner uno, se quita el que hubiera (los que trae el programa también los tienen: la corona, el gorro y los cuernos son de la cabeza). Sin él, no estorba a ninguno. |
| `orden` | En qué orden se dibuja, de abajo arriba. Los que trae el programa van del 0 (la capa) al 9 (los audífonos); sin él, el tuyo va el 10, encima de todos. |
| `tapa` | Lo que no se dibuja mientras lo llevas puesto: `cabeza` (lo que sale de lo alto de la cabeza: el rulo de Nook, el arándano del Limón, lo que haya puesto el autor…), `ojos` o `boca`. |
| `colores` | `fijos`: colores del dibujo que no se pueden cambiar. `nombres`: cómo se llama cada color que sí (por su color en el dibujo y por idioma); sin nombre, solo salen las filas. |

### Los tres modos

- **`cara`**: pintado en la cara, como los ojos. Gira sobre ella y se desvanece al pasar al otro
  lado de la cabeza, y no se sale del cuerpo. Para gafas, antifaces, bigotes, pecas, mascarillas…
- **`cartel`**: se pega a la cabeza y siempre mira de frente: se inclina, se estira y salta con el
  cuerpo, pero no se estrecha al girar la cabeza. Para gorros, lazos, corbatines… Con el ancla
  `espalda` va detrás del cuerpo (capas, mochilas, alas).
- **`alrededor`**: rodea la cabeza, como una diadema o una aureola: lo que queda del lado que no se
  ve pasa por detrás del cuerpo. Cada forma es del lado en el que está su centro (a la izquierda o a
  la derecha de x = 0); lo que está en medio se ve siempre delante.

### Consejos

- Empieza copiando uno de los ejemplos y cambiando lo que quieras.
- Para ver cómo queda en otros personajes, elige el Limón, uno de imágenes… y mira que sube, baja
  y se coloca bien. Si no, prueba con otra `ancla`.
- Los bordes (`stroke`) se dibujan siempre con los extremos y las uniones redondeados.

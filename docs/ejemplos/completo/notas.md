El más complejo: un chico gato con orejas, pelo en punta alrededor de la cabeza, flequillo, nuca,
cejas y un ojo de cada color. Junta `animal` y `pelo`; si tu personaje tiene varias de estas cosas,
parte de aquí y quita lo que sobre.

El orden de `capas` es lo que más importa:

1. `pelo-atras` (`cabeza`, `"detras": true, "delante": "nunca"`, `"profundidad": -0.12`): los
   pinchos de alrededor, una forma cerrada que rodea la cabeza y solo se ve por detrás.
2. `oreja-izquierda`, `oreja-derecha` (`cabeza`, `"detras": true, "delante": "girar"`): después del
   pelo de atrás para quedar encima de él, con un contorno (`stroke`) que las separa del pelo.
3. `cuerpo`.
4. `nuca-izquierda`, `nuca-derecha` (`franja` hacia los lados): la parte de atrás de la cabeza.
5. Lo pintado en la cara: `mejillas`, `cejas` (las cejas, una uniceja, cicatrices, pecas…).
6. Los rasgos: `ojo`, `ojo-izquierdo`, `ojo-alegre`, `boca`.
7. `flequillo` (`franja` hacia arriba con `"encima": true`): lo último, encima de todo.

**Ojos distintos:** `ojo` es el derecho (x positiva) y `ojo-izquierdo` el izquierdo, cada uno
dibujado en su sitio. Sirve para un ojo de cada color, uno guiñado, uno más cerrado o un parche:
dibuja en `ojo-izquierdo` lo que tenga ese ojo. Los dos parpadean. `ojo-alegre` es el ojo derecho
contento; el izquierdo contento es su reflejo.

**Ojos con párpado** (cansados, enfadados): en el grupo del ojo, encima del blanco, un trozo del
color de la piel que tape la parte de arriba (por ejemplo, un arco `A` cerrado con `Z`), con su
línea.

**Boca con varias piezas** (un colmillo, la lengua, babas): un grupo `boca` con todas.

Si las orejas no se ven, es que el pelo las tapa: súbelas o ponlas después del pelo en `capas`.

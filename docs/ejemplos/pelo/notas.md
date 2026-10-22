Una persona con pelo: flequillo, pelo alrededor de la cabeza, nuca y moño. La cabeza es el cuerpo.

El pelo tiene cuatro piezas, cada una con su forma de moverse:

| Pieza | Capa | Por qué |
|---|---|---|
| Masa de pelo que asoma alrededor de la cabeza | `pelo-atras`: `cabeza`, `"detras": true, "delante": "nunca"`, `"profundidad"` de `0` a `-0.15` | Va solo por detrás del cuerpo: forma cerrada más grande que la cabeza. |
| Moño, coleta o trenza en la nuca | `mono`: `cabeza`, igual pero `"profundidad"` de `-0.3` a `-0.5`, con `"muelle": true` | Al girar se va hacia el otro lado, como lo de atrás de una cabeza. |
| Pelo de los lados (la parte de atrás que se ve al girar) | `nuca-izquierda` / `nuca-derecha`: `franja` con `"hacia": "izquierda"` / `"derecha"` | Su borde es una línea **abierta** pegada por dentro al borde de la cabeza, de arriba a la altura de la oreja, y al final sale hacia fuera del cuerpo. |
| Flequillo | `flequillo`: `franja`, `"hacia": "arriba"`, `"encima": true`, `"recorte": 1.06`, `"muelle": true` | Su borde es una línea abierta que cruza la frente de lado a lado (de x = -50 a 50) con la forma de las puntas; se rellena hacia arriba y tapa lo que haya debajo. |

- `ojo-alegre`: un arco, para cuando está contento (los ojos con brillo quedan mejor así).
- Los ojos y la boca, por debajo del borde del flequillo para que no los tape.

Variantes:
- **Pelo largo:** alarga `pelo-atras` hacia abajo por los lados (hasta y = 40 o más) y baja el final
  de las nucas.
- **Calvo por arriba:** sin flequillo; las nucas y `pelo-atras` solo en los lados.
- **Pelo en punta:** `pelo-atras` con pinchos (mira `completo`).
- **Coleta de lado:** como el moño, pero con el ancla a un lado de la nuca.
- **Estilo con contorno:** pon `stroke` oscuro a cada pieza (también al cuerpo y a las franjas: en
  una franja, el `stroke` se dibuja a lo largo de su borde).

Un animal con orejas a los lados (zorro) y la parte de abajo de la cara de otro color.

- `oreja-izquierda` y `oreja-derecha`: **una capa por lado**, tipo `cabeza`, cada una con su ancla
  (`ancla-oreja-…`) en la base, donde toca la cabeza. Con `"detras": true, "delante": "girar"`, al
  mirar hacia un lado la oreja de ese lado pasa delante y la otra se esconde detrás.
  `"profundidad": -0.1` las pone en los lados de la cabeza. `"tapable": false`: un gorro no las
  esconde. Van las primeras en `capas` (debajo de todo). Cada oreja es un grupo: el exterior y el
  interior de otro color.
- `hocico`: tipo `franja` con `"hacia": "abajo"`. Su trazado es **solo una línea abierta** de lado a
  lado (más larga que el cuerpo, de x = -56 a 56): todo lo que queda por debajo de ella dentro del
  cuerpo se pinta de su `fill`. Para una barriga o un pecho, lo mismo más abajo.
- `nariz`: pintado. `ojo` y `boca` como siempre.

Variantes:
- **Gato:** orejas más cortas y rayas pintadas en la frente.
- **Conejo:** orejas largas y estrechas hacia arriba, con el ancla en su base.
- **Oso o ratón:** orejas redondas (`circle`) arriba a los lados.
- **Perro de orejas caídas:** las orejas cuelgan por los lados de la cabeza; ancla arriba, en la
  base, y `"profundidad": -0.15`.
- **Cuernos o alas:** igual que las orejas, una capa por lado.

Una fresa: una silueta con forma, hojas encima y un rabito que asoma al girar.

- `cuerpo`: la forma de la fruta (más ancha arriba). Lo que no es redondo también vale.
- `semillas` y `brillo`: pintados en la cara.
- `hojas`: tipo `cabeza` con `"muelle": true`, en lo alto, con el ancla abajo en medio. Tienen que
  quedarse en la mitad central del ancho del cuerpo (aquí, de x = -23 a 23): si se abren más, la
  comprobación avisa de que parecen orejas.
- `rabito`: tipo `cabeza` con `"detras": true, "delante": "revelar"` y `"profundidad": 0.2`: de
  frente solo se ve su mitad de arriba y, al mirar hacia él, asoma entero. Va el primero en `capas`.

Variantes:
- **Limón, naranja, manzana:** cambia la silueta y pon una hoja con `revelar` como el rabito.
- **Aguacate:** el hueso, pintado en la barriga; o una `franja` con `"hacia": "abajo"` para la
  parte de abajo de otro color.
- **Taza, nube, casa:** la silueta del objeto como cuerpo y los detalles pintados; lo que sobresalga
  (el asa, una chimenea), tipo `cabeza`.

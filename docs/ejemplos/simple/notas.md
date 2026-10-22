Una mascota redonda sin nada que sobresalga del cuerpo: todo va pintado en la cara.

- `cuerpo`: la silueta. Puede ser cualquier forma cerrada (aquí, una bola con la base plana).
- `brillo` y `mejillas`: tipo `pintado` (el de por defecto, sin poner `tipo`). Giran con la cara y
  se recortan por la silueta.
- `ojo`: un grupo con la pupila y su brillo blanco, **solo el derecho**. Como es relleno, al estar
  contento se recorta solo en arco «∩» (no hace falta `ojo-alegre`).
- `colores.cuerpo` y `colores.rasgos`: el azul del cuerpo pasa a ser el color de acento del usuario
  y el azul oscuro de ojos y boca el que se lee sobre él. Quítalos para dejar los colores fijos.

Para adaptarlo: cambia la silueta y los colores, añade manchas o pecas como otro grupo pintado (y
ponlo en `capas`). Si algo tiene que sobresalir (orejas, antena, pelo), mira `animal`, `antena` o
`pelo`.

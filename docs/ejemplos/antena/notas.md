Un robot con una antena que se balancea y un cuerpo cuadrado.

- `cuerpo`: un cuadrado redondeado. La silueta puede tener cualquier forma.
- `antena`: tipo `cabeza` con `"muelle": true` (se balancea al moverse) y `"tapable": false` (un
  gorro no la esconde). Su ancla, `ancla-antena`, en la base, justo sobre el borde de arriba.
- `pantalla` y `tornillos`: pintados en la cara, giran con ella.
- `ojo` es un `rect` redondeado; `ojo-alegre`, una «^». Los ojos claros sobre la pantalla oscura:
  por eso aquí **no** hay `colores.rasgos` (cambiaría los ojos por un color que no se lee sobre la
  pantalla). `colores.cuerpo` sí: el gris pasa a ser el color de acento del usuario.

Variantes:
- **Insecto o alien:** dos antenas, una por lado, cada una su capa con su ancla y `"muelle": true`.
- **Algo pegado a la frente** (un rulo, un cuerno de unicornio): tipo `cabeza` con el ancla en su
  base; si es del color del cuerpo y quieres que parezca parte de él, `"fundir": true` y
  `"orientacion": "cuerpo"`.
- **Sombrero fijo o casco:** tipo `cabeza` sin muelle.

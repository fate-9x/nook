Un personaje de imágenes PNG (un fantasma de pixel art de 24 × 24): no lleva SVG, sino unos PNG y
este `personaje.json`. Necesitas poder crear archivos PNG (por ejemplo, con un programa o un script);
si no puedes, haz el personaje en SVG con otro tipo de ejemplo.

- **No mira al cursor ni parpadea solo**: salta, se estira, rebota y baila igual. Para que parpadee,
  repite la imagen de ojos abiertos y pon una con los ojos cerrados (como `reposo`, abajo).
- `reposo` (obligatoria), `aviso`, `musica` y `clic`: la lista de PNG de cada momento, que se repite
  a `fps` imágenes por segundo (de 0,5 a 60). Las que faltan usan el reposo.
- `"pixelado": true`: amplía sin suavizar, para pixel art. Quítalo para dibujos normales.
- Opcional: `"ojo": [x, y]` y `"boca": [x, y]`, en píxeles de la imagen, para colocar los accesorios.

Reglas de los PNG: fondo transparente; todos del mismo tamaño y con el personaje en el mismo sitio;
RGBA o gris con transparencia; hasta 1024 × 1024 píxeles, 8 MB cada uno y 64 en total; nombres sin
carpetas. Lo que no es transparente en el primer PNG del reposo es el cuerpo.

Los PNG van en la carpeta del personaje junto al `personaje.json`. `personaje__guardar` solo guarda
el json: escribe tú los PNG en la carpeta que te dice.

Este ejemplo usa `reposo-1.png` (ojos abiertos), `reposo-2.png` (cerrados) y `aviso-1.png` (de
otro color).

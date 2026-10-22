Un personaje de dibujos animados con contorno: cabeza alargada, calvo por arriba, pelo alborotado a
los lados, orejas, cejas, párpados caídos, nariz grande y bigote. Para personas y personajes de
series con un estilo marcado.

**Lo primero, las proporciones.** Si copias un personaje, mide su cabeza en la referencia y dibuja
el `cuerpo` con esa forma (aquí, 66 × 90: más alta que ancha y más estrecha abajo). Es lo que más
hace que se parezca; una cabeza redonda lo convierte en otro.

- **Contorno:** todas las piezas llevan el mismo `stroke` oscuro, también el `cuerpo` (su trazo se
  dibuja fijo sobre la silueta) y las franjas (su trazo se dibuja a lo largo del borde).
- `pelo`: `cabeza` con `"detras": true, "delante": "nunca"` y `"muelle": true`. Mechones distintos
  en cada lado y que bajan hasta la mandíbula. La parte de arriba de la forma queda dentro de la
  cabeza: de frente no se ve, y así es calvo. Para pelo por arriba, saca puntas por encima.
- `oreja-izquierda`/`oreja-derecha`: medio óvalo pegado al borde de la cabeza, a la altura de los
  ojos, cada una con su ancla en la base. Van **después** del pelo para quedar encima de él.
- `nuca-izquierda`/`nuca-derecha`: el pelo de los lados que se ve al girar (como en `pelo`).
- `cejas`: dos elementos sueltos dentro del grupo. Lo pintado se desvanece por **elementos**, según
  dónde quede el centro de cada uno: así cada ceja desaparece sola al pasar al otro lado. Una
  uniceja, en cambio, va en un solo elemento: su centro está en medio y siempre se ve.
- `ojo` con párpado: el blanco, encima un trozo del color de la piel que tapa la parte de arriba
  (un arco `A` cerrado con `Z`) y la pupila. Con párpado **hace falta `ojo-alegre`**: sin él, el
  motor recorta el ojo en arco y se ve raro.
- `nariz` y `bigote`: pintados, un solo elemento centrado cada uno, para que no desaparezcan al
  girar. Van antes de la `boca` para que la boca quede encima.

No pongas nada pintado cerca del borde de arriba de la cabeza (arrugas en lo alto de la frente,
por ejemplo): al mirar arriba se aplasta contra el borde y se ve como una mancha oscura.

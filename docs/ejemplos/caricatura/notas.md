Un personaje de dibujos animados con contorno: cabeza alargada, calvo por arriba, pelo alborotado a
los lados, orejas, cejas, párpados caídos, nariz grande, bigote y dos expresiones (Enojado y
Sorprendido). Para personas y personajes de series con un estilo marcado.

**Lo primero, las proporciones.** Si copias un personaje, mide su cabeza en la referencia y dibuja
el `cuerpo` con esa forma (aquí, 66 × 90: más alta que ancha y más estrecha abajo). Es lo que más
hace que se parezca; una cabeza redonda lo convierte en otro.

- **Contorno:** todas las piezas llevan el mismo `stroke` oscuro, también el `cuerpo` (su trazo se
  dibuja fijo sobre la silueta) y las franjas (su trazo se dibuja a lo largo del borde).
- `pelo`: `cabeza` con `"detras": true, "delante": "nunca"` y `"muelle": true`. Mechones distintos
  en cada lado y que bajan hasta la mandíbula. La parte de arriba de la forma queda dentro de la
  cabeza: de frente no se ve, y así es calvo. Para pelo por arriba, saca puntas por encima.
- `nuca-izquierda`/`nuca-derecha`: el pelo de los lados que se ve al girar (como en `pelo`).
- `oreja`: solo la derecha, a la altura de los ojos; el motor hace la izquierda como su reflejo, la
  funde con la cabeza y la esconde al girar. El relleno entra un poco en la cabeza y la línea de
  contorno va solo por fuera. En `capas`, **después** de las franjas de la nuca (si no, la tapan).
- `cejas`: dos elementos sueltos dentro del grupo. Lo pintado se desvanece por **elementos**, según
  dónde quede el centro de cada uno: así cada ceja desaparece sola al pasar al otro lado. Una
  uniceja, en cambio, va en un solo elemento: su centro está en medio y siempre se ve.
- `ojo`: el blanco con su contorno y la pupila (`id="pupila"`: se mueve dentro del ojo hacia donde
  mira). Los párpados los dibuja el motor con `"parpado": 0.4` en el json (tapa el 40 % de arriba
  del ojo, con su línea, y parpadea bajándolos). El rectángulo `ancla-pupila` limita por dónde va
  el centro de la pupila: no sube del párpado. Con párpados **hace falta `ojo-alegre`**.
- `nariz` y `bigote`: pintados, un solo elemento centrado cada uno, para que no desaparezcan al
  girar. Van antes de la `boca` para que la boca quede encima.

**Barbilla:** `"menton": 0.4` en el json la saca hacia delante: al girar la cabeza, la barbilla y la
mandíbula se van hacia donde mira, con la boca y el bigote. Sin él, el contorno no cambia al girar.

**Expresiones** (en el json, `expresiones`): Enojado cambia las cejas por `cejas-enojado` y baja
los párpados (`"parpado": 0.55`); Sorprendido cambia las cejas y la boca y casi los abre (`0.1`).
Las piezas están al final del SVG, fuera de las capas, dibujadas en su sitio.

No pongas nada pintado cerca del borde de arriba de la cabeza (arrugas en lo alto de la frente,
por ejemplo): al mirar arriba se aplasta contra el borde y se ve como una mancha oscura.

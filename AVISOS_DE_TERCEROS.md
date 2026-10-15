# Avisos de terceros

Nook usa trabajo de otras personas. Aquí se indica qué se usa de cada proyecto, quién lo hizo y
bajo qué licencia. Este mismo texto va dentro del programa: Configuración → Créditos → Ver
licencias.

## bloub

- Proyecto: https://github.com/jeremy-prt/bloub
- Autor: Jérémy Perret
- Licencia: MIT (texto completo más abajo)
- **Qué se usa:** la **técnica de dibujo y animación** del personaje, pasada de TypeScript a Rust
  en `src/personaje/`:
  - la silueta como perfil radial (el radio medido en 64 direcciones fijas), que se interpola
    para cambiar de forma, y su contorno suave con curvas de Catmull-Rom;
  - los ojos pintados sobre una esfera: la orientación de la cabeza (giro, cabeceo e
    inclinación), el plano tangente de cada ojo y su proyección, que hace que el ojo cercano al
    borde se estreche;
  - la vida en reposo como función pura del tiempo: la deriva de la mirada con ruido periódico,
    el calendario de parpadeos generado de antemano y la respiración;
  - la mirada guiada desde fuera (absoluta, mezclada con la de reposo) y las transiciones con
    curvas que frenan sin pasarse.
- **Qué no se usa:** el personaje animado de la barra no es el de bloub (ni su forma, ni sus ojos
  inclinados, ni sus colores, que recrean el avatar de x.ai): es propio de Nook. Tampoco el icono
  del programa (`assets/nook.ico`), que es ese mismo personaje (ver `src/icono.rs`).

## Slint

- Proyecto: https://slint.dev
- Autores: Slint Developers
- Licencia: Slint Royalty-free Desktop, Mobile, and Web Applications License 2.0
  (https://github.com/slint-ui/slint/blob/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md)
- **Qué se usa:** la interfaz gráfica: el lenguaje de diseño de la barra (`ui/barra.slint`), su
  dibujo por software y sus animaciones.

## zeno

- Proyecto: https://github.com/dfrg/zeno
- Copyright (c) 2020 Chad Brokaw
- Licencia: MIT o Apache-2.0, a elección; se usa bajo MIT (texto completo más abajo)
- **Qué se usa:** el rasterizado de las formas del personaje (su cuerpo y sus ojos).

## windows-rs

- Proyecto: https://github.com/microsoft/windows-rs (crates `windows` y `windows-core`)
- Copyright (c) Microsoft Corporation
- Licencia: MIT o Apache-2.0, a elección; se usa bajo MIT (texto completo más abajo)
- **Qué se usa:** el acceso a las funciones de Windows: ventanas, iconos, atajos de teclado, el
  área de notificación, el desenfoque del fondo, el aviso de notificaciones nuevas, los controles
  multimedia y el nivel de audio (para que el personaje reaccione).

## serde y serde_json

- Proyecto: https://serde.rs
- Autores: Erick Tryzelaar y David Tolnay
- Licencia: MIT o Apache-2.0, a elección; se usa bajo MIT (texto completo más abajo)
- **Qué se usa:** la lectura y el guardado de la configuración.

## chrono

- Proyecto: https://github.com/chronotope/chrono
- Copyright (c) 2014, Kang Seonghoon, y colaboradores
- Licencia: MIT o Apache-2.0, a elección; se usa bajo MIT (texto completo más abajo)
- **Qué se usa:** la hora que muestra la barra.

---

## Licencia MIT

Se aplica a bloub (Copyright (c) 2026 Jérémy Perret), zeno, windows-rs, serde, serde_json y
chrono, cada uno con el aviso de copyright indicado arriba.

```
MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

# Royal Bet · Frontend de casa de apuestas (demo)

Casino y apuestas con **créditos ficticios**: sin registro, sin backend y sin dinero real.

## Cómo abrirlo
Abre `index.html` en el navegador (necesita internet para cargar Tailwind y las fuentes desde CDN).
Opcional: `python3 -m http.server 8080` y visita http://localhost:8080.

## Juegos
- **Ruleta europea**: tablero completo (plenos, docenas, columnas, rojo/negro…), rueda en canvas, deshacer/repetir/doblar.
- **Blackjack**: 6 barajas, pedir/plantarse/doblar, BJ 3:2, el crupier se planta en 17.
- **Texas Hold'em**: cara a cara contra la casa con evaluador de manos y apuestas de tamaño fijo.
- **Deportes**: fútbol, baloncesto y tenis; eventos próximos y en directo con cuotas variables, boleto simple o combinada.
- **Hípica**: carreras de 8 caballos, apuestas a ganador/colocado, animación de la carrera.

## Juegos "originales" (nuevos)
- **Mines**: cuadrícula 5×5, 1–24 minas, retirada en cualquier momento (multiplicador exacto con 3 % de ventaja).
- **Chicken Road**: la gallina cruza 15 carriles; 4 dificultades; cada carril multiplica.
- **Crash**: el cohete sube hasta que explota; retirada manual o automática.
- **Plinko**: 8/12/16 filas, 3 niveles de riesgo, varias bolas a la vez.
- **Dice**: umbral menor/mayor que, multiplicador según probabilidad.
- **Tragaperras**: 3×3, 5 líneas, RTP ≈ 95,8 %.

Extras: sonidos WebAudio (botón 🔊), historial de actividad, RTP real del jugador, atajo de barra espaciadora en Crash/Dice/Plinko/Slots/Chicken.

## Estructura
```
index.html        Maquetación base, cabecera, footer
css/styles.css    Estilos propios (fichas, cartas, fieltro, pista…)
js/core.js        Monedero (localStorage), toasts, baraja, selector de fichas
js/roulette.js · blackjack.js · poker.js · sports.js · horses.js
js/mines.js · chicken.js · crash.js · plinko.js · dice.js · slots.js
js/app.js         Router por hash y portada
```
El saldo y las estadísticas se guardan en `localStorage`; el botón **+ Recargar** añade 1.000 créditos.

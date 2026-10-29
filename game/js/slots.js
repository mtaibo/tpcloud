/* Tragaperras 3×3 con 5 líneas de pago (3 horizontales + 2 diagonales) */
(() => {
  const root = $('#view-slots');
  // símbolo, peso, pago por 3 iguales (×apuesta por línea), pago por 2 iguales a la izquierda
  const SYM = [
    { s: '🍒', w: 30, tri: 10, pair: 2 }, { s: '🍋', w: 24, tri: 15, pair: 1 }, { s: '🍇', w: 18, tri: 20, pair: 1 },
    { s: '🔔', w: 12, tri: 40, pair: 1 }, { s: '⭐', w: 8, tri: 80, pair: 1 }, { s: '💎', w: 5, tri: 200, pair: 1 }, { s: '7️⃣', w: 3, tri: 400, pair: 1 },
  ];
  const TOTAL_W = SYM.reduce((a, b) => a + b.w, 0);
  const LINES = [[[0, 0], [0, 1], [0, 2]], [[1, 0], [1, 1], [1, 2]], [[2, 0], [2, 1], [2, 2]], [[0, 0], [1, 1], [2, 2]], [[2, 0], [1, 1], [0, 2]]];
  let spinning = false, grid = [], last = 0, turbo = false;

  const roll = () => { let r = rnd() * TOTAL_W; for (const x of SYM) { if ((r -= x.w) < 0) return x; } return SYM[0]; };

  root.innerHTML = gameHeader('Tragaperras', '5 líneas de pago · 3 iguales o 2 iguales desde la izquierda', '95,8 %') + `
    <div class="grid lg:grid-cols-[330px_1fr] gap-5">
      <div class="card-panel p-5 space-y-4">
        ${betBoxHTML('sl-bet', 10)}
        <p class="text-xs text-slate-500 -mt-2">Apuesta total por giro (se reparte entre las 5 líneas).</p>
        <button id="sl-spin" class="btn-gold w-full !py-4 text-base">GIRAR</button>
        <label class="flex items-center gap-2 text-sm text-slate-300 cursor-pointer"><input type="checkbox" id="sl-turbo" class="accent-yellow-400"> Giro rápido</label>
        <div class="border-t border-line pt-4">
          <div class="label-sm mb-2">Tabla de pagos (× línea)</div>
          <table class="w-full text-sm">
            ${SYM.slice().reverse().map(x => `<tr class="border-b border-line/60"><td class="py-1.5 text-lg">${x.s}${x.s}${x.s}</td><td class="text-right font-bold text-gold">${x.tri}×</td><td class="text-right text-slate-500 text-xs">${x.s}${x.s} → ${x.pair}×</td></tr>`).join('')}
          </table>
        </div>
      </div>
      <div class="card-panel p-4 sm:p-8 grid place-items-center">
        <div class="slot-machine">
          <div class="slot-grid" id="sl-grid">${Array.from({ length: 9 }, (_, i) => `<div class="slot-cell" data-c="${i}"></div>`).join('')}</div>
        </div>
        <div class="mt-5 text-center min-h-[52px]">
          <div id="sl-msg" class="font-display text-2xl font-extrabold text-slate-300">¡Buena suerte!</div>
          <div id="sl-wins" class="text-xs text-slate-500 mt-1"></div>
        </div>
      </div>
    </div>`;

  const cells = $$('.slot-cell', root);
  const cellAt = (r, c) => cells[r * 3 + c];
  function setGrid(g) { g.forEach((row, r) => row.forEach((x, c) => { cellAt(r, c).textContent = x.s; })); }
  grid = Array.from({ length: 3 }, () => Array.from({ length: 3 }, roll)); setGrid(grid);

  function evaluate(g, bet) {
    const per = bet / LINES.length, wins = []; let total = 0;
    LINES.forEach((ln, i) => {
      const [a, b, c] = ln.map(([r, col]) => g[r][col]);
      let pay = 0;
      if (a === b && b === c) pay = a.tri;
      else if (a === b) pay = a.pair;
      if (pay) { total += per * pay; wins.push({ i, pay, cells: pay === a.tri && a === c ? ln : ln.slice(0, 2), sym: a.s }); }
    });
    return { total, wins };
  }

  async function spin() {
    if (spinning) return;
    const bet = readBet('sl-bet'); if (!bet || !Wallet.debit(bet)) return;
    spinning = true; setDisabled([$('#sl-spin', root), $('#sl-bet', root), ...$$('[data-bet-op]', root)], true);
    cells.forEach(c => c.classList.remove('win', 'lose-dim'));
    $('#sl-msg', root).textContent = 'Girando…'; $('#sl-msg', root).className = 'font-display text-2xl font-extrabold text-slate-300'; $('#sl-wins', root).textContent = '';
    turbo = $('#sl-turbo', root).checked;
    const final = Array.from({ length: 3 }, () => Array.from({ length: 3 }, roll));
    const stopAt = [0, 1, 2].map(c => (turbo ? 250 : 700) + c * (turbo ? 150 : 450)), t0 = performance.now();
    cells.forEach(c => c.classList.add('spinning'));
    await new Promise(res => {
      const id = setInterval(() => {
        const el = performance.now() - t0; let done = 0;
        for (let c = 0; c < 3; c++) {
          if (el >= stopAt[c]) {
            done++;
            for (let r = 0; r < 3; r++) { const cell = cellAt(r, c); if (cell.classList.contains('spinning')) { cell.classList.remove('spinning'); Sound.play('tick'); } cell.textContent = final[r][c].s; }
          } else for (let r = 0; r < 3; r++) cellAt(r, c).textContent = roll().s;
        }
        if (done === 3) { clearInterval(id); res(); }
      }, 70);
    });
    grid = final;
    const { total, wins } = evaluate(final, bet);
    Wallet.credit(total); Wallet.log('Tragaperras', bet, total);
    const msg = $('#sl-msg', root);
    if (total > 0) {
      const mult = total / bet;
      wins.forEach(w => w.cells.forEach(([r, c]) => cellAt(r, c).classList.add('win')));
      msg.textContent = total > bet ? `¡Ganas ${fmt(total)}!` : `Recuperas ${fmt(total)}`;
      msg.className = 'font-display text-2xl font-extrabold ' + (total > bet ? 'text-gold-light' : 'text-slate-200');
      $('#sl-wins', root).textContent = wins.map(w => `Línea ${w.i + 1}: ${w.sym} ${w.pay}×`).join(' · ');
      if (total > bet) { Sound.play(mult >= 10 ? 'big' : 'win'); Toast.show(`Tragaperras: +${fmt(total - bet)} cr (${fmt(mult)}×)`, 'win'); } else Sound.play('cash');
    } else { msg.textContent = 'Sin premio'; Sound.play('lose'); }
    spinning = false; setDisabled([$('#sl-spin', root), $('#sl-bet', root), ...$$('[data-bet-op]', root)], false);
  }

  $('#sl-spin', root).onclick = spin;
  document.addEventListener('keydown', e => {
    if (root.classList.contains('hidden') || e.target.tagName === 'INPUT' || e.code !== 'Space') return;
    e.preventDefault(); spin();
  });
})();

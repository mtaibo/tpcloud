/* Mines: cuadrícula 5×5, encuentra gemas y evita las minas */
(() => {
  const root = $('#view-mines');
  const N = 25, EDGE = 0.97;
  let active = false, mines = new Set(), shown = new Set(), bet = 0, count = 3;

  const mult = (k, m) => { let p = 1; for (let i = 0; i < k; i++) p *= (N - i) / (N - m - i); return Math.floor(EDGE * p * 100) / 100; };

  root.innerHTML = gameHeader('Mines', 'Descubre gemas, evita las minas y retírate cuando quieras', '97 %') + `
    <div class="grid lg:grid-cols-[330px_1fr] gap-5">
      <div class="card-panel p-5 space-y-4 order-2 lg:order-1">
        ${betBoxHTML('mn-bet', 10)}
        <div>
          <div class="flex justify-between"><label class="label-sm" for="mn-count">Minas</label><span class="font-bold text-white" id="mn-count-v">3</span></div>
          <input id="mn-count" type="range" min="1" max="24" value="3" class="w-full accent-yellow-400 mt-2">
          <div class="flex gap-1 mt-2">${[1, 3, 5, 10, 24].map(v => `<button class="btn !py-1 !px-2 text-xs flex-1" data-m="${v}">${v}</button>`).join('')}</div>
        </div>
        <button id="mn-start" class="btn-gold w-full !py-3 text-base">Apostar</button>
        <button id="mn-random" class="btn w-full" disabled>🎲 Casilla aleatoria</button>
        <button id="mn-cash" class="btn btn-green w-full !py-3" disabled>Retirar</button>
        <dl class="text-sm space-y-1.5 border-t border-line pt-4">
          <div class="flex justify-between"><dt class="text-slate-400">Multiplicador</dt><dd class="font-bold text-white" id="mn-mult">1,00×</dd></div>
          <div class="flex justify-between"><dt class="text-slate-400">Siguiente</dt><dd class="font-bold text-slate-300" id="mn-next">–</dd></div>
          <div class="flex justify-between"><dt class="text-slate-400">Beneficio actual</dt><dd class="font-bold text-gold" id="mn-profit">0,00</dd></div>
          <div class="flex justify-between"><dt class="text-slate-400">Gemas restantes</dt><dd class="font-bold text-white" id="mn-gems">–</dd></div>
        </dl>
      </div>
      <div class="card-panel p-4 sm:p-8 order-1 lg:order-2 grid place-items-center">
        <div class="relative w-full max-w-[520px]">
          <div id="mn-grid" class="grid grid-cols-5 gap-2 sm:gap-3"></div>
          <div id="mn-banner" class="hidden absolute inset-0 grid place-items-center pointer-events-none">
            <div class="result-banner text-center bg-ink/90 border-2 border-gold rounded-2xl px-8 py-5 shadow-glow">
              <div class="text-3xl font-extrabold text-gold" id="mn-bm"></div><div class="text-sm text-slate-300" id="mn-bp"></div>
            </div>
          </div>
        </div>
      </div>
    </div>`;

  const grid = $('#mn-grid', root);
  grid.innerHTML = Array.from({ length: N }, (_, i) => `<button class="mine-tile" data-i="${i}" aria-label="Casilla ${i + 1}"></button>`).join('');
  const tiles = $$('.mine-tile', root);

  const ctlEls = () => [$('#mn-bet', root), $('#mn-count', root), $('#mn-start', root), ...$$('[data-m],[data-bet-op]', root)];

  function refresh() {
    const k = shown.size, m = count;
    const cur = k ? mult(k, m) : 1, nxt = k < N - m ? mult(k + 1, m) : null;
    $('#mn-mult', root).textContent = fmt(cur) + '×';
    $('#mn-next', root).textContent = active && nxt ? fmt(nxt) + '×' : '–';
    $('#mn-profit', root).textContent = active ? fmt(bet * cur - bet) : '0,00';
    $('#mn-gems', root).textContent = active ? (N - m - k) : '–';
    $('#mn-cash', root).disabled = !active || k === 0;
    $('#mn-cash', root).textContent = active && k ? `Retirar ${fmt(bet * cur)}` : 'Retirar';
    $('#mn-random', root).disabled = !active;
  }

  function start() {
    const b = readBet('mn-bet'); if (!b || !Wallet.debit(b)) return;
    bet = b; count = +$('#mn-count', root).value; active = true; shown.clear();
    mines = new Set(); while (mines.size < count) mines.add(Math.floor(rnd() * N));
    tiles.forEach(t => { t.className = 'mine-tile'; t.innerHTML = ''; t.disabled = false; });
    $('#mn-banner', root).classList.add('hidden');
    ctlEls().forEach(e => e.disabled = true);
    Sound.play('click'); refresh();
  }

  function reveal(i) {
    if (!active || shown.has(i)) return;
    const t = tiles[i];
    if (mines.has(i)) { t.className = 'mine-tile boom'; t.innerHTML = '<span>💣</span>'; Sound.play('boom'); return end(false); }
    shown.add(i); t.className = 'mine-tile gem'; t.innerHTML = '<span>💎</span>'; Sound.play('gem');
    if (shown.size === N - count) return cash();
    refresh();
  }

  function end(won) {
    active = false;
    tiles.forEach((t, i) => {
      t.disabled = true;
      if (mines.has(i) && !t.classList.contains('boom')) { t.className = 'mine-tile dim'; t.innerHTML = '<span>💣</span>'; }
      else if (!mines.has(i) && !shown.has(i)) { t.className = 'mine-tile dim'; t.innerHTML = '<span>💎</span>'; }
    });
    ctlEls().forEach(e => e.disabled = false);
    refresh();
    const k = shown.size, m = mult(k, count), ret = won ? bet * m : 0;
    if (won) Wallet.credit(ret);
    Wallet.log('Mines', bet, ret);
    const ban = $('#mn-banner', root); ban.classList.remove('hidden');
    $('#mn-bm', root).textContent = won ? `${fmt(m)}×` : '¡BOOM!';
    $('#mn-bm', root).style.color = won ? '' : '#f87171';
    $('#mn-bp', root).textContent = won ? `+${fmt(ret - bet)} cr` : `-${fmt(bet)} cr`;
    if (won) { Sound.play(m >= 10 ? 'big' : 'cash'); Toast.show(`Retiras ${fmt(ret)} cr (${fmt(m)}×)`, 'win'); }
  }
  function cash() { if (active && shown.size) end(true); }

  grid.addEventListener('click', e => { const t = e.target.closest('.mine-tile'); if (t) reveal(+t.dataset.i); });
  $('#mn-start', root).onclick = start;
  $('#mn-cash', root).onclick = cash;
  $('#mn-random', root).onclick = () => {
    const free = [...Array(N).keys()].filter(i => !shown.has(i));
    if (free.length) reveal(free[Math.floor(rnd() * free.length)]);
  };
  $('#mn-count', root).oninput = e => {
    count = +e.target.value; $('#mn-count-v', root).textContent = count;
    $('#mn-next', root).textContent = '–'; $('#mn-mult', root).textContent = '1,00×';
  };
  root.addEventListener('click', e => {
    const m = e.target.closest('[data-m]'); if (!m || active) return;
    $('#mn-count', root).value = m.dataset.m; $('#mn-count', root).oninput({ target: $('#mn-count', root) });
  });
  refresh();
})();

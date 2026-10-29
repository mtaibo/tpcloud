/* Dice: elige un umbral y apuesta a que el dado (0–99,99) cae por encima o por debajo */
(() => {
  const root = $('#view-dice');
  const EDGE = 0.98;
  let mode = 'under', target = 50, rolling = false, history = [];

  const chance = () => (mode === 'under' ? target : 100 - target);
  const mult = () => Math.floor((EDGE * 100 / chance()) * 10000) / 10000;

  root.innerHTML = gameHeader('Dice', 'Mueve el umbral: menos probabilidad, más multiplicador', '98 %') + `
    <div class="grid lg:grid-cols-[330px_1fr] gap-5">
      <div class="card-panel p-5 space-y-4">
        ${betBoxHTML('dc-bet', 10)}
        <div class="grid grid-cols-2 gap-1 bg-ink rounded-lg p-1" id="dc-mode">
          <button class="tab !rounded-md !border-0 active" data-mode="under">Menor que</button>
          <button class="tab !rounded-md !border-0" data-mode="over">Mayor que</button>
        </div>
        <dl class="text-sm space-y-1.5">
          <div class="flex justify-between"><dt class="text-slate-400">Multiplicador</dt><dd class="font-bold text-white" id="dc-mult"></dd></div>
          <div class="flex justify-between"><dt class="text-slate-400">Probabilidad de ganar</dt><dd class="font-bold text-white" id="dc-chance"></dd></div>
          <div class="flex justify-between"><dt class="text-slate-400">Ganancia potencial</dt><dd class="font-bold text-gold" id="dc-win"></dd></div>
        </dl>
        <button id="dc-roll" class="btn-gold w-full !py-4 text-base">Tirar dado</button>
      </div>
      <div class="card-panel p-5 sm:p-8 min-w-0 flex flex-col justify-center">
        <div class="flex gap-1.5 justify-end mb-6 overflow-x-auto no-scrollbar" id="dc-hist"></div>
        <div class="text-center mb-8">
          <div id="dc-result" class="text-7xl sm:text-8xl font-extrabold tabular-nums text-white">–</div>
          <div id="dc-msg" class="text-sm text-slate-400 mt-1">Ajusta el umbral y tira</div>
        </div>
        <div class="relative pt-8 pb-2 px-2">
          <div id="dc-marker" class="absolute top-0 -translate-x-1/2 transition-[left] duration-500 hidden">
            <div class="px-2 py-1 rounded bg-white text-ink text-xs font-extrabold" id="dc-mval">0</div>
            <div class="w-0 h-0 mx-auto border-x-[6px] border-x-transparent border-t-[7px] border-t-white"></div>
          </div>
          <div class="relative h-4 rounded-full overflow-hidden bg-red-700/80" id="dc-bar"><div id="dc-win-zone" class="absolute inset-y-0 bg-emerald-500"></div></div>
          <input id="dc-slider" type="range" min="2" max="98" step="1" value="50" class="dice-slider w-full mt-4">
          <div class="flex justify-between text-xs text-slate-500 mt-1"><span>0</span><span>25</span><span>50</span><span>75</span><span>100</span></div>
        </div>
      </div>
    </div>`;

  function refresh() {
    const bet = +$('#dc-bet', root).value || 0;
    $('#dc-mult', root).textContent = mult().toFixed(4).replace('.', ',') + '×';
    $('#dc-chance', root).textContent = chance().toFixed(2).replace('.', ',') + ' %';
    $('#dc-win', root).textContent = fmt(bet * mult() - bet);
    const z = $('#dc-win-zone', root);
    if (mode === 'under') { z.style.left = '0'; z.style.width = target + '%'; } else { z.style.left = target + '%'; z.style.width = (100 - target) + '%'; }
  }

  function renderHist() {
    $('#dc-hist', root).innerHTML = history.slice(0, 12).map(h =>
      `<span class="shrink-0 text-xs font-bold px-2.5 py-1 rounded-full tabular-nums ${h.win ? 'bg-emerald-900/50 text-emerald-300' : 'bg-red-900/40 text-red-300'}">${h.r.toFixed(2).replace('.', ',')}</span>`).join('');
  }

  async function roll() {
    if (rolling) return;
    const b = readBet('dc-bet'); if (!b || !Wallet.debit(b)) return;
    rolling = true; setDisabled([$('#dc-roll', root), $('#dc-slider', root)], true);
    const r = Math.floor(rnd() * 10000) / 100;
    const win = mode === 'under' ? r < target : r > target;
    const out = $('#dc-result', root), mk = $('#dc-marker', root);
    out.className = 'text-7xl sm:text-8xl font-extrabold tabular-nums text-white';
    mk.classList.remove('hidden');
    for (let i = 0; i < 14; i++) { const f = rnd() * 100; out.textContent = f.toFixed(2).replace('.', ','); mk.style.left = f + '%'; $('#dc-mval', root).textContent = Math.floor(f); Sound.play('tick'); await sleep(40 + i * 7); }
    out.textContent = r.toFixed(2).replace('.', ','); mk.style.left = r + '%'; $('#dc-mval', root).textContent = r.toFixed(2).replace('.', ',');
    const ret = win ? b * mult() : 0;
    if (win) Wallet.credit(ret);
    Wallet.log('Dice', b, ret);
    out.classList.replace('text-white', win ? 'text-emerald-400' : 'text-red-400');
    $('#dc-msg', root).textContent = win ? `¡Ganas! +${fmt(ret - b)} cr` : `Pierdes ${fmt(b)} cr`;
    if (win) Toast.show(`Dice: ${r.toFixed(2)} · +${fmt(ret - b)} cr`, 'win'); else Sound.play('lose');
    history.unshift({ r, win }); renderHist();
    rolling = false; setDisabled([$('#dc-roll', root), $('#dc-slider', root)], false);
  }

  $('#dc-slider', root).addEventListener('input', e => { target = +e.target.value; refresh(); });
  $('#dc-bet', root).addEventListener('input', refresh);
  $('#dc-mode', root).addEventListener('click', e => {
    const b = e.target.closest('[data-mode]'); if (!b) return;
    mode = b.dataset.mode; $$('[data-mode]', root).forEach(x => x.classList.toggle('active', x === b)); refresh();
  });
  $('#dc-roll', root).onclick = roll;
  document.addEventListener('keydown', e => {
    if (root.classList.contains('hidden') || e.target.tagName === 'INPUT' || e.code !== 'Space') return;
    e.preventDefault(); roll();
  });
  refresh();
})();

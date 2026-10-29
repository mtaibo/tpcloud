/* Hípica: carreras con apuestas a ganador y colocado */
(() => {
  const root = $('#view-horses');
  const NAMES = ['Trueno Negro', 'Dulce Victoria', 'Rayo de Luna', 'El Cierzo', 'Furia Roja', 'Estrella Fugaz', 'Gran Duque', 'Brisa Marina', 'Capitán Nemo', 'Luna Llena', 'Viento Sur', 'Don Quijote', 'Tormenta', 'Reina Mora'];
  const COLORS = ['#dc2626', '#2563eb', '#16a34a', '#eab308', '#9333ea', '#ea580c', '#0891b2', '#db2777'];
  const TRACKS = ['Hipódromo de la Zarzuela', 'Ascot', 'Longchamp', 'Churchill Downs', 'Sandown Park'];

  let race = null, bets = [], type = 'win', pickIdx = null, chip = 10, running = false, raceNo = 0, history = [];

  function newRace() {
    raceNo++;
    const n = 8, names = [...NAMES].sort(() => Math.random() - 0.5).slice(0, n);
    const str = names.map(() => rand(0.55, 1));
    const w = str.map(s => Math.pow(s, 4)), tot = w.reduce((a, b) => a + b, 0);
    race = {
      track: pick(TRACKS), dist: pick([1200, 1600, 2000, 2400]),
      horses: names.map((name, i) => {
        const p = w[i] / tot, odds = Math.min(41, Math.max(1.8, Math.round((0.9 / p) * 2) / 2));
        return {
          name, color: COLORS[i], n: i + 1, str: str[i], odds, place: Math.max(1.15, Math.round((1 + (odds - 1) / 3.5) * 100) / 100),
          form: Array.from({ length: 5 }, () => randInt(1, 8)).join('-'), pos: 0, phase: rand(0, 6), finishT: null,
        };
      }),
    };
    bets = []; pickIdx = null; running = false;
    renderAll();
  }

  root.innerHTML = `
    <div class="flex items-end justify-between mb-5 flex-wrap gap-2">
      <div>
        <h1 class="font-display text-3xl font-extrabold text-white">Hípica</h1>
        <p class="text-sm text-slate-400" id="hr-sub"></p>
      </div>
      <div id="hr-hist" class="flex gap-2 flex-wrap text-xs text-slate-400"></div>
    </div>
    <div class="track mb-5" id="hr-track"></div>
    <div class="grid lg:grid-cols-[1fr_340px] gap-5 items-start">
      <div class="card-panel overflow-hidden">
        <table class="w-full text-sm">
          <thead class="bg-panel2 text-xs uppercase text-slate-500"><tr>
            <th class="text-left p-3">Caballo</th><th class="p-3 hidden sm:table-cell">Forma</th><th class="p-3">Ganador</th><th class="p-3">Colocado</th></tr></thead>
          <tbody id="hr-table"></tbody>
        </table>
      </div>
      <aside class="card-panel p-5 lg:sticky lg:top-32">
        <h3 class="font-bold text-white mb-3">Nueva apuesta</h3>
        <div class="grid grid-cols-2 gap-1 bg-ink rounded-lg p-1 mb-3">
          <button class="tab !rounded-md !border-0 active" data-type="win">Ganador</button>
          <button class="tab !rounded-md !border-0" data-type="place">Colocado (top 3)</button>
        </div>
        <div class="bg-ink border border-line rounded-lg p-3 text-sm min-h-[52px]" id="hr-sel"></div>
        <div class="label-sm mt-4 mb-2">Importe</div>
        <div class="flex gap-2 flex-wrap" id="hr-chips">${chipTrayHTML([5, 10, 25, 100], 10)}</div>
        <button id="hr-add" class="btn w-full mt-4" disabled>Añadir apuesta</button>
        <div class="mt-4 space-y-1.5 text-sm" id="hr-bets"></div>
        <button id="hr-go" class="btn-gold w-full mt-4 !py-3">¡Dar la salida!</button>
        <button id="hr-next" class="btn w-full mt-2 hidden">Siguiente carrera →</button>
      </aside>
    </div>`;

  const horseEmoji = '🏇';

  function renderTrack() {
    $('#hr-track', root).innerHTML = race.horses.map(h => `
      <div class="lane" data-lane="${h.n}">
        <div class="horse" id="hr-h${h.n}"><span class="silk" style="background:${h.color}">${h.n}</span><span class="emoji">${horseEmoji}</span></div>
      </div>`).join('') + '<div class="start-line"></div><div class="finish-line"></div>';
  }
  function renderTable() {
    $('#hr-table', root).innerHTML = race.horses.map((h, i) => `
      <tr class="border-t border-line hover:bg-panel2/60 ${pickIdx === i ? 'bg-panel2' : ''}">
        <td class="p-3"><span class="inline-grid place-items-center w-6 h-6 rounded text-white text-xs font-extrabold mr-2" style="background:${h.color}">${h.n}</span><span class="font-semibold text-white">${h.name}</span></td>
        <td class="p-3 text-center text-slate-500 hidden sm:table-cell tabular-nums">${h.form}</td>
        <td class="p-3 text-center"><button class="odd-btn mx-auto ${pickIdx === i && type === 'win' ? 'sel' : ''}" data-h="${i}:win" ${running ? 'disabled' : ''}><span>${h.odds.toFixed(2)}</span></button></td>
        <td class="p-3 text-center"><button class="odd-btn mx-auto ${pickIdx === i && type === 'place' ? 'sel' : ''}" data-h="${i}:place" ${running ? 'disabled' : ''}><span>${h.place.toFixed(2)}</span></button></td>
      </tr>`).join('');
  }
  function renderSel() {
    const el = $('#hr-sel', root);
    if (pickIdx === null) el.innerHTML = '<span class="text-slate-500">Elige un caballo en la tabla de cuotas.</span>';
    else {
      const h = race.horses[pickIdx], o = type === 'win' ? h.odds : h.place;
      el.innerHTML = `<div class="flex justify-between"><span class="font-semibold text-white">#${h.n} ${h.name}</span><span class="text-gold font-bold">${o.toFixed(2)}</span></div>
        <div class="text-xs text-slate-500">${type === 'win' ? 'Ganador' : 'Colocado (top 3)'} · ganancia potencial ${fmt(chip * o)}</div>`;
    }
    $('#hr-add', root).disabled = pickIdx === null || running;
  }
  function renderBets() {
    const total = bets.reduce((a, b) => a + b.stake, 0);
    $('#hr-bets', root).innerHTML = bets.length
      ? bets.map((b, i) => `<div class="flex justify-between bg-ink border border-line rounded-lg px-3 py-2">
          <span><b class="text-white">#${race.horses[b.h].n} ${race.horses[b.h].name}</b> <span class="text-xs text-slate-500">${b.type === 'win' ? 'Ganador' : 'Colocado'} @ ${b.odds.toFixed(2)}</span></span>
          <span class="font-bold text-gold">${fmt(b.stake)}</span></div>`).join('') + `<div class="text-right text-xs text-slate-500">Total apostado: ${fmt(total)}</div>`
      : '<div class="text-xs text-slate-500">Aún no hay apuestas en esta carrera.</div>';
    $('#hr-go', root).textContent = bets.length ? '¡Dar la salida!' : 'Ver carrera sin apostar';
  }
  function renderHist() {
    $('#hr-hist', root).innerHTML = history.slice(-5).reverse().map(h => `<span class="bg-panel border border-line rounded px-2 py-1">🥇 ${h}</span>`).join('');
  }
  function renderAll() {
    $('#hr-sub', root).textContent = `Carrera ${raceNo} · ${race.track} · ${race.dist} m`;
    renderTrack(); renderTable(); renderSel(); renderBets();
    $('#hr-go', root).classList.remove('hidden'); $('#hr-next', root).classList.add('hidden'); $('#hr-go', root).disabled = false;
  }

  /* ---------- Carrera ---------- */
  function run() {
    running = true; renderTable(); renderSel(); $('#hr-go', root).disabled = true;
    const trackW = () => $('#hr-track', root).clientWidth - 40 - 34 - 56;   // salida → meta
    const t0 = performance.now(); let last = t0; const order = [];
    const step = now => {
      const dt = (now - last) / 1000, t = (now - t0) / 1000; last = now;
      const W = trackW();
      race.horses.forEach(h => {
        if (h.finishT !== null) return;
        const wobble = Math.sin(t * 1.7 + h.phase) * 0.12 + Math.sin(t * 0.6 + h.phase * 2) * 0.08;
        h.pos += (6.4 + h.str * 2.2) * (1 + wobble + rand(-0.06, 0.06)) * dt;   // % por segundo
        if (h.pos >= 100) { h.pos = 100; h.finishT = t; order.push(h); }
        $('#hr-h' + h.n, root).style.transform = `translateX(${40 + (h.pos / 100) * W}px)`;
      });
      if (order.length < race.horses.length) requestAnimationFrame(step);
      else finish(order);
    };
    requestAnimationFrame(step);
  }

  function finish(order) {
    running = false;
    const winner = order[0];
    history.push(winner.name); renderHist();
    let staked = 0, ret = 0;
    const rows = order.map((h, i) => `<span class="${i < 3 ? 'text-white' : 'text-slate-500'}">${i + 1}º #${h.n} ${h.name}</span>`).join(' · ');
    bets.forEach(b => {
      staked += b.stake;
      const rank = order.findIndex(h => h === race.horses[b.h]);
      if ((b.type === 'win' && rank === 0) || (b.type === 'place' && rank < 3)) ret += b.stake * b.odds;
    });
    Wallet.credit(ret); if (staked) Wallet.log('Hípica', staked, ret);
    if (staked) Toast.show(ret > staked ? `¡Cobras ${fmt(ret)} cr! (+${fmt(ret - staked)})` : ret > 0 ? `Recuperas ${fmt(ret)} cr` : 'Sin suerte esta vez', ret > staked ? 'win' : ret > 0 ? '' : 'lose');
    else Toast.show(`Gana ${winner.name}`);
    $('#hr-bets', root).innerHTML = `<div class="text-xs leading-6 bg-ink border border-line rounded-lg p-3">🏁 <b class="text-gold">Resultado:</b><br>${rows}</div>` +
      (staked ? `<div class="text-right text-sm mt-2 font-bold ${ret > staked ? 'text-green-400' : 'text-red-400'}">${ret > 0 ? 'Cobrado: ' + fmt(ret) : 'Perdido: ' + fmt(staked)}</div>` : '');
    $('#hr-go', root).classList.add('hidden'); $('#hr-next', root).classList.remove('hidden');
    $$('.lane', root).forEach(l => l.style.outline = '');
    $(`[data-lane="${winner.n}"]`, root).style.outline = '2px solid #f5c451';
  }

  /* ---------- Eventos ---------- */
  root.addEventListener('click', e => {
    const t = e.target;
    const h = t.closest('[data-h]');
    if (h && !running) { const [i, ty] = h.dataset.h.split(':'); pickIdx = +i; type = ty; $$('[data-type]', root).forEach(b => b.classList.toggle('active', b.dataset.type === type)); renderTable(); renderSel(); return; }
    const ty = t.closest('[data-type]');
    if (ty) { type = ty.dataset.type; $$('[data-type]', root).forEach(b => b.classList.toggle('active', b === ty)); renderTable(); renderSel(); return; }
    if (t.closest('#hr-add') && pickIdx !== null && !running) {
      if (!Wallet.debit(chip)) return;
      const hs = race.horses[pickIdx];
      bets.push({ h: pickIdx, type, stake: chip, odds: type === 'win' ? hs.odds : hs.place });
      pickIdx = null; renderTable(); renderSel(); renderBets(); return;
    }
    if (t.closest('#hr-go') && !running) return run();
    if (t.closest('#hr-next') && !running) return newRace();
  });
  bindChipTray($('#hr-chips', root), v => { chip = v; renderSel(); });

  newRace();
})();

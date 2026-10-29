/* Apuestas deportivas simuladas: eventos próximos y en directo, boleto simple/combinada */
(() => {
  const root = $('#view-sports');
  const LIVE_MS = 45000;          // duración del "partido" en tiempo real
  const SPORTS = {
    football: { name: 'Fútbol', icon: '⚽', three: true },
    basket: { name: 'Baloncesto', icon: '🏀', three: false },
    tennis: { name: 'Tenis', icon: '🎾', three: false },
  };
  const DATA = {
    football: [
      ['LaLiga', ['Real Madrid', 'FC Barcelona', 'Atlético', 'Sevilla', 'Valencia', 'Real Sociedad', 'Villarreal', 'Athletic']],
      ['Premier League', ['Man City', 'Arsenal', 'Liverpool', 'Chelsea', 'Tottenham', 'Newcastle', 'Aston Villa', 'Man United']],
      ['Serie A', ['Inter', 'Milan', 'Juventus', 'Napoli', 'Roma', 'Lazio']],
    ],
    basket: [['NBA', ['Lakers', 'Celtics', 'Warriors', 'Bucks', 'Nuggets', 'Heat', 'Suns', 'Knicks']], ['ACB', ['Real Madrid', 'Barça', 'Baskonia', 'Unicaja', 'Valencia Basket']]],
    tennis: [['ATP Masters', ['Alcaraz', 'Sinner', 'Djokovic', 'Medvedev', 'Zverev', 'Rune', 'Ruud', 'Tsitsipas']]],
  };

  let events = [], slip = [], bets = [], filter = 'all', mode = 'single', nextId = 1, betId = 1;

  /* ---------- Generación de eventos ---------- */
  function makeEvent(startInSec) {
    const sport = pick(Object.keys(SPORTS)), [league, teams] = pick(DATA[sport]);
    let h = pick(teams), a; do { a = pick(teams); } while (a === h);
    const three = SPORTS[sport].three;
    const ph = rand(0.25, 0.7), pd = three ? rand(0.18, 0.3) : 0;
    const pa = Math.max(0.1, 1 - ph - pd);
    const sum = ph + pd + pa, probs = three ? [ph / sum, pd / sum, pa / sum] : [ph / (ph + pa), pa / (ph + pa)];
    const keys = three ? ['1', 'X', '2'] : ['1', '2'];
    const outcomes = keys.map((k, i) => ({ k, base: 1 / (probs[i] * 1.07), odds: 0 }));
    outcomes.forEach(o => o.odds = Math.max(1.05, Math.round(o.base * 100) / 100));
    // resultado oculto ponderado por probabilidad
    let r = Math.random(), idx = 0;
    for (; idx < probs.length - 1; idx++) { if (r < probs[idx]) break; r -= probs[idx]; }
    const result = keys[idx];
    return {
      id: nextId++, sport, league, home: h, away: a, outcomes, result, final: finalScore(sport, result),
      startAt: Date.now() + startInSec * 1000, status: 'upcoming', progress: 0, doneAt: 0,
    };
  }
  function finalScore(sport, res) {
    if (sport === 'football') {
      if (res === 'X') { const g = randInt(0, 3); return [g, g]; }
      const lo = randInt(0, 2), hi = lo + randInt(1, 3);
      return res === '1' ? [hi, lo] : [lo, hi];
    }
    if (sport === 'basket') {
      const w = randInt(94, 125), l = w - randInt(1, 16);
      return res === '1' ? [w, l] : [l, w];
    }
    const sets = Math.random() < 0.55 ? 0 : 1;
    return res === '1' ? [2, sets] : [sets, 2];
  }
  const label = (ev, k) => (k === '1' ? ev.home : k === '2' ? ev.away : 'Empate');

  function seed() {
    const starts = [-30, -18, -8, 40, 70, 110, 150, 200, 260, 320];
    events = starts.map(s => makeEvent(s));
  }

  /* ---------- Vista ---------- */
  root.innerHTML = `
    <div class="flex items-end justify-between mb-5 flex-wrap gap-2">
      <div>
        <h1 class="font-display text-3xl font-extrabold text-white">Apuestas Deportivas</h1>
        <p class="text-sm text-slate-400">Eventos simulados en tiempo real · Cuotas decimales · Apuesta en directo hasta el 60 % del partido</p>
      </div>
    </div>
    <div class="flex gap-2 overflow-x-auto no-scrollbar mb-4" id="sp-tabs"></div>
    <div class="grid lg:grid-cols-[1fr_340px] gap-5 items-start">
      <div>
        <div id="sp-list" class="space-y-3"></div>
        <h2 class="font-bold text-white text-lg mt-8 mb-3">Mis apuestas</h2>
        <div id="sp-bets" class="space-y-2"></div>
      </div>
      <aside class="card-panel p-5 lg:sticky lg:top-32">
        <div class="flex items-center justify-between mb-3">
          <h3 class="font-bold text-white">Boleto <span id="sp-count" class="ml-1 text-xs bg-gold text-ink rounded-full px-2 py-0.5">0</span></h3>
          <button id="sp-clear" class="text-xs text-slate-500 hover:text-white">Vaciar</button>
        </div>
        <div class="grid grid-cols-2 gap-1 bg-ink rounded-lg p-1 mb-3">
          <button class="tab !rounded-md !border-0 active" data-mode="single">Simples</button>
          <button class="tab !rounded-md !border-0" data-mode="combo">Combinada</button>
        </div>
        <div id="sp-slip" class="space-y-2 mb-4"></div>
        <label class="label-sm">Importe<span id="sp-stakehint" class="normal-case tracking-normal"></span></label>
        <input id="sp-stake" type="number" min="1" step="1" value="10" class="input mt-1">
        <div class="flex gap-1 mt-2">
          ${[5, 10, 25, 50].map(v => `<button class="btn !py-1 !px-2 text-xs flex-1" data-quick="${v}">${v}</button>`).join('')}
        </div>
        <dl class="mt-4 text-sm space-y-1">
          <div class="flex justify-between"><dt class="text-slate-400">Cuota total</dt><dd class="font-bold text-white" id="sp-odds">–</dd></div>
          <div class="flex justify-between"><dt class="text-slate-400">Importe total</dt><dd class="font-bold text-white" id="sp-total">0,00</dd></div>
          <div class="flex justify-between"><dt class="text-slate-400">Ganancia potencial</dt><dd class="font-bold text-gold" id="sp-win">0,00</dd></div>
        </dl>
        <button id="sp-place" class="btn-gold w-full mt-4 !py-3" disabled>Realizar apuesta</button>
      </aside>
    </div>`;

  const tabs = $('#sp-tabs', root);
  tabs.innerHTML = [['all', 'Todo', '🏆'], ...Object.entries(SPORTS).map(([k, v]) => [k, v.name, v.icon]), ['live', 'En directo', '🔴']]
    .map(([k, n, i]) => `<button class="tab ${k === filter ? 'active' : ''}" data-f="${k}">${i} ${n}</button>`).join('');

  const statusHTML = ev => ev.status === 'live'
    ? `<span class="text-red-400 font-bold flex items-center gap-1.5"><span class="live-dot"></span>${clock(ev)}</span>`
    : ev.status === 'finished' ? '<span class="text-slate-500 font-semibold">Finalizado</span>'
      : `<span class="text-slate-400">Empieza en ${countdown(ev)}</span>`;
  function clock(ev) {
    if (ev.sport === 'football') return Math.min(90, Math.floor(ev.progress * 90)) + "'";
    if (ev.sport === 'basket') return 'Q' + Math.min(4, 1 + Math.floor(ev.progress * 4));
    return 'Set ' + Math.min(3, 1 + Math.floor(ev.progress * 3));
  }
  function countdown(ev) {
    const s = Math.max(0, Math.ceil((ev.startAt - Date.now()) / 1000));
    return s >= 60 ? `${Math.floor(s / 60)}m ${String(s % 60).padStart(2, '0')}s` : s + 's';
  }
  const scoreOf = ev => ev.final.map(v => ev.status === 'upcoming' ? '–' : ev.status === 'finished' ? v : Math.floor(v * ev.progress));
  const locked = ev => ev.status === 'finished' || (ev.status === 'live' && ev.progress > 0.6);

  function renderList() {
    const list = events.filter(e => filter === 'all' || (filter === 'live' ? e.status === 'live' : e.sport === filter));
    $('#sp-list', root).innerHTML = list.length ? list.map(ev => {
      const [sh, sa] = scoreOf(ev);
      return `<div class="card-panel p-4" data-ev="${ev.id}">
        <div class="flex justify-between text-xs mb-3">
          <span class="text-slate-500 font-semibold">${SPORTS[ev.sport].icon} ${ev.league}</span>
          <span data-st>${statusHTML(ev)}</span>
        </div>
        <div class="flex items-center justify-between gap-4 flex-wrap">
          <div class="min-w-[150px] flex-1 space-y-1.5">
            <div class="flex justify-between font-semibold text-white"><span>${ev.home}</span><span class="tabular-nums text-gold" data-sh>${sh}</span></div>
            <div class="flex justify-between font-semibold text-white"><span>${ev.away}</span><span class="tabular-nums text-gold" data-sa>${sa}</span></div>
          </div>
          <div class="flex gap-2">
            ${ev.outcomes.map(o => `<button class="odd-btn" data-pick="${ev.id}:${o.k}" ${locked(ev) ? 'disabled' : ''}>
              <small>${o.k === '1' ? '1' : o.k === 'X' ? 'X' : '2'}</small><span>${locked(ev) ? '🔒' : o.odds.toFixed(2)}</span></button>`).join('')}
          </div>
        </div>
      </div>`;
    }).join('') : '<div class="card-panel p-8 text-center text-slate-500">No hay eventos en esta categoría ahora mismo.</div>';
    markSelected();
  }
  function markSelected() {
    $$('.odd-btn', root).forEach(b => b.classList.toggle('sel', slip.some(s => `${s.ev}:${s.k}` === b.dataset.pick)));
  }

  /* ---------- Boleto ---------- */
  const evById = id => events.find(e => e.id === id);
  function totals() {
    const stake = Math.max(0, +$('#sp-stake', root).value || 0);
    if (!slip.length) return { stake, total: 0, odds: 0, win: 0 };
    if (mode === 'combo') {
      const odds = slip.reduce((a, s) => a * s.odds, 1);
      return { stake, total: stake, odds, win: stake * odds };
    }
    return { stake, total: stake * slip.length, odds: 0, win: slip.reduce((a, s) => a + stake * s.odds, 0) };
  }
  function renderSlip() {
    $('#sp-count', root).textContent = slip.length;
    $('#sp-slip', root).innerHTML = slip.length ? slip.map(s => {
      const ev = evById(s.ev);
      return `<div class="bg-ink border border-line rounded-lg p-3 text-sm relative">
        <button class="absolute top-1.5 right-2 text-slate-500 hover:text-white" data-rm="${s.ev}:${s.k}" aria-label="Quitar">✕</button>
        <div class="text-xs text-slate-500">${ev.home} – ${ev.away}</div>
        <div class="flex justify-between font-semibold text-white mt-0.5"><span>${label(ev, s.k)}</span><span class="text-gold">${s.odds.toFixed(2)}</span></div>
      </div>`;
    }).join('') : '<p class="text-sm text-slate-500 text-center py-4">Haz clic en una cuota para añadirla al boleto.</p>';
    const t = totals();
    $('#sp-stakehint', root).textContent = mode === 'single' ? ' (por selección)' : '';
    $('#sp-odds', root).textContent = mode === 'combo' && slip.length ? t.odds.toFixed(2) : '–';
    $('#sp-total', root).textContent = fmt(t.total);
    $('#sp-win', root).textContent = fmt(t.win);
    $('#sp-place', root).disabled = !slip.length || t.stake <= 0 || (mode === 'combo' && slip.length < 2);
    $('#sp-place', root).textContent = mode === 'combo' && slip.length < 2 ? 'Añade 2+ selecciones' : 'Realizar apuesta';
    markSelected();
  }
  function toggle(evId, k) {
    const ev = evById(evId); if (!ev || locked(ev)) return;
    const i = slip.findIndex(s => s.ev === evId && s.k === k);
    if (i >= 0) slip.splice(i, 1);
    else {
      const j = slip.findIndex(s => s.ev === evId);   // una selección por evento
      if (j >= 0) slip.splice(j, 1);
      slip.push({ ev: evId, k, odds: ev.outcomes.find(o => o.k === k).odds });
    }
    renderSlip();
  }

  function place() {
    const t = totals();
    if (!Wallet.debit(t.total)) return;
    const mk = sels => ({
      id: betId++, status: 'open',
      sels: sels.map(s => { const ev = evById(s.ev); return { ...s, state: 'open', names: `${ev.home} – ${ev.away} · ${label(ev, s.k)} @${s.odds.toFixed(2)}` }; }),
    });
    if (mode === 'combo') bets.unshift({ ...mk(slip), stake: t.stake, odds: t.odds });
    else slip.forEach(s => bets.unshift({ ...mk([s]), stake: t.stake, odds: s.odds }));
    Toast.show(`Apuesta realizada · ${fmt(t.total)} cr`);
    slip = []; renderSlip(); renderBets();
  }

  function renderBets() {
    $('#sp-bets', root).innerHTML = bets.length ? bets.slice(0, 20).map(b => {
      const cls = b.status === 'won' ? 'text-green-400' : b.status === 'lost' ? 'text-red-400' : 'text-gold';
      const txt = b.status === 'won' ? `Ganada +${fmt(b.stake * b.odds - b.stake)}` : b.status === 'lost' ? 'Perdida' : 'Pendiente';
      return `<div class="card-panel p-3 text-sm">
        <div class="flex justify-between"><span class="font-semibold text-white">${b.sels.length > 1 ? `Combinada (${b.sels.length})` : 'Simple'} · ${fmt(b.stake)} cr @ ${b.odds.toFixed(2)}</span><span class="font-bold ${cls}">${txt}</span></div>
        ${b.sels.map(s => `<div class="text-xs text-slate-500 mt-1">${s.state === 'won' ? '✔' : s.state === 'lost' ? '✘' : '•'} ${s.names}</div>`).join('')}
      </div>`;
    }).join('') : '<div class="text-sm text-slate-500">Todavía no has apostado.</div>';
  }

  function settle(ev) {
    bets.forEach(b => {
      if (b.status !== 'open') return;
      b.sels.forEach(s => { if (s.ev === ev.id) s.state = s.k === ev.result ? 'won' : 'lost'; });
      if (b.sels.some(s => s.state === 'lost')) { b.status = 'lost'; Wallet.log('Deportes', b.stake, 0); Toast.show(`Apuesta perdida (${b.sels[0].names})`, 'lose'); }
      else if (b.sels.every(s => s.state === 'won')) {
        b.status = 'won'; Wallet.credit(b.stake * b.odds); Wallet.log('Deportes', b.stake, b.stake * b.odds);
        Toast.show(`¡Apuesta ganada! +${fmt(b.stake * b.odds - b.stake)} cr`, 'win');
      }
    });
    renderBets();
  }

  /* ---------- Reloj ---------- */
  function tick() {
    const now = Date.now();
    let structural = false;
    events.forEach(ev => {
      if (ev.status === 'upcoming') {
        if (now >= ev.startAt) { ev.status = 'live'; structural = true; }
        else ev.outcomes.forEach(o => {            // la cuota fluctúa
          const prev = o.odds; o.odds = Math.max(1.05, Math.round(o.base * (1 + rand(-0.05, 0.05)) * 100) / 100);
          o.dir = o.odds > prev ? 'up' : o.odds < prev ? 'down' : '';
        });
      }
      if (ev.status === 'live') {
        ev.progress = Math.min(1, (now - ev.startAt) / LIVE_MS);
        if (ev.progress >= 1) { ev.status = 'finished'; ev.doneAt = now; structural = true; settle(ev); }
        else if (ev.progress > 0.6 && !ev.lockedShown) { ev.lockedShown = true; structural = true; }
      }
    });
    // sustituir eventos terminados hace >20s
    const before = events.length;
    events = events.filter(e => !(e.status === 'finished' && now - e.doneAt > 20000 && !slip.some(s => s.ev === e.id)));
    while (events.length < 10) events.push(makeEvent(rand(60, 300)));
    if (events.length !== before || structural) {
      slip = slip.filter(s => { const e = evById(s.ev); return e && !locked(e); });
      renderList(); renderSlip(); return;
    }
    // actualización ligera (sin recrear el DOM)
    events.forEach(ev => {
      const card = $(`[data-ev="${ev.id}"]`, root); if (!card) return;
      $('[data-st]', card).innerHTML = statusHTML(ev);
      const [sh, sa] = scoreOf(ev);
      $('[data-sh]', card).textContent = sh; $('[data-sa]', card).textContent = sa;
      if (ev.status === 'upcoming') $$('.odd-btn', card).forEach((b, i) => {
        const o = ev.outcomes[i];
        b.lastElementChild.textContent = o.odds.toFixed(2);
        b.classList.toggle('up', o.dir === 'up'); b.classList.toggle('down', o.dir === 'down');
      });
    });
    slip.forEach(s => { const e = evById(s.ev); if (e) s.odds = e.outcomes.find(o => o.k === s.k).odds; });
    if (slip.length) renderSlip();
  }

  /* ---------- Eventos de interfaz ---------- */
  root.addEventListener('click', e => {
    const t = e.target;
    const pickBtn = t.closest('[data-pick]'); if (pickBtn) { const [id, k] = pickBtn.dataset.pick.split(':'); return toggle(+id, k); }
    const rm = t.closest('[data-rm]'); if (rm) { const [id, k] = rm.dataset.rm.split(':'); slip = slip.filter(s => !(s.ev === +id && s.k === k)); return renderSlip(); }
    const f = t.closest('[data-f]'); if (f) { filter = f.dataset.f; $$('[data-f]', root).forEach(b => b.classList.toggle('active', b === f)); return renderList(); }
    const m = t.closest('[data-mode]'); if (m) { mode = m.dataset.mode; $$('[data-mode]', root).forEach(b => b.classList.toggle('active', b === m)); return renderSlip(); }
    const q = t.closest('[data-quick]'); if (q) { $('#sp-stake', root).value = q.dataset.quick; return renderSlip(); }
    if (t.closest('#sp-clear')) { slip = []; return renderSlip(); }
    if (t.closest('#sp-place')) return place();
  });
  $('#sp-stake', root).addEventListener('input', renderSlip);

  seed(); renderList(); renderSlip(); renderBets();
  setInterval(tick, 1000);
  tick();
})();

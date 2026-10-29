/* Router por hash, navegación, portada y actividad */
(() => {
  const CATS = { casino: 'Casino', originals: 'Originales', sports: 'Deportes' };
  const GAMES = [
    { id: 'mines', title: 'Mines', desc: 'Descubre gemas en una cuadrícula 5×5 y evita las minas.', icon: '💣', cat: 'originals', rtp: '97 %', grad: 'from-emerald-700 to-emerald-950', hot: true },
    { id: 'chicken', title: 'Chicken Road', desc: 'Cruza la carretera con la gallina. Cada carril multiplica.', icon: '🐔', cat: 'originals', rtp: '97 %', grad: 'from-amber-600 to-amber-950', isNew: true },
    { id: 'crash', title: 'Crash', desc: 'El cohete sube y sube… retírate antes de que explote.', icon: '🚀', cat: 'originals', rtp: '97 %', grad: 'from-rose-700 to-rose-950', hot: true },
    { id: 'plinko', title: 'Plinko', desc: 'Suelta la bola entre las clavijas y cobra el multiplicador.', icon: '🔴', cat: 'originals', rtp: '97 %', grad: 'from-fuchsia-700 to-purple-950' },
    { id: 'dice', title: 'Dice', desc: 'Elige tu probabilidad y tira el dado. Tú pones el riesgo.', icon: '🎲', cat: 'originals', rtp: '98 %', grad: 'from-sky-700 to-sky-950' },
    { id: 'slots', title: 'Tragaperras', desc: '3×3 con 5 líneas de pago y premios de hasta 400×.', icon: '🎰', cat: 'casino', rtp: '95,8 %', grad: 'from-yellow-600 to-orange-950', isNew: true },
    { id: 'roulette', title: 'Ruleta Europea', desc: 'Pleno a 35:1 con tablero completo y rueda animada.', icon: '🎡', cat: 'casino', rtp: '97,3 %', grad: 'from-red-700 to-red-950' },
    { id: 'blackjack', title: 'Blackjack', desc: 'Pide, plántate o dobla. El blackjack paga 3:2.', icon: '🃏', cat: 'casino', rtp: '≈99 %', grad: 'from-green-700 to-green-950' },
    { id: 'poker', title: "Texas Hold'em", desc: 'Cara a cara contra la casa. El farol también cuenta.', icon: '♠️', cat: 'casino', rtp: '–', grad: 'from-indigo-700 to-indigo-950' },
    { id: 'sports', title: 'Deportes en vivo', desc: 'Fútbol, baloncesto y tenis con combinadas y cuotas vivas.', icon: '⚽', cat: 'sports', rtp: '≈93 %', grad: 'from-cyan-700 to-cyan-950', live: true },
    { id: 'horses', title: 'Hípica', desc: 'Apuesta a ganador o colocado y vive la carrera.', icon: '🏇', cat: 'sports', rtp: '≈90 %', grad: 'from-orange-700 to-stone-950' },
  ];
  const NAMES = Object.fromEntries(GAMES.map(g => [g.id, g]));
  const VIEWS = ['lobby', ...GAMES.map(g => g.id)];
  let catFilter = 'all', query = '';

  /* ---------- Navegación ---------- */
  const link = (g, cls) => `<a data-nav="${g.id}" href="#/${g.id}" class="${cls}"><span>${g.icon}</span> ${g.title}</a>`;
  $('#nav-desktop').innerHTML =
    `<a data-nav="lobby" href="#/lobby" class="nav-link">Inicio</a>` +
    Object.entries(CATS).map(([k, name]) => `
      <div class="dd">
        <button class="nav-link" data-navgroup="${k}" aria-haspopup="true">${name} <span class="text-[10px] opacity-60">▾</span></button>
        <div class="dd-menu">${GAMES.filter(g => g.cat === k).map(g => link(g, 'dd-item')).join('')}</div>
      </div>`).join('');
  $('#nav-mobile').innerHTML =
    `<a data-nav="lobby" href="#/lobby" class="nav-link">Inicio</a>` + GAMES.map(g => link(g, 'nav-link')).join('');

  /* ---------- Portada ---------- */
  $('#view-lobby').innerHTML = `
    <section class="relative overflow-hidden rounded-3xl border border-line bg-gradient-to-br from-panel2 via-panel to-ink p-8 sm:p-14 mb-6">
      <div class="absolute -right-10 -top-16 text-[20rem] leading-none text-gold/5 select-none">♛</div>
      <div class="absolute right-8 bottom-6 hidden md:flex gap-3 text-5xl opacity-80 select-none"><span class="animate-bounce">🎲</span><span class="animate-bounce [animation-delay:.15s]">🎰</span><span class="animate-bounce [animation-delay:.3s]">🐔</span></div>
      <span class="inline-block text-xs font-bold tracking-widest text-gold border border-gold/40 rounded-full px-3 py-1 mb-4">SIMULADOR · CRÉDITOS FICTICIOS</span>
      <h1 class="font-display text-4xl sm:text-6xl font-extrabold text-white leading-tight max-w-2xl">
        Aprende cómo funciona el juego <span class="text-transparent bg-clip-text bg-gradient-to-r from-gold-light to-gold-dark">sin arriesgar nada</span>
      </h1>
      <p class="mt-4 text-slate-400 max-w-xl">${GAMES.length} juegos con las mismas reglas y ventaja de la casa que uno real. Sin registro, sin pagos. Comprueba por ti mismo por qué, a la larga, la banca siempre gana.</p>
      <div class="mt-7 flex flex-wrap gap-3">
        <a href="#/mines" class="btn-gold !px-6 !py-3">Jugar a Mines</a>
        <a href="#/crash" class="btn !px-6 !py-3">Probar Crash</a>
      </div>
    </section>

    <section class="grid grid-cols-2 lg:grid-cols-5 gap-3 mb-8" id="lobby-stats"></section>

    <div class="flex flex-wrap items-center justify-between gap-3 mb-4">
      <div class="flex gap-2 overflow-x-auto no-scrollbar" id="lobby-cats">
        <button class="tab active" data-cat="all">Todos</button>
        ${Object.entries(CATS).map(([k, n]) => `<button class="tab" data-cat="${k}">${n}</button>`).join('')}
      </div>
      <input id="lobby-search" type="search" placeholder="Buscar juego…" class="input !w-full sm:!w-64" aria-label="Buscar juego">
    </div>
    <section class="grid sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-10" id="lobby-games"></section>

    <div class="grid lg:grid-cols-[1fr_340px] gap-5">
      <section class="card-panel overflow-hidden">
        <div class="flex items-center justify-between p-4 border-b border-line">
          <h2 class="font-bold text-white">Tu actividad reciente</h2>
          <button id="btn-reset" class="text-xs text-slate-500 hover:text-red-400">Reiniciar todo</button>
        </div>
        <div id="lobby-activity" class="max-h-96 overflow-y-auto"></div>
      </section>
      <aside class="card-panel p-5">
        <h2 class="font-bold text-white mb-2">🛡️ Juego responsable</h2>
        <p class="text-sm text-slate-400">Este proyecto es una simulación educativa. En un casino real, cada juego tiene una ventaja matemática para la casa (el «RTP» indica cuánto se devuelve de media): a largo plazo, el jugador pierde.</p>
        <ul class="text-sm text-slate-400 mt-3 space-y-1.5 list-disc list-inside">
          <li>Nunca apuestes dinero que no puedas perder.</li>
          <li>Las rachas no «compensan»: cada ronda es independiente.</li>
          <li>Si el juego deja de ser divertido, para.</li>
        </ul>
      </aside>
    </div>`;

  function renderGames() {
    const q = query.trim().toLowerCase();
    const list = GAMES.filter(g => (catFilter === 'all' || g.cat === catFilter) && (!q || (g.title + g.desc).toLowerCase().includes(q)));
    $('#lobby-games').innerHTML = list.length ? list.map(g => `
      <a href="#/${g.id}" class="game-tile group">
        <div class="art bg-gradient-to-br ${g.grad}">
          <span class="drop-shadow-lg group-hover:scale-110 transition-transform">${g.icon}</span>
          <div class="absolute top-2 left-2 flex gap-1 z-10">
            ${g.hot ? '<span class="badge bg-orange-500 text-white">🔥 Popular</span>' : ''}
            ${g.isNew ? '<span class="badge bg-gold text-ink">Nuevo</span>' : ''}
            ${g.live ? '<span class="badge bg-red-600 text-white">● En vivo</span>' : ''}
          </div>
        </div>
        <div class="p-4">
          <div class="flex items-center justify-between"><h3 class="font-bold text-white">${g.title}</h3><span class="text-[11px] font-bold text-emerald-300">RTP ${g.rtp}</span></div>
          <p class="text-xs text-slate-400 mt-1 min-h-[32px]">${g.desc}</p>
        </div>
      </a>`).join('') : '<p class="col-span-full text-center text-slate-500 py-10">No se han encontrado juegos.</p>';
  }

  function timeAgo(t) {
    const s = Math.round((Date.now() - t) / 1000);
    return s < 5 ? 'ahora' : s < 60 ? `hace ${s}s` : s < 3600 ? `hace ${Math.floor(s / 60)} min` : `hace ${Math.floor(s / 3600)} h`;
  }
  function renderActivity(s) {
    const h = s.history || [];
    $('#lobby-activity').innerHTML = h.length ? `
      <table class="w-full text-sm">
        <thead class="text-xs uppercase text-slate-500 sticky top-0 bg-panel"><tr><th class="text-left p-3">Juego</th><th class="text-right p-3">Apuesta</th><th class="text-right p-3">Multip.</th><th class="text-right p-3">Resultado</th></tr></thead>
        <tbody>${h.map(r => {
          const net = r.ret - r.stake;
          return `<tr class="border-t border-line"><td class="p-3"><span class="text-white font-semibold">${r.game}</span><div class="text-[11px] text-slate-500">${timeAgo(r.t)}</div></td>
            <td class="p-3 text-right tabular-nums">${fmt(r.stake)}</td>
            <td class="p-3 text-right tabular-nums text-slate-400">${r.stake ? fmt(r.ret / r.stake) + '×' : '–'}</td>
            <td class="p-3 text-right tabular-nums font-bold ${net > 0 ? 'text-green-400' : net < 0 ? 'text-red-400' : 'text-slate-400'}">${net > 0 ? '+' : ''}${fmt(net)}</td></tr>`;
        }).join('')}</tbody></table>`
      : '<p class="p-8 text-center text-slate-500 text-sm">Aún no has jugado ninguna ronda.</p>';
  }

  function renderStats(s) {
    const net = s.won - s.wagered, rtp = s.wagered ? (s.won / s.wagered * 100) : null;
    $('#lobby-stats').innerHTML = [
      ['Saldo', fmt(s.balance), 'text-gold'],
      ['Total apostado', fmt(s.wagered), 'text-white'],
      ['Balance neto', (net >= 0 ? '+' : '') + fmt(net), net >= 0 ? 'text-green-400' : 'text-red-400'],
      ['Tu RTP real', rtp === null ? '–' : rtp.toFixed(1).replace('.', ',') + ' %', 'text-white'],
      ['Mejor ronda', '+' + fmt(s.best || 0), 'text-white'],
    ].map(([l, v, c]) => `<div class="card-panel p-4"><div class="label-sm">${l}</div><div class="text-xl font-extrabold tabular-nums mt-1 ${c}">${v}</div></div>`).join('');
    renderActivity(s);
  }

  $('#lobby-cats').addEventListener('click', e => {
    const b = e.target.closest('[data-cat]'); if (!b) return;
    catFilter = b.dataset.cat; $$('[data-cat]').forEach(x => x.classList.toggle('active', x === b)); renderGames();
  });
  $('#lobby-search').addEventListener('input', e => { query = e.target.value; renderGames(); });
  $('#btn-reset').onclick = () => {
    if (confirm('¿Reiniciar saldo, estadísticas y actividad?')) { Wallet.resetAll(); Toast.show('Cuenta de prueba reiniciada'); }
  };

  /* ---------- Cabecera ---------- */
  let shown = null;
  const balEl = $('#balance');
  Wallet.onChange(s => {
    balEl.textContent = fmt(s.balance);
    if (shown !== null && s.balance !== shown) {
      balEl.classList.remove('bal-up', 'bal-down'); void balEl.offsetWidth;
      balEl.classList.add(s.balance > shown ? 'bal-up' : 'bal-down');
    }
    shown = s.balance;
    renderStats(s);
  });
  $('#btn-reload').onclick = () => { Wallet.reload(1000); Toast.show('+1.000 créditos de prueba añadidos', 'win'); };
  const soundBtn = $('#btn-sound');
  const paintSound = () => { soundBtn.textContent = Sound.on ? '🔊' : '🔇'; };
  soundBtn.onclick = () => { Sound.toggle(); paintSound(); Sound.play('click'); };
  paintSound();
  $('#year').textContent = new Date().getFullYear();

  /* ---------- Router ---------- */
  function route() {
    const h = location.hash.replace(/^#\/?/, '') || 'lobby';
    const v = VIEWS.includes(h) ? h : 'lobby';
    VIEWS.forEach(x => $('#view-' + x).classList.toggle('hidden', x !== v));
    $$('[data-nav]').forEach(a => a.classList.toggle('active', a.dataset.nav === v));
    $$('[data-navgroup]').forEach(b => b.classList.toggle('active', NAMES[v] && NAMES[v].cat === b.dataset.navgroup));
    document.title = (NAMES[v] ? NAMES[v].title + ' · ' : '') + 'Royal Bet';
    window.scrollTo(0, 0);
    window.dispatchEvent(new Event('resize'));   // los canvas recalculan su tamaño al mostrarse
  }
  window.addEventListener('hashchange', route);
  renderGames(); route();
  setInterval(() => renderActivity(Wallet.state), 30000);
})();

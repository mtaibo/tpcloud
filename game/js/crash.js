/* Crash: el multiplicador sube hasta que explota. Retírate antes. */
(() => {
  const root = $('#view-crash');
  const K = 0.12;                         // m(t) = e^(K·t)
  let state = 'idle', bet = 0, crashAt = 1, t0 = 0, raf = 0, curM = 1, history = [];

  root.innerHTML = gameHeader('Crash', 'Retírate antes de que el cohete explote · 3 % de ventaja de la casa', '97 %') + `
    <div class="grid lg:grid-cols-[330px_1fr] gap-5">
      <div class="card-panel p-5 space-y-4">
        ${betBoxHTML('cr-bet', 10)}
        <div>
          <label class="label-sm" for="cr-auto">Retirada automática (×)</label>
          <input id="cr-auto" type="number" min="1.01" step="0.01" value="2.00" class="input mt-1.5">
        </div>
        <button id="cr-go" class="btn-gold w-full !py-4 text-base">Apostar</button>
        <dl class="text-sm space-y-1.5 border-t border-line pt-4">
          <div class="flex justify-between"><dt class="text-slate-400">Beneficio potencial</dt><dd class="font-bold text-gold" id="cr-profit">0,00</dd></div>
          <div class="flex justify-between"><dt class="text-slate-400">Probabilidad de llegar a la auto-retirada</dt><dd class="font-bold text-white" id="cr-prob">–</dd></div>
        </dl>
      </div>
      <div class="card-panel p-3 sm:p-4 min-w-0">
        <div class="flex gap-1.5 overflow-x-auto no-scrollbar pb-2" id="cr-hist"></div>
        <div class="relative rounded-xl overflow-hidden bg-ink border border-line">
          <canvas id="cr-canvas" class="w-full block" style="height:380px"></canvas>
          <div class="absolute inset-0 grid place-items-center pointer-events-none">
            <div class="text-center"><div id="cr-big" class="text-6xl sm:text-7xl font-extrabold text-white tabular-nums">1,00×</div><div id="cr-sub" class="text-sm text-slate-400 mt-1">Haz tu apuesta para despegar</div></div>
          </div>
        </div>
      </div>
    </div>`;

  const cv = $('#cr-canvas', root), ctx = cv.getContext('2d');
  const fit = () => { const r = devicePixelRatio || 1; cv.width = cv.clientWidth * r; cv.height = cv.clientHeight * r; ctx.setTransform(r, 0, 0, r, 0, 0); };

  function draw(m, t, crashed) {
    const W = cv.clientWidth, H = cv.clientHeight, P = 36;
    ctx.clearRect(0, 0, W, H);
    const tMax = Math.max(6, t * 1.15), mMax = Math.max(2, m * 1.25);
    const X = tt => P + (tt / tMax) * (W - P - 14), Y = mm => H - P - ((mm - 1) / (mMax - 1)) * (H - P - 20);
    // rejilla
    ctx.strokeStyle = 'rgba(255,255,255,.06)'; ctx.fillStyle = '#64748b'; ctx.font = '11px Inter'; ctx.lineWidth = 1;
    const step = mMax > 20 ? 10 : mMax > 8 ? 2 : mMax > 4 ? 1 : 0.5;
    for (let v = 1; v <= mMax; v += step) { ctx.beginPath(); ctx.moveTo(P, Y(v)); ctx.lineTo(W - 10, Y(v)); ctx.stroke(); ctx.fillText(v.toFixed(v % 1 ? 1 : 0) + '×', 4, Y(v) + 4); }
    for (let s = 0; s <= tMax; s += tMax > 20 ? 5 : 2) ctx.fillText(s + 's', X(s) - 6, H - 12);
    if (t <= 0) return;
    // curva
    const pts = []; for (let i = 0; i <= 80; i++) { const tt = (t * i) / 80; pts.push([X(tt), Y(Math.exp(K * tt))]); }
    const col = crashed ? '#ef4444' : '#f5c451';
    const g = ctx.createLinearGradient(0, 0, 0, H); g.addColorStop(0, crashed ? 'rgba(239,68,68,.35)' : 'rgba(245,196,81,.35)'); g.addColorStop(1, 'rgba(0,0,0,0)');
    ctx.beginPath(); ctx.moveTo(X(0), Y(1)); pts.forEach(p => ctx.lineTo(...p)); ctx.lineTo(X(t), Y(1)); ctx.closePath(); ctx.fillStyle = g; ctx.fill();
    ctx.beginPath(); pts.forEach((p, i) => i ? ctx.lineTo(...p) : ctx.moveTo(...p));
    ctx.strokeStyle = col; ctx.lineWidth = 4; ctx.lineCap = 'round'; ctx.stroke();
    const [ex, ey] = pts[pts.length - 1];
    ctx.font = '26px sans-serif'; ctx.fillText(crashed ? '💥' : '🚀', ex - 12, ey + 8);
  }

  function renderHist() {
    $('#cr-hist', root).innerHTML = history.slice(0, 14).map(v =>
      `<span class="shrink-0 text-xs font-bold px-2.5 py-1 rounded-full ${v >= 2 ? 'bg-emerald-900/50 text-emerald-300' : 'bg-red-900/40 text-red-300'}">${fmt(v)}×</span>`).join('');
  }

  const setBig = (txt, cls = 'text-white') => { const b = $('#cr-big', root); b.textContent = txt; b.className = `text-6xl sm:text-7xl font-extrabold tabular-nums ${cls}`; };

  function updateProb() {
    const a = +$('#cr-auto', root).value;
    $('#cr-prob', root).textContent = a > 1 ? (0.97 / a * 100).toFixed(2).replace('.', ',') + ' %' : '–';
  }

  function launch() {
    if (state === 'running') return cashOut();
    if (state !== 'idle') return;
    const b = readBet('cr-bet'); if (!b || !Wallet.debit(b)) return;
    bet = b; state = 'running';
    const u = rnd();
    crashAt = Math.max(1, Math.floor((0.97 / (1 - u)) * 100) / 100);
    t0 = performance.now();
    setDisabled([$('#cr-bet', root), $('#cr-auto', root), ...$$('[data-bet-op]', root)], true);
    $('#cr-sub', root).textContent = 'En vuelo…';
    $('#cr-go', root).textContent = 'Retirar'; $('#cr-go', root).className = 'btn btn-green w-full !py-4 text-base';
    Sound.play('click');
    raf = requestAnimationFrame(loop);
  }

  function loop(now) {
    const t = (now - t0) / 1000, m = Math.exp(K * t);
    const auto = +$('#cr-auto', root).value;
    if (m >= crashAt) { curM = crashAt; return boom(t); }
    curM = m;
    if (auto > 1 && m >= auto) { curM = auto; return cashOut(); }
    setBig(fmt(Math.floor(m * 100) / 100) + '×', m >= 2 ? 'text-gold' : 'text-white');
    $('#cr-profit', root).textContent = fmt(bet * Math.floor(m * 100) / 100 - bet);
    $('#cr-go', root).textContent = `Retirar ${fmt(bet * Math.floor(m * 100) / 100)}`;
    draw(m, t, false);
    raf = requestAnimationFrame(loop);
  }

  function reset() {
    state = 'idle';
    setDisabled([$('#cr-bet', root), $('#cr-auto', root), ...$$('[data-bet-op]', root)], false);
    $('#cr-go', root).textContent = 'Apostar'; $('#cr-go', root).className = 'btn-gold w-full !py-4 text-base';
    $('#cr-profit', root).textContent = '0,00';
  }

  function cashOut() {
    if (state !== 'running') return;
    cancelAnimationFrame(raf);
    const m = Math.floor(curM * 100) / 100, ret = bet * m, t = Math.log(crashAt) / K;
    Wallet.credit(ret); Wallet.log('Crash', bet, ret);
    setBig(fmt(m) + '×', 'text-emerald-400');
    $('#cr-sub', root).textContent = `¡Retirado! +${fmt(ret - bet)} cr · el cohete habría llegado a ${fmt(crashAt)}×`;
    draw(Math.exp(K * (Math.log(m) / K)), Math.log(m) / K, false);
    history.unshift(crashAt); renderHist();
    Toast.show(`Retiras ${fmt(ret)} cr (${fmt(m)}×)`, 'win'); Sound.play(m >= 5 ? 'big' : 'cash');
    reset();
  }

  function boom(t) {
    cancelAnimationFrame(raf);
    Wallet.log('Crash', bet, 0);
    setBig(fmt(crashAt) + '×', 'text-red-500');
    $('#cr-sub', root).textContent = `¡Crash! Pierdes ${fmt(bet)} cr`;
    draw(crashAt, Math.log(crashAt) / K, true);
    history.unshift(crashAt); renderHist();
    Toast.show(`Crash en ${fmt(crashAt)}×`, 'lose');
    reset();
  }

  $('#cr-go', root).onclick = launch;
  $('#cr-auto', root).addEventListener('input', updateProb);
  window.addEventListener('resize', () => { fit(); if (state === 'idle') draw(1, 0, false); });
  document.addEventListener('keydown', e => {
    if (root.classList.contains('hidden') || e.target.tagName === 'INPUT' || e.code !== 'Space') return;
    e.preventDefault(); launch();
  });
  new ResizeObserver(() => { fit(); if (state === 'idle') draw(1, 0, false); }).observe(cv);
  fit(); draw(1, 0, false); updateProb(); renderHist();
})();

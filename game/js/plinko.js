/* Plinko: la bola cae entre clavijas hasta un multiplicador */
(() => {
  const root = $('#view-plinko');
  const RTP = 0.97;
  const RISK = { low: [0.5, 4, 3], medium: [0.2, 25, 4], high: [0.1, 300, 6] };   // base, amplitud, exponente
  let rows = 12, risk = 'medium', mults = [], balls = [], flashes = {}, history = [], raf = 0, pending = 0;

  const comb = (n, k) => { let r = 1; for (let i = 1; i <= k; i++) r = r * (n - k + i) / i; return r; };
  function buildMults() {
    const [base, amp, e] = RISK[risk];
    const raw = Array.from({ length: rows + 1 }, (_, i) => base + amp * Math.pow(Math.abs(i - rows / 2) / (rows / 2), e));
    const ev = raw.reduce((a, v, i) => a + v * comb(rows, i) / Math.pow(2, rows), 0);
    mults = raw.map(v => Math.max(0.1, Math.round(v * RTP / ev * 10) / 10));
  }

  root.innerHTML = gameHeader('Plinko', 'Suelta la bola y mira dónde cae · puedes lanzar varias a la vez', '97 %') + `
    <div class="grid lg:grid-cols-[330px_1fr] gap-5">
      <div class="card-panel p-5 space-y-4">
        ${betBoxHTML('pl-bet', 5)}
        <div>
          <div class="label-sm mb-2">Riesgo</div>
          <div class="grid grid-cols-3 gap-1 bg-ink rounded-lg p-1" id="pl-risk">
            <button class="tab !rounded-md !border-0" data-risk="low">Bajo</button>
            <button class="tab !rounded-md !border-0 active" data-risk="medium">Medio</button>
            <button class="tab !rounded-md !border-0" data-risk="high">Alto</button>
          </div>
        </div>
        <div>
          <div class="label-sm mb-2">Filas</div>
          <div class="grid grid-cols-3 gap-1 bg-ink rounded-lg p-1" id="pl-rows">
            ${[8, 12, 16].map(r => `<button class="tab !rounded-md !border-0 ${r === rows ? 'active' : ''}" data-rows="${r}">${r}</button>`).join('')}
          </div>
        </div>
        <button id="pl-drop" class="btn-gold w-full !py-4 text-base">Soltar bola</button>
        <p class="text-xs text-slate-500">Atajo: barra espaciadora. El importe se descuenta al soltar cada bola.</p>
        <div><div class="label-sm mb-2">Últimos resultados</div><div class="flex gap-1.5 flex-wrap" id="pl-hist"></div></div>
      </div>
      <div class="card-panel p-3 grid place-items-center min-w-0">
        <canvas id="pl-canvas" class="max-w-full"></canvas>
      </div>
    </div>`;

  const cv = $('#pl-canvas', root), ctx = cv.getContext('2d');
  const SP = 34, TOP = 30, RH = 30, W = 620;
  let H = 0;
  const px = (r, k) => W / 2 + k * SP;   // k: desplazamiento en unidades de SP

  function layout() {
    buildMults();
    H = TOP + rows * RH + 70;
    const r = devicePixelRatio || 1;
    cv.width = W * r; cv.height = H * r; cv.style.width = W + 'px'; cv.style.height = H + 'px';
    ctx.setTransform(r, 0, 0, r, 0, 0);
  }
  const binColor = i => {
    const d = Math.abs(i - rows / 2) / (rows / 2);
    return `hsl(${Math.round(50 - d * 50)} 90% ${48 - d * 6}%)`;
  };

  function frame(now) {
    ctx.clearRect(0, 0, W, H);
    // clavijas
    ctx.fillStyle = '#cbd5e1';
    for (let r = 0; r < rows; r++) for (let j = 0; j <= r + 2; j++) {
      ctx.beginPath(); ctx.arc(px(r, j - (r + 2) / 2), TOP + r * RH, 3, 0, 7); ctx.fill();
    }
    // casillas
    const bw = SP - 4, by = TOP + rows * RH + 14;
    mults.forEach((m, i) => {
      const f = flashes[i] ? Math.max(0, 1 - (now - flashes[i]) / 400) : 0;
      const x = px(0, i - rows / 2) - bw / 2, y = by + f * 6;
      ctx.fillStyle = binColor(i); ctx.globalAlpha = 0.85 + f * 0.15;
      ctx.beginPath(); ctx.roundRect(x, y, bw, 28, 5); ctx.fill(); ctx.globalAlpha = 1;
      ctx.fillStyle = '#1a1305'; ctx.font = `800 ${rows === 16 ? 9 : 10.5}px Inter`; ctx.textAlign = 'center';
      ctx.fillText(m >= 100 ? Math.round(m) : m, x + bw / 2, y + 18);
    });
    // bolas
    balls = balls.filter(b => {
      const el = now - b.t0, s = Math.min(rows + 1, Math.floor(el / b.dt)), u = Math.min(1, (el - s * b.dt) / b.dt);
      if (s >= rows + 1) { land(b, now); return false; }
      const k0 = b.ks[s], k1 = b.ks[s + 1];
      const x = px(0, k0 + (k1 - k0) * u), y0 = TOP + (s - 1) * RH, y1 = TOP + s * RH;
      const y = (s === 0 ? TOP - 22 : y0) + ((s === 0 ? TOP : s === rows ? y1 + 6 : y1) - (s === 0 ? TOP - 22 : y0)) * u - Math.sin(u * Math.PI) * 9;
      ctx.fillStyle = '#ef4444'; ctx.shadowColor = 'rgba(239,68,68,.8)'; ctx.shadowBlur = 10;
      ctx.beginPath(); ctx.arc(x, y, 7, 0, 7); ctx.fill(); ctx.shadowBlur = 0;
      return true;
    });
    ctx.textAlign = 'start';
    raf = balls.length || Object.values(flashes).some(t => now - t < 400) ? requestAnimationFrame(frame) : 0;
  }
  const ensureLoop = () => { if (!raf) raf = requestAnimationFrame(frame); };

  function land(b, now) {
    flashes[b.bin] = now;
    const m = mults[b.bin], ret = b.bet * m;
    Wallet.credit(ret); Wallet.log('Plinko', b.bet, ret);
    history.unshift({ m, win: m >= 1 }); history = history.slice(0, 12);
    $('#pl-hist', root).innerHTML = history.map(h => `<span class="text-xs font-bold px-2 py-1 rounded-full ${h.win ? 'bg-emerald-900/50 text-emerald-300' : 'bg-red-900/40 text-red-300'}">${h.m}×</span>`).join('');
    if (m >= 5) { Toast.show(`Plinko ${m}× → +${fmt(ret - b.bet)} cr`, 'win'); Sound.play('big'); }
    else if (m >= 1) Sound.play('gem'); else Sound.play('tick');
    pending--; updateLock();
  }

  function drop() {
    const bet = readBet('pl-bet'); if (!bet || !Wallet.debit(bet)) return;
    let k = 0; const ks = [0];
    for (let r = 0; r < rows; r++) { k += rnd() < 0.5 ? -0.5 : 0.5; ks.push(k); }
    // ks[0]=0 inicio; ks[s] posición tras pasar la fila s-1; casilla final = nº de pasos a la derecha
    ks.push(k);   // último tramo: caer a la casilla
    const bin = Math.round(k + rows / 2);
    balls.push({ t0: performance.now(), dt: 150, ks, bin, bet });
    pending++; updateLock(); Sound.play('click'); ensureLoop();
  }
  function updateLock() { setDisabled([...$$('[data-risk],[data-rows]', root)], pending > 0); }

  $('#pl-drop', root).onclick = drop;
  $('#pl-risk', root).addEventListener('click', e => {
    const b = e.target.closest('[data-risk]'); if (!b || pending) return;
    risk = b.dataset.risk; $$('[data-risk]', root).forEach(x => x.classList.toggle('active', x === b)); layout(); frame(performance.now());
  });
  $('#pl-rows', root).addEventListener('click', e => {
    const b = e.target.closest('[data-rows]'); if (!b || pending) return;
    rows = +b.dataset.rows; $$('[data-rows]', root).forEach(x => x.classList.toggle('active', x === b)); layout(); frame(performance.now());
  });
  document.addEventListener('keydown', e => {
    if (root.classList.contains('hidden') || e.target.tagName === 'INPUT' || e.code !== 'Space') return;
    e.preventDefault(); drop();
  });
  layout(); frame(performance.now());
})();
